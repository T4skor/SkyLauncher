"""Launcher independiente de Minecraft Java con autenticación Microsoft PKCE."""
import json
import os
from pathlib import Path
import queue
import subprocess
import threading
import tkinter as tk
from tkinter import messagebox, simpledialog, ttk
import webbrowser

import minecraft_launcher_lib as mc


DATA = Path.home() / ".mi-minecraft-launcher"
GAME = DATA / "minecraft"
CONFIG = DATA / "config.json"
REDIRECT = "https://login.microsoftonline.com/common/oauth2/nativeclient"


def read_config():
    try:
        value = json.loads(CONFIG.read_text(encoding="utf-8"))
        return value if isinstance(value, dict) else {}
    except (OSError, ValueError):
        return {}


class Launcher(tk.Tk):
    def __init__(self):
        super().__init__()
        self.title("SkyLauncher · by T4skor")
        self.geometry("670x560")
        self.minsize(610, 520)
        self.events = queue.Queue()
        self.profile = None
        self.busy = False
        self.versions = []
        self.settings = read_config()
        self.client = tk.StringVar(value=self.settings.get("client_id", ""))
        self.version = tk.StringVar(value=self.settings.get("version", ""))
        self.ram = tk.StringVar(value=str(self.settings.get("ram", 4)))
        self.java = tk.StringVar(value=self.settings.get("java", ""))
        self.snapshots = tk.BooleanVar(value=False)
        self.status = tk.StringVar(value="Preparando launcher…")
        self.account = tk.StringVar(value="Sin sesión")
        box = ttk.Frame(self, padding=24)
        box.pack(fill="both", expand=True)
        ttk.Label(box, text="SkyLauncher · by T4skor", font=("sans-serif", 24, "bold")).pack(anchor="w")
        ttk.Label(box, text="Minecraft Java Edition", font=("sans-serif", 12)).pack(anchor="w", pady=(0, 16))
        fields = ttk.Frame(box)
        fields.pack(fill="x")
        fields.columnconfigure(1, weight=1)
        for row, (label, var) in enumerate([
            ("Microsoft · Client ID", self.client),
            ("RAM (GB, 1–64)", self.ram),
            ("Java (opcional)", self.java),
        ]):
            ttk.Label(fields, text=label).grid(row=row, column=0, sticky="w", padx=(0, 16), pady=6)
            ttk.Entry(fields, textvariable=var).grid(row=row, column=1, sticky="ew", pady=6)
        ttk.Label(fields, text="Versión").grid(row=3, column=0, sticky="w")
        self.selector = ttk.Combobox(fields, textvariable=self.version, state="readonly")
        self.selector.grid(row=3, column=1, sticky="ew", pady=6)
        ttk.Checkbutton(box, text="Mostrar snapshots", variable=self.snapshots, command=self.filter_versions).pack(anchor="w", pady=8)
        ttk.Label(box, textvariable=self.account).pack(anchor="w", pady=6)
        actions = ttk.Frame(box)
        actions.pack(fill="x", pady=10)
        self.login_button = ttk.Button(actions, text="Iniciar sesión", command=self.login)
        self.login_button.pack(side="left")
        self.logout_button = ttk.Button(actions, text="Cerrar sesión", command=self.logout)
        self.logout_button.pack(side="left", padx=8)
        self.play_button = ttk.Button(actions, text="Instalar y jugar", command=self.play)
        self.play_button.pack(side="right")
        self.progress = ttk.Progressbar(box, mode="determinate")
        self.progress.pack(fill="x", pady=10)
        ttk.Label(box, textvariable=self.status, wraplength=580).pack(anchor="w")
        ttk.Label(box, text=f"Datos y registro: {DATA}", wraplength=580).pack(anchor="w", pady=16)
        self.after(80, self.drain)
        self.run_task(self.load_versions)

    def save_settings(self, ram=None):
        DATA.mkdir(parents=True, exist_ok=True)
        self.settings = {"client_id": self.client.get().strip(), "ram": ram or self.ram.get(),
                         "java": self.java.get().strip(), "version": self.version.get()}
        temporary = CONFIG.with_suffix(".tmp")
        temporary.write_text(json.dumps(self.settings, indent=2), encoding="utf-8")
        temporary.replace(CONFIG)

    def run_task(self, function):
        if self.busy:
            return
        self.busy = True
        for button in (self.login_button, self.logout_button, self.play_button):
            button.configure(state="disabled")

        def worker():
            try:
                function()
            except Exception as exc:
                # Never expose authentication responses or tokens in logs/dialogs.
                self.events.put(("error", type(exc).__name__))
            finally:
                self.events.put(("done", None))
        threading.Thread(target=worker, daemon=True).start()

    def drain(self):
        try:
            while True:
                kind, value = self.events.get_nowait()
                if kind == "done":
                    self.busy = False
                    for button in (self.login_button, self.logout_button, self.play_button):
                        button.configure(state="normal")
                elif kind == "versions":
                    self.versions = value
                    self.filter_versions()
                elif kind == "profile":
                    self.profile = value
                    self.account.set(f"Conectado: {value['name']}")
                elif kind == "maximum":
                    self.progress.configure(maximum=max(1, value), value=0)
                elif kind == "progress":
                    self.progress.configure(value=value)
                elif kind == "error":
                    self.status.set(f"Error: {value}. Consulta la sección de ayuda del README.")
                    messagebox.showerror("No se pudo completar", self.status.get())
                else:
                    self.status.set(value)
        except queue.Empty:
            pass
        self.after(80, self.drain)

    def load_versions(self):
        self.events.put(("status", "Consultando versiones oficiales…"))
        self.events.put(("versions", mc.utils.get_version_list()))
        self.events.put(("status", "Listo. Inicia sesión para jugar."))

    def filter_versions(self):
        values = [v["id"] for v in self.versions if v["type"] == "release" or self.snapshots.get()]
        self.selector.configure(values=values)
        if values and self.version.get() not in values:
            self.version.set(values[0])

    def login(self):
        client = self.client.get().strip()
        if not client:
            messagebox.showinfo("Configurar Microsoft", "Introduce el Client ID de tu aplicación. Consulta README.md.")
            return
        self.save_settings()
        url, state, verifier = mc.microsoft_account.get_secure_login_data(client, REDIRECT)
        webbrowser.open(url)
        response = simpledialog.askstring("Inicio de sesión Microsoft",
            "Completa el inicio de sesión en el navegador.\n"
            "Copia aquí la URL final completa de la barra de direcciones\n"
            "(contiene code= y state=; la página puede quedar en blanco).", parent=self, show="*")
        if not response:
            return

        def authenticate():
            self.events.put(("status", "Verificando cuenta Microsoft…"))
            code = mc.microsoft_account.parse_auth_code_url(response.strip(), state)
            profile = mc.microsoft_account.complete_login(client, None, REDIRECT, code, verifier)
            self.events.put(("profile", profile))
            self.events.put(("status", "Sesión iniciada. Ya puedes instalar y jugar."))
        self.run_task(authenticate)

    def logout(self):
        self.profile = None
        self.account.set("Sin sesión")
        self.status.set("Sesión local cerrada.")

    def play(self):
        if not self.profile:
            messagebox.showinfo("Iniciar sesión", "Inicia sesión con tu cuenta de Minecraft Java.")
            return
        version = self.version.get()
        if not version:
            messagebox.showinfo("Versión", "Espera a que cargue la lista de versiones.")
            return
        try:
            ram = int(self.ram.get())
            if not 1 <= ram <= 64:
                raise ValueError
        except ValueError:
            messagebox.showerror("RAM", "Introduce un número entero entre 1 y 64 GB.")
            return
        self.save_settings(ram)
        profile = dict(self.profile)
        java = self.java.get().strip()

        def launch():
            callbacks = {"setStatus": lambda text: self.events.put(("status", text)),
                         "setProgress": lambda n: self.events.put(("progress", n)),
                         "setMax": lambda n: self.events.put(("maximum", n))}
            mc.install.install_minecraft_version(version, str(GAME), callback=callbacks)
            options = {"username": profile["name"], "uuid": profile["id"],
                       "token": profile["access_token"], "jvmArguments": [f"-Xmx{ram}G", "-Xms512M"],
                       "launcherName": "SkyLauncher", "launcherVersion": "0.3.0",
                       "gameDirectory": str(GAME)}
            if java:
                options["executablePath"] = os.path.expanduser(java)
            command = mc.command.get_minecraft_command(version, str(GAME), options)
            self.events.put(("status", "Minecraft está en ejecución…"))
            with (DATA / "game.log").open("w", encoding="utf-8") as log:
                process = subprocess.Popen(command, cwd=GAME, stdout=log, stderr=subprocess.STDOUT)
                code = process.wait()
            self.events.put(("status", "Minecraft cerrado." if code == 0 else
                             f"Minecraft terminó con código {code}. Consulta game.log."))
        self.run_task(launch)


if __name__ == "__main__":
    Launcher().mainloop()

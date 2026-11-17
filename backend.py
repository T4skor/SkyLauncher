"""Motor JSON-lines de la interfaz Rust. Nunca transmite tokens al frontend."""
import json
import hashlib
import io
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys
import uuid
import zipfile
import urllib.request
import struct
import zlib
import time
import tempfile
import base64
from urllib.parse import urlparse, urlencode

import minecraft_launcher_lib as mc
import requests

DATA = Path(os.environ.get("MI_LAUNCHER_DATA", str(Path.home() / ".mi-minecraft-launcher"))).expanduser()
GAME = DATA / "minecraft"
CONFIG = DATA / "rust-config.json"
REDIRECT = "https://login.microsoftonline.com/common/oauth2/nativeclient"
profile = None
pending = None
LOADERS = {"fabric": "Fabric", "forge": "Forge", "neoforge": "NeoForge", "quilt": "Quilt"}
AGENT = "SkyLauncher/0.3.0 (Minecraft desktop launcher)"
MAX_SKIN = 2 * 1024 * 1024
profile_client = ""
profile_deadline = 0.
session_refresh_token = ""


class UserError(Exception):
    """Mensaje local que se puede mostrar sin incluir credenciales."""


def managed_dir(*parts):
    path = DATA
    for part in (None, *parts):
        if part is not None:
            path /= part
        if path.is_symlink():
            raise UserError("Una carpeta de datos gestionados es un enlace simbólico.")
        path.mkdir(parents=True, exist_ok=True)
    return path


def write_json(path, value, backup=False):
    temporary = path.with_suffix(path.suffix + ".tmp")
    previous = path.with_suffix(path.suffix + ".bak")
    if any(p.is_symlink() for p in (path, temporary, previous)):
        raise UserError("No se puede guardar sobre un enlace simbólico.")
    with temporary.open("w", encoding="utf-8") as writer:
        json.dump(value, writer, indent=2, ensure_ascii=False)
        writer.flush()
        os.fsync(writer.fileno())
    if backup and path.exists():
        shutil.copyfile(path, previous)
    temporary.replace(path)


def profile_store():
    path = managed_dir() / "profiles.json"
    if not path.exists():
        return {"schema": 1, "profiles": []}
    try:
        if path.is_symlink() or path.stat().st_size > 1024 * 1024:
            raise ValueError
        value = json.loads(path.read_text(encoding="utf-8"))
        if value["schema"] != 1 or not isinstance(value["profiles"], list):
            raise ValueError
        seen = set()
        for p in value["profiles"]:
            if (not re.fullmatch(r"[0-9a-f]{32}", p["id"]) or p["id"] in seen
                    or not re.fullmatch(r"[A-Za-z0-9_]{3,16}", p["name"])
                    or p.get("variant") not in ("classic", "slim")):
                raise ValueError
            seen.add(p["id"])
            version_dir(p["version"])
            if p.get("skin") and not re.fullmatch(r"[0-9a-f]{64}\.png", p["skin"]):
                raise ValueError
        return value
    except (OSError, ValueError, KeyError, TypeError):
        raise UserError("profiles.json está dañado o no es compatible. No se ha sobrescrito; revisa su copia .bak.") from None


def find_profile(profile_id):
    value = next((p for p in profile_store()["profiles"] if p["id"] == profile_id), None)
    if value is None:
        raise UserError("El perfil ya no existe. Actualiza la lista.")
    return value


def profile_game(p):
    return GAME if p.get("legacy") is True else managed_dir("instances", p["id"], "minecraft")


def request_game(req=None):
    if req and req.get("offline") and req.get("local_profile"):
        return profile_game(find_profile(req["local_profile"]))
    return GAME


def mods_dir(req=None):
    path = request_game(req) / "mods"
    if path.is_symlink():
        raise UserError("La carpeta mods no puede ser un enlace simbólico.")
    path.mkdir(parents=True, exist_ok=True)
    return path


def mod_path_in(root, name):
    if not isinstance(name, str) or not name or any(c in name for c in "/\\") or not name.endswith((".jar", ".jar.disabled")):
        raise UserError("Nombre de mod no válido.")
    path = root / name
    if path.is_symlink() or path.resolve().parent != root.resolve():
        raise UserError("Ruta de mod no válida.")
    return path


def mod_path(name, req=None):
    return mod_path_in(mods_dir(req), name)


def list_mods(req=None):
    items = []
    for path in sorted(mods_dir(req).iterdir(), key=lambda p: p.name.lower()):
        if path.is_file() and not path.is_symlink() and path.name.endswith((".jar", ".jar.disabled")):
            items.append({"name": path.name, "enabled": path.name.endswith(".jar"), "size": path.stat().st_size})
    emit("mods", mods=items)


def manage_mod(req):
    action = req["action"]
    if action == "mod_import":
        source = Path(req["path"]).expanduser()
        if source.suffix != ".jar" or not source.is_file() or not zipfile.is_zipfile(source):
            raise UserError("Selecciona un mod .jar válido (archivo Java/ZIP).")
        destination = mod_path(source.name, req)
        if destination.exists() or mod_path(source.name + ".disabled", req).exists():
            raise UserError("Ese mod ya está en la biblioteca. Elimínalo antes de reemplazarlo.")
        try:
            with source.open("rb") as reader, destination.open("xb") as writer:
                try:
                    shutil.copyfileobj(reader, writer)
                except Exception:
                    destination.unlink(missing_ok=True)
                    raise
        except FileExistsError:
            raise UserError("Ese mod ya existe.") from None
    else:
        source = mod_path(req["name"], req)
        if not source.is_file():
            raise UserError("El mod ya no existe. Actualiza la lista.")
        if action == "mod_delete":
            source.unlink()
        elif action == "mod_toggle":
            name = source.name.removesuffix(".disabled") if source.name.endswith(".disabled") else source.name + ".disabled"
            destination = mod_path(name, req)
            if destination.exists():
                raise UserError("Ya existe otra copia del mod con ese estado.")
            source.rename(destination)
    list_mods(req)
    emit_profiles()
    status("Biblioteca de mods actualizada.")


def emit(kind, **fields):
    print(json.dumps({"event": kind, **fields}), flush=True)


def status(text):
    emit("status", text=text)


def installation_callbacks():
    maximum = [1]

    def progress(value):
        emit("progress", value=value / max(1, maximum[0]))

    def set_max(value):
        maximum[0] = value
        progress(0)

    return {"setStatus": status, "setMax": set_max, "setProgress": progress}


def installed():
    emit("installed", versions=[v["id"] for v in mc.utils.get_installed_versions(str(GAME))])


def version_dir(version):
    if not isinstance(version, str) or not version or version in (".", "..") or any(c in version for c in "/\\"):
        raise ValueError("Versión no válida")
    path = GAME / "versions" / version
    if path.is_symlink() or path.resolve().parent != (GAME / "versions").resolve():
        raise ValueError("Ruta de versión no válida")
    return path


def settings(req):
    defaults = {"client_id": "", "java": "", "version": "", "ram": 4, "snapshots": False,
                "offline": False, "offline_name": "", "local_profile": ""}
    if CONFIG.exists():
        try:
            old = json.loads(CONFIG.read_text(encoding="utf-8"))
            if not isinstance(old, dict):
                raise ValueError
            defaults.update({k: old[k] for k in defaults if k in old})
        except (OSError, ValueError):
            raise UserError("La configuración está dañada; no se ha sobrescrito.") from None
    value = {k: req.get(k, defaults[k]) for k in defaults}
    value["offline"] = value["offline"] is True
    value["client_id"] = str(value["client_id"]).strip()
    value["offline_name"] = str(value["offline_name"]).strip()
    if value["local_profile"]:
        find_profile(value["local_profile"])
    value["ram"] = int(value["ram"])
    if not 1 <= value["ram"] <= 64:
        raise ValueError("RAM no válida")
    managed_dir()
    write_json(CONFIG, value)
    return value


def credential_backend():
    """Use an OS vault explicitly: never keyring's configurable plaintext fallbacks."""
    if sys.platform.startswith("linux"):
        from keyring.backends.SecretService import Keyring
    elif sys.platform == "win32":
        from keyring.backends.Windows import WinVaultKeyring as Keyring
    elif sys.platform == "darwin":
        from keyring.backends.macOS import Keyring
    else:
        raise RuntimeError("No secure credential store for this platform")
    vault = Keyring()
    if sys.platform.startswith("linux"):
        vault.appid = "SkyLauncher"
    return vault


def credential_service():
    scope = hashlib.sha256(str(DATA.resolve()).encode()).hexdigest()[:32]
    return "SkyLauncher/Microsoft/" + scope


class MicrosoftHTTP(requests.Session):
    def request(self, method, url, **kwargs):
        kwargs.setdefault("timeout", (10, 20))
        return super().request(method, url, **kwargs)


def microsoft_call(function, *args):
    # minecraft-launcher-lib 8.0's public auth helpers have no timeout parameter.
    # Only its auth module gets this scoped HTTP adapter, not requests globally.
    # The JSON backend dispatches synchronously; restore the module in all cases.
    module = mc.microsoft_account
    original = module.requests
    with MicrosoftHTTP() as http:
        module.requests = http
        try:
            return function(*args)
        finally:
            module.requests = original


def session_state():
    path = DATA / "session-state.json"
    if not path.exists():
        return {"schema": 1, "restore": False, "cleanup_pending": False}
    try:
        if path.is_symlink() or path.stat().st_size > 4096:
            raise ValueError
        value = json.loads(path.read_text(encoding="utf-8"))
        if (value.get("schema") != 1 or type(value.get("restore")) is not bool
                or type(value.get("cleanup_pending")) is not bool):
            raise ValueError
        return value
    except (OSError, ValueError, TypeError, AttributeError):
        raise UserError("El estado de sesión no es válido. No se ha leído ninguna credencial.") from None


def set_session_state(restore, cleanup_pending=False):
    write_json(managed_dir() / "session-state.json",
               {"schema": 1, "restore": restore, "cleanup_pending": cleanup_pending})


def session_notice(text, remembered=False):
    try:
        state = session_state()
        can_restore = state["restore"]
        can_forget = state["restore"] or state["cleanup_pending"]
    except UserError:
        can_restore, can_forget = False, True
    emit("session_state", text=text, remembered=remembered,
         can_restore=can_restore, can_forget=can_forget)


def remember_session(client):
    """Persist only the renewable Microsoft token, not the Minecraft access token."""
    # A tombstone prevents an older account from being restored if saving fails.
    try:
        set_session_state(False, cleanup_pending=True)
        if not session_refresh_token:
            session_notice("Sesión solo en memoria: Microsoft no devolvió un token de renovación.")
            return
        payload = json.dumps({"schema": 1, "client_id": client, "refresh_token": session_refresh_token})
        credential_backend().set_password(credential_service(), "session", payload)
        set_session_state(True)
    except Exception:
        # OS keyring errors may contain secrets. Never propagate their text.
        session_notice("Sesión solo para esta vez: no se pudo guardar en el llavero. Desbloquea el almacén de credenciales del escritorio.")
        return
    session_notice("Sesión guardada de forma segura. Se restaurará al abrir el launcher.", True)


def accept_online_profile(value, client):
    global profile, profile_client, profile_deadline, session_refresh_token
    if (not isinstance(value, dict)
            or not re.fullmatch(r"[A-Za-z0-9_]{3,16}", str(value.get("name", "")))
            or not re.fullmatch(r"[0-9a-fA-F]{32}", str(value.get("id", "")))
            or not isinstance(value.get("access_token"), str) or not value["access_token"]):
        raise UserError("Microsoft no devolvió un perfil de Minecraft válido.")
    refresh = value.get("refresh_token", "")
    if not isinstance(refresh, str) or len(refresh) > 16384:
        raise UserError("Microsoft no devolvió una sesión renovable válida.")
    profile = {key: value[key] for key in ("name", "id", "access_token")}
    profile_client = client
    lifetime = 45 * 60
    try:
        claim = value["access_token"].split(".")[1]
        if len(claim) <= 16384:
            expiry = json.loads(base64.urlsafe_b64decode(claim + "=" * (-len(claim) % 4)))["exp"]
            if type(expiry) in (int, float):
                lifetime = max(0, min(lifetime, expiry - time.time() - 60))
    except (ValueError, TypeError, IndexError, KeyError):
        pass
    profile_deadline = time.monotonic() + lifetime
    session_refresh_token = refresh
    emit("profile", name=profile["name"])


def forget_session():
    global profile, pending, profile_client, profile_deadline, session_refresh_token
    # Must be durable before clearing memory: even a locked vault must not log back in.
    set_session_state(False, cleanup_pending=True)
    profile = pending = None
    profile_client, profile_deadline, session_refresh_token = "", 0., ""
    emit("profile", name="")
    try:
        vault = credential_backend()
        if vault.get_password(credential_service(), "session") is not None:
            vault.delete_password(credential_service(), "session")
        set_session_state(False)
    except Exception:
        session_notice("Sesión cerrada y restauración desactivada. No se pudo borrar la credencial del llavero; desbloquéalo y pulsa Olvidar sesión guardada.")
        return
    session_notice("Sesión cerrada y credencial guardada eliminada.")


def restore_session(client):
    client = client.strip() if isinstance(client, str) else ""
    try:
        state = session_state()
    except UserError:
        session_notice("No se pudo leer el estado de sesión. Revisa session-state.json; no se ha restaurado ninguna cuenta.")
        return False
    if not state["restore"]:
        session_notice("No hay una sesión guardada activa." if not state["cleanup_pending"] else
                       "La restauración está desactivada. Desbloquea el llavero y pulsa Olvidar sesión guardada para borrar la credencial pendiente.")
        return False
    if not client:
        session_notice("Configura el Client ID original para restaurar la sesión guardada.")
        return False
    try:
        raw = credential_backend().get_password(credential_service(), "session")
        if raw is None:
            set_session_state(False)
            session_notice("No se encontró la credencial guardada. Inicia sesión de nuevo.")
            return False
        if not isinstance(raw, str) or len(raw) > 32768:
            raise ValueError
        saved = json.loads(raw)
        if (saved.get("schema") != 1 or not isinstance(saved.get("client_id"), str)
                or not isinstance(saved.get("refresh_token"), str)
                or not 1 <= len(saved["refresh_token"]) <= 16384):
            raise ValueError
    except (ValueError, TypeError, AttributeError):
        session_notice("La credencial guardada no es válida. Olvida la sesión guardada e inicia sesión de nuevo.")
        return False
    except Exception:
        session_notice("No se pudo abrir el llavero. Desbloquea tu almacén de credenciales y pulsa Restaurar sesión guardada.")
        return False
    if saved["client_id"] != client:
        session_notice("La sesión guardada corresponde a otro Client ID. Usa el original o inicia una sesión nueva.")
        return False
    status("Restaurando sesión Microsoft…")
    try:
        value = microsoft_call(mc.microsoft_account.complete_refresh, client, None, REDIRECT, saved["refresh_token"])
    except (mc.exceptions.InvalidRefreshToken, mc.exceptions.AccountNotOwnMinecraft):
        forget_session()
        session_notice("Microsoft ya no permite renovar esta sesión. Inicia sesión de nuevo.")
        return False
    except Exception:
        session_notice("No se pudo verificar la sesión. La credencial sigue guardada; comprueba la conexión y pulsa Restaurar sesión guardada.", True)
        return False
    accept_online_profile(value, client)
    remember_session(client)
    return True


def ensure_online_session(client=None):
    if not profile:
        raise ValueError("Inicia sesión para jugar")
    if isinstance(client, str):
        client = client.strip()
    if client is not None and profile_client and client != profile_client:
        raise UserError("El Client ID ha cambiado. Usa el original o vuelve a iniciar sesión.")
    if time.monotonic() < profile_deadline:
        return False
    if not session_refresh_token or not profile_client:
        raise UserError("La sesión necesita renovarse. Vuelve a iniciar sesión.")
    status("Renovando sesión Microsoft…")
    try:
        value = microsoft_call(mc.microsoft_account.complete_refresh, profile_client, None, REDIRECT, session_refresh_token)
    except (mc.exceptions.InvalidRefreshToken, mc.exceptions.AccountNotOwnMinecraft):
        forget_session()
        raise UserError("La sesión Microsoft ha caducado o fue revocada. Inicia sesión de nuevo.") from None
    except Exception:
        raise UserError("No se pudo renovar la sesión. Comprueba tu conexión e inténtalo de nuevo.") from None
    client = profile_client
    accept_online_profile(value, client)
    remember_session(client)
    return True


def launch_identity(cfg):
    if cfg.get("offline"):
        name = cfg.get("offline_name", "").strip()
        if not re.fullmatch(r"[A-Za-z0-9_]{3,16}", name):
            raise ValueError("Nombre local: usa entre 3 y 16 letras, números o guiones bajos")
        # Equivalente a Java UUID.nameUUIDFromBytes, sin namespace adicional.
        digest = hashlib.md5(("OfflinePlayer:" + name).encode("utf-8")).digest()
        return {"name": name, "id": uuid.UUID(bytes=digest, version=3).hex, "access_token": "0"}
    if not profile:
        raise ValueError("Inicia sesión para jugar")
    ensure_online_session(cfg.get("client_id"))
    return profile


def skin_request(method, suffix, **kwargs):
    if not profile:
        raise ValueError("Inicia sesión primero")
    ensure_online_session()
    response = requests.request(method, "https://api.minecraftservices.com/minecraft/profile" + suffix,
                                headers={"Authorization": "Bearer " + profile["access_token"]},
                                timeout=45, **kwargs)
    if not response.ok:
        raise ValueError(f"Servicio de skins: HTTP {response.status_code}. Comprueba la sesión.")
    return response


def read_skin(path):
    path = Path(path).expanduser()
    if not path.is_file() or path.stat().st_size > MAX_SKIN:
        raise UserError("Selecciona un PNG de skin de hasta 2 MB.")
    data = path.read_bytes()
    validate_skin(data)
    return data


def validate_skin(data):
    """Validate the PNG container, CRCs and bounded image stream before storing it."""
    if len(data) > MAX_SKIN or not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise UserError("La skin no es un PNG válido.")
    offset, compressed, header, ended = 8, bytearray(), None, False
    try:
        while offset < len(data):
            length, kind = struct.unpack(">I4s", data[offset:offset + 8])
            chunk = data[offset + 8:offset + 8 + length]
            crc = struct.unpack(">I", data[offset + 8 + length:offset + 12 + length])[0]
            if len(chunk) != length or zlib.crc32(kind + chunk) & 0xffffffff != crc:
                raise ValueError
            if header is None and kind != b"IHDR":
                raise ValueError
            if kind == b"IHDR":
                if header is not None or length != 13:
                    raise ValueError
                header = struct.unpack(">IIBBBBB", chunk)
                width, height, depth, color, compression, filtering, interlace = header
                if (width != 64 or height not in (32, 64) or depth != 8
                        or color not in (2, 3, 6) or compression or filtering or interlace):
                    raise ValueError
            elif kind == b"IDAT":
                compressed.extend(chunk)
            elif kind == b"IEND":
                ended = length == 0 and offset + 12 == len(data)
                break
            offset += length + 12
        if not ended or not header or not compressed:
            raise ValueError
        decoder = zlib.decompressobj()
        pixels = decoder.decompress(bytes(compressed), 65537)
        channels = {2: 3, 3: 1, 6: 4}[header[3]]
        row = 1 + 64 * channels
        if (len(pixels) != row * header[1] or not decoder.eof or decoder.unused_data
                or any(pixels[i] > 4 for i in range(0, len(pixels), row))):
            raise ValueError
    except (ValueError, struct.error, zlib.error, KeyError):
        raise UserError("Usa una skin PNG de 64×64 o 64×32, de 8 bits y sin entrelazado.") from None


def save_skin(data):
    validate_skin(data)
    path = managed_dir("skins") / (hashlib.sha256(data).hexdigest() + ".png")
    if path.is_symlink():
        raise UserError("Ruta de skin no válida.")
    if not path.exists():
        with path.open("xb") as writer:
            writer.write(data)
    return path


def skin_path(p):
    if not p.get("skin"):
        return None
    path = managed_dir("skins") / p["skin"]
    if path.is_symlink():
        raise UserError("Ruta de skin no válida.")
    return path


def support_marker(p):
    path = profile_game(p) / "skylauncher-skin-support.json"
    try:
        if path.is_symlink() or path.stat().st_size > 8192:
            return {}
        value = json.loads(path.read_text(encoding="utf-8"))
        filename = value["file"]
        if not re.fullmatch(r"[A-Za-z0-9_.+-]+\.jar", filename):
            return {}
        mod = profile_game(p) / "mods" / filename
        if mod.is_symlink() or not mod.is_file():
            return {}
        return value
    except (OSError, ValueError, KeyError, TypeError):
        return {}


def emit_profiles():
    values = []
    for p in profile_store()["profiles"]:
        path = skin_path(p)
        values.append({**p, "skin_path": str(path) if path else "",
                       "skin_support_version": support_marker(p).get("version", "")})
    emit("local_profiles", profiles=values)


def emit_selected(p):
    path = skin_path(p)
    emit("local_profile_selected", value={**p, "skin_path": str(path) if path else ""})


def draft_profile(req, legacy=False):
    """Validate a profile edit without publishing it or changing its runtime skin."""
    name = str(req.get("offline_name", "")).strip()
    version = req.get("version", "")
    if not re.fullmatch(r"[A-Za-z0-9_]{3,16}", name):
        raise UserError("Nombre local: usa entre 3 y 16 letras, números o guiones bajos.")
    version_dir(version)
    variant = req.get("variant", "classic")
    if variant not in ("classic", "slim"):
        raise UserError("El modelo de skin debe ser Classic o Slim.")
    store = profile_store()
    profile_id = req.get("local_profile", "")
    p = next((p for p in store["profiles"] if p["id"] == profile_id), None)
    if profile_id and p is None:
        raise UserError("Ese perfil no existe; no se ha creado otro por error.")
    if p is None:
        if len(store["profiles"]) >= 100:
            raise UserError("Se ha alcanzado el límite de 100 perfiles locales.")
        p = {"id": uuid.uuid4().hex, "legacy": legacy}
        store["profiles"].append(p)
    source = req.get("skin_path", "")
    skin = save_skin(read_skin(source)).name if source else ""
    if skin and variant == "slim":
        # Legacy 64×32 textures only have Classic arms.
        if struct.unpack(">I", (managed_dir("skins") / skin).read_bytes()[20:24])[0] == 32:
            raise UserError("Las skins de 64×32 solo admiten Classic.")
    p.update(name=name, version=version, skin=skin, variant=variant)
    return store, p


def save_profile(req, legacy=False):
    store, p = draft_profile(req, legacy)
    write_json(managed_dir() / "profiles.json", store, backup=True)
    profile_game(p).mkdir(parents=True, exist_ok=True)
    if not p.get("skin"):
        prepare_local_skin(p, profile_game(p))
    emit_profiles()
    emit_selected(p)
    return p


def selected_settings(req, p):
    value = settings({**req, "offline": True, "offline_name": p["name"],
                      "version": p["version"], "local_profile": p["id"]})
    emit("settings", value=value)
    list_mods(value)


def base_version_and_loader(version):
    """Resolve installed inheritance, without guessing loader compatibility from its label."""
    loader, seen = "vanilla", set()
    while True:
        if version in seen or len(seen) > 8:
            raise UserError("La herencia de esta versión no es válida.")
        seen.add(version)
        path = version_dir(version) / (version + ".json")
        if not path.exists():
            return version, loader
        if path.is_symlink() or path.stat().st_size > 8 * 1024 * 1024:
            raise UserError("Metadatos de versión no válidos.")
        info = json.loads(path.read_text(encoding="utf-8"))
        for lib in info.get("libraries", []):
            name = lib.get("name", "")
            for prefix, kind in (("net.minecraftforge:forge:", "forge"),
                                 ("net.neoforged:neoforge:", "neoforge"),
                                 ("net.neoforged:forge:", "neoforge"),
                                 ("net.fabricmc:fabric-loader:", "fabric"),
                                 ("org.quiltmc:quilt-loader:", "quilt")):
                if name.startswith(prefix) and loader == "vanilla":
                    loader = kind
        parent = info.get("inheritsFrom")
        if not parent:
            return version, loader
        version = parent


def fetch_bytes(url, limit, hosts):
    """Only documented HTTPS providers; reject redirects rather than following arbitrary hosts."""
    parsed = urlparse(url)
    if parsed.scheme != "https" or parsed.hostname not in hosts or parsed.username or parsed.port not in (None, 443):
        raise UserError("El proveedor devolvió una URL no permitida.")
    with requests.get(url, headers={"User-Agent": AGENT}, timeout=(10, 30),
                      stream=True, allow_redirects=False) as response:
        if response.status_code == 429:
            raise UserError("El servicio ha limitado las consultas. Espera un poco antes de actualizar.")
        if response.status_code != 200:
            raise UserError(f"El servicio no está disponible (HTTP {response.status_code}).")
        data = bytearray()
        for chunk in response.iter_content(65536):
            data.extend(chunk)
            if len(data) > limit:
                raise UserError("La descarga supera el tamaño permitido.")
        return bytes(data)


def fetch_json(url, host):
    try:
        return json.loads(fetch_bytes(url, 4 * 1024 * 1024, {host}))
    except (ValueError, UnicodeError):
        raise UserError("El servicio devolvió una respuesta no válida.") from None


def mod_catalog_context(req):
    # Validate the destination profile even for searches, without persisting settings.
    if req.get("offline") and req.get("local_profile"):
        find_profile(req["local_profile"])
    base, loader = base_version_and_loader(req.get("version", ""))
    if loader not in LOADERS:
        raise UserError("Primero instala y selecciona Fabric, Forge, NeoForge o Quilt para usar el catálogo de mods.")
    return base, loader


def modrinth_id(value):
    if not isinstance(value, str) or not re.fullmatch(r"[A-Za-z0-9_-]{1,64}", value):
        raise UserError("Identificador de Modrinth no válido.")
    return value


def modrinth_get(route):
    return fetch_json("https://api.modrinth.com/v2/" + route, "api.modrinth.com")


def mod_catalog_search(req):
    base, loader = mod_catalog_context(req)
    query, offset = req.get("query", ""), req.get("offset", 0)
    if not isinstance(query, str) or len(query) > 200 or type(offset) is not int or not 0 <= offset <= 1000:
        raise UserError("Búsqueda de mods no válida (máximo 200 caracteres y página 1000).")
    params = urlencode({"query": query.strip(), "limit": 20, "offset": offset, "index": "relevance",
                        "facets": json.dumps([["project_type:mod"], ["versions:" + base],
                                              ["categories:" + loader]])})
    value = modrinth_get("search?" + params)
    if not isinstance(value, dict) or not isinstance(value.get("hits"), list):
        raise UserError("Modrinth devolvió un catálogo no válido.")
    items = []
    for hit in value["hits"][:20]:
        if not isinstance(hit, dict) or hit.get("project_type") != "mod":
            continue
        project_id = modrinth_id(hit.get("project_id"))
        downloads = hit.get("downloads", 0)
        items.append({"project_id": project_id, "title": str(hit.get("title", ""))[:160],
                      "description": str(hit.get("description", ""))[:600],
                      "author": str(hit.get("author", ""))[:160],
                      "downloads": downloads if type(downloads) is int and downloads >= 0 else 0})
    total = value.get("total_hits", len(items))
    emit("mod_catalog", items=items, minecraft_version=base, loader=loader, offset=offset,
         total_hits=total if type(total) is int and total >= 0 else len(items))
    status("Catálogo Modrinth actualizado. Los mods son código de terceros; revisa su autor antes de instalar.")


def mod_catalog_plan(project_id, base, loader):
    """Resolve a bounded DAG, rejecting cycles and conflicting pinned versions."""
    selected, visiting, ordered, projects = {}, set(), [], {}
    params = urlencode({"game_versions": json.dumps([base]), "loaders": json.dumps([loader])})

    def compatible(v):
        return (isinstance(v, dict) and v.get("version_type") in ("release", "beta", "alpha")
                and base in v.get("game_versions", []) and loader in v.get("loaders", []))

    def pick_release(releases):
        # Prefer the latest stable release. New Minecraft versions often only
        # have beta/alpha builds for a few weeks; falling back to them keeps the
        # catalog usable instead of failing after the user picked a mod.
        stable = [v for v in releases if v.get("version_type") == "release"]
        pool = stable or [v for v in releases if v.get("version_type") in ("beta", "alpha")]
        if not pool:
            return None
        return max(pool, key=lambda v: (str(v.get("date_published", "")),
                                        v.get("version_type") == "release"))

    def visit(project=None, version=None, depth=0):
        if depth >= 16:
            raise UserError("Demasiados niveles de dependencias (máximo 16).")
        if project:
            project = modrinth_id(project)
        if version:
            version = modrinth_id(version)
            release = modrinth_get("version/" + version)
            if not compatible(release) or release.get("id") != version:
                raise UserError("Una dependencia fijada no tiene una versión compatible con Minecraft y el loader elegidos.")
            if project and release.get("project_id") != project:
                raise UserError("La dependencia no pertenece al proyecto esperado.")
            project = modrinth_id(release.get("project_id"))
        elif project:
            if project in visiting:
                raise UserError("Modrinth declara un ciclo de dependencias.")
            if project in selected:
                return
            releases = modrinth_get("project/" + project + "/version?" + params)
            if not isinstance(releases, list):
                raise UserError("Modrinth devolvió versiones no válidas.")
            releases = [v for v in releases if compatible(v) and v.get("project_id") == project]
            release = pick_release(releases)
            if release is None:
                raise UserError("No hay una release, beta o alfa compatible para el mod o una dependencia requerida.")
        else:
            raise UserError("Una dependencia requerida no identifica proyecto ni versión.")
        version = modrinth_id(release.get("id"))
        if project in visiting:
            raise UserError("Modrinth declara un ciclo de dependencias.")
        if project in selected:
            if selected[project] != version:
                raise UserError("Dos mods requieren versiones distintas de la misma dependencia.")
            return
        if len(selected) >= 32:
            raise UserError("Demasiadas dependencias (máximo 32 proyectos).")
        if project not in projects:
            projects[project] = modrinth_get("project/" + project)
        metadata = projects[project]
        if not isinstance(metadata, dict) or metadata.get("project_type") != "mod" or metadata.get("id") != project:
            raise UserError("El proyecto o una dependencia no es un mod válido.")
        selected[project] = version
        visiting.add(project)
        dependencies = release.get("dependencies", [])
        if not isinstance(dependencies, list) or len(dependencies) > 128:
            raise UserError("Lista de dependencias no válida.")
        for dep in dependencies:
            if not isinstance(dep, dict):
                raise UserError("Dependencia no válida.")
            if dep.get("dependency_type") == "required":
                visit(dep.get("project_id"), dep.get("version_id"), depth + 1)
        visiting.remove(project)
        ordered.append(release)

    visit(modrinth_id(project_id))
    return ordered


def mod_catalog_install(req):
    base, loader = mod_catalog_context(req)
    status("Resolviendo mods y dependencias requeridas en Modrinth…")
    releases = mod_catalog_plan(req.get("project_id"), base, loader)
    artifacts, names = [], set()
    for release in releases:
        files = release.get("files", [])
        if not isinstance(files, list) or not all(isinstance(f, dict) for f in files):
            raise UserError("Modrinth no devolvió archivos válidos.")
        primary = [f for f in files if f.get("primary") is True]
        artifact = primary[0] if len(primary) == 1 else files[0] if len(files) == 1 else None
        if not artifact or not isinstance(artifact.get("filename"), str) or not re.fullmatch(
                r"[A-Za-z0-9_][A-Za-z0-9_.+-]{0,180}\.jar", artifact["filename"]):
            raise UserError("Nombre de archivo de mod no válido.")
        name = artifact["filename"]
        if name.casefold() in names:
            raise UserError("Dos mods usan el mismo nombre de archivo.")
        names.add(name.casefold())
        hashes = artifact.get("hashes", {})
        expected = hashes.get("sha512", "") if isinstance(hashes, dict) else ""
        if not isinstance(expected, str) or not re.fullmatch(r"[0-9a-f]{128}", expected):
            raise UserError("Falta la verificación SHA-512 del mod.")
        # fetch_bytes repeats this check and rejects every redirect.
        url = artifact.get("url", "")
        try:
            parsed = urlparse(url)
            allowed = (parsed.scheme == "https" and parsed.hostname == "cdn.modrinth.com"
                       and not parsed.username and parsed.port in (None, 443))
        except (ValueError, TypeError):
            allowed = False
        if not allowed:
            raise UserError("El proveedor devolvió una URL no permitida.")
        artifacts.append((name, url, expected))
    root = mods_dir(req)

    def check_conflicts():
        existing = {p.name.casefold() for p in root.iterdir()}
        reusable = set()
        for name, _, expected in artifacts:
            active = mod_path(name, req)
            mod_path(name + ".disabled", req)
            if (name + ".disabled").casefold() in existing:
                raise UserError("Una dependencia o mod está desactivado. Actívalo en Mods antes de instalar; no se ha reactivado.")
            if name.casefold() in existing:
                if (active.is_file() and active.stat().st_size <= 64 * 1024 * 1024
                        and hashlib.sha512(active.read_bytes()).hexdigest() == expected
                        and zipfile.is_zipfile(active)):
                    reusable.add(name)
                    continue
                raise UserError("Ya existe un mod con ese nombre (activo o desactivado). No se ha reemplazado.")
        return reusable

    reusable = check_conflicts()
    # Same filesystem: hard-link publication is atomic and never replaces a target.
    # Roll back only files published by this request on a recoverable commit error.
    with tempfile.TemporaryDirectory(prefix=".modrinth-stage-", dir=root) as temporary:
        staging, total = Path(temporary), 0
        for name, url, expected in artifacts:
            if name in reusable:
                continue
            data = fetch_bytes(url, 64 * 1024 * 1024, {"cdn.modrinth.com"})
            total += len(data)
            if total > 256 * 1024 * 1024:
                raise UserError("La instalación supera el límite de 256 MB.")
            if hashlib.sha512(data).hexdigest() != expected:
                raise UserError("La descarga no coincide con su hash SHA-512. No se ha instalado ningún mod.")
            if not zipfile.is_zipfile(io.BytesIO(data)):
                raise UserError("El archivo descargado no es un JAR válido.")
            (staging / name).write_bytes(data)
        current_reusable = check_conflicts()
        if not reusable.issubset(current_reusable):
            raise UserError("Un mod existente cambió durante la instalación. Recarga la lista e inténtalo de nuevo.")
        reusable = current_reusable
        published = []
        try:
            for name, _, _ in artifacts:
                if name in reusable:
                    continue
                destination = mod_path(name, req)
                os.link(staging / name, destination)
                published.append(destination)
        except Exception:
            for destination in reversed(published):
                if destination.exists() and os.path.samefile(destination, staging / destination.name):
                    destination.unlink()
            raise
    list_mods(req)
    unstable = sum(1 for release in releases if release.get("version_type") != "release")
    channel = ""
    if unstable == 1:
        channel = " Incluye una versión beta o alfa publicada por el autor."
    elif unstable > 1:
        channel = f" Incluye {unstable} versiones beta o alfa publicadas por el autor."
    status(f"Instalados {len(artifacts) - len(reusable)} mods; reutilizados {len(reusable)} idénticos activos "
           f"(incluidas dependencias requeridas). Reinicia Minecraft para usarlos.{channel}")


def texture_skin(texture):
    if not isinstance(texture, str) or not re.fullmatch(r"[0-9a-f]{32,64}", texture):
        raise UserError("Identificador de textura no válido.")
    cache = managed_dir("skin-catalog") / (texture + ".png")
    if cache.is_symlink():
        raise UserError("Ruta de caché no válida.")
    if cache.exists():
        read_skin(cache)
        return cache
    data = fetch_bytes("https://textures.minecraft.net/texture/" + texture, MAX_SKIN, {"textures.minecraft.net"})
    validate_skin(data)
    with cache.open("xb") as writer:
        writer.write(data)
    return cache


def catalog(req):
    cursor = req.get("after", "")
    if cursor and not re.fullmatch(r"[0-9a-f]{32}", cursor):
        raise UserError("Página de catálogo no válida.")
    url = "https://api.mineskin.org/v2/skins?size=8" + ("&after=" + cursor if cursor else "")
    value = fetch_json(url, "api.mineskin.org")
    if value.get("success") is not True or not isinstance(value.get("skins"), list):
        raise UserError("El catálogo no está disponible en este momento.")
    items = []
    for entry in value["skins"][:8]:
        if not re.fullmatch(r"[0-9a-f]{32}", str(entry.get("uuid", ""))):
            continue
        try:
            path = texture_skin(entry["texture"])
            items.append({"id": entry["uuid"], "name": str(entry.get("name") or "Skin " + entry["uuid"][:8])[:80],
                          "path": str(path)})
        except (UserError, requests.RequestException):
            continue
    if not items and value["skins"]:
        raise UserError("No se pudieron descargar las vistas previas. Inténtalo de nuevo.")
    after = ((value.get("pagination") or {}).get("next") or {}).get("after", "")
    emit("skin_catalog", skins=items, after=after if re.fullmatch(r"[0-9a-f]{32}", str(after)) else "")
    status("Catálogo público de MineSkin. Las skins pertenecen a sus autores.")


def choose_catalog_skin(req):
    skin_id = req.get("id", "")
    if not re.fullmatch(r"[0-9a-f]{32}", skin_id):
        raise UserError("Skin del catálogo no válida.")
    data = fetch_json("https://api.mineskin.org/v2/skins/" + skin_id, "api.mineskin.org")
    entry = data.get("skin", {})
    if data.get("success") is not True or entry.get("visibility") != "public":
        raise UserError("Esa skin ya no está disponible públicamente.")
    path = texture_skin(entry.get("texture", {}).get("hash", {}).get("skin"))
    variant = entry.get("variant")
    if variant not in ("classic", "slim"):
        raise UserError("El proveedor no especifica un modelo de skin compatible.")
    emit("skin", path=str(path), variant=variant)
    status("Skin seleccionada. Guárdala en tu perfil local o aplícala a tu cuenta Microsoft.")


def prepare_local_skin(p, game):
    path = skin_path(p)
    root = game / "CustomSkinLoader"
    if path is None and not root.exists():
        return
    # Do not follow symlinks in mod-controlled subdirectories.
    for directory in (root, root / "SkyLauncher", root / "SkyLauncher" / "skins"):
        if directory.is_symlink():
            raise UserError("La carpeta de skins contiene un enlace simbólico.")
        directory.mkdir(parents=True, exist_ok=True)
    config = root / "CustomSkinLoader.json"
    value = {"version": "0", "buildNumber": 0, "loadlist": [{"name": "Mojang", "type": "MojangAPI"}]}
    if config.exists():
        if config.is_symlink() or config.stat().st_size > 1024 * 1024:
            raise UserError("Configuración de CustomSkinLoader no válida.")
        try:
            value = json.loads(config.read_text(encoding="utf-8"))
            if not isinstance(value, dict) or not isinstance(value.get("loadlist", []), list):
                raise ValueError
        except (OSError, ValueError):
            raise UserError("La configuración de CustomSkinLoader está dañada; no se ha sobrescrito.") from None
    remaining = [e for e in value.get("loadlist", []) if isinstance(e, dict) and e.get("name") != "SkyLauncherLocal"]
    if path is not None:
        data = read_skin(path)
        destination = root / "SkyLauncher" / "skins" / (p["name"] + ".png")
        if destination.is_symlink():
            raise UserError("Ruta de skin local no válida.")
        destination.write_bytes(data)
        entry = {"name": "SkyLauncherLocal", "type": "Legacy",
                 "skin": "SkyLauncher/skins/{USERNAME}.png",
                 "model": "slim" if p["variant"] == "slim" else "default", "checkPNG": True}
        value["loadlist"] = [entry] + remaining
    else:
        value["loadlist"] = remaining
    value["enableLocalProfileCache"] = False
    write_json(config, value, backup=True)


def published_texture_url(value):
    """Extract the public PNG URL from a MineSkin API response."""
    if not isinstance(value, dict):
        return None
    data = value.get("data")
    texture = data.get("texture") if isinstance(data, dict) else None
    if not isinstance(texture, dict):
        texture = {}
    url = texture.get("url")
    if not url and isinstance(texture.get("urls"), dict):
        url = texture["urls"].get("skin")
    if isinstance(url, str) and re.fullmatch(
            r"https://textures\.minecraft\.net/texture/[0-9a-f]{32,64}", url):
        return url
    return None


def publish_skin(req):
    """Upload the chosen PNG so SkinsRestorer can fetch a public texture URL."""
    if not req.get("skin_path"):
        raise UserError("Elige una skin antes de publicarla.")
    variant = "slim" if req.get("variant") == "slim" else "classic"
    data = read_skin(req["skin_path"])
    # Legacy MineSkin upload accepts anonymous multipart requests and returns a
    # permanent textures.minecraft.net URL, the same shape SkinsRestorer uses.
    try:
        with requests.post(
            "https://api.mineskin.org/generate/upload",
            files={"file": ("skin.png", data, "image/png")},
            data={"variant": variant, "visibility": "0"},
            headers={"User-Agent": AGENT},
            timeout=(10, 60),
            allow_redirects=False,
            stream=True,
        ) as response:
            if response.status_code == 429:
                raise UserError("MineSkin ha limitado las publicaciones. Espera un poco y vuelve a intentarlo.")
            if response.status_code != 200:
                raise UserError(f"MineSkin no pudo publicar la skin (HTTP {response.status_code}).")
            body = bytearray()
            for chunk in response.iter_content(16384):
                body.extend(chunk)
                if len(body) > 256 * 1024:
                    raise UserError("MineSkin devolvió una respuesta demasiado grande.")
    except requests.RequestException:
        raise UserError("No se pudo contactar con MineSkin. Revisa la conexión e inténtalo de nuevo.") from None
    try:
        value = json.loads(bytes(body))
    except (ValueError, UnicodeError):
        raise UserError("MineSkin devolvió una respuesta no válida.") from None
    url = published_texture_url(value)
    if url is None:
        raise UserError("MineSkin no devolvió una URL de textura pública.")
    emit("skin_published", url=url, variant=variant)
    status("Skin publicada. Copia el comando y aplícalo en el servidor con SkinsRestorer.")


def enable_local_skin(req):
    if not req.get("offline"):
        raise UserError("Elige un perfil local para activar una skin sin Microsoft.")
    if not req.get("skin_path"):
        raise UserError("Elige una skin antes de activar su soporte en el juego.")
    base, loader = base_version_and_loader(req.get("version", ""))
    if loader not in LOADERS:
        raise UserError("Primero instala Fabric, Forge, NeoForge o Quilt para este perfil.")
    store, p = draft_profile(req)
    params = urlencode({"game_versions": json.dumps([base]), "loaders": json.dumps([loader])})
    versions = fetch_json("https://api.modrinth.com/v2/project/idMHQ4n2/version?" + params, "api.modrinth.com")
    compatible = [v for v in versions if base in v.get("game_versions", []) and loader in v.get("loaders", [])
                  and v.get("version_type") == "release" and not v.get("dependencies")]
    if not compatible:
        raise UserError("Modrinth no ofrece una versión estable compatible de CustomSkinLoader para este perfil.")
    artifact = next((f for f in compatible[0].get("files", []) if f.get("primary")), None)
    if not artifact or not re.fullmatch(r"[A-Za-z0-9_.+-]+\.jar", artifact.get("filename", "")):
        raise UserError("Modrinth no devolvió un archivo de mod válido.")
    expected = artifact.get("hashes", {}).get("sha512", "")
    if not re.fullmatch(r"[0-9a-f]{128}", expected):
        raise UserError("Falta la verificación SHA-512 del mod.")
    game = profile_game(p)
    context = {"offline": True, "local_profile": p["id"]}
    # A brand-new draft profile is not written yet: resolve its mods folder from
    # the profile record instead of looking it up on disk.
    mods_root = game / "mods"
    if mods_root.is_symlink():
        raise UserError("La carpeta mods no puede ser un enlace simbólico.")
    mods_root.mkdir(parents=True, exist_ok=True)
    destination = mod_path_in(mods_root, artifact["filename"])
    data = fetch_bytes(artifact["url"], 32 * 1024 * 1024, {"cdn.modrinth.com"})
    if hashlib.sha512(data).hexdigest() != expected:
        raise UserError("La descarga de CustomSkinLoader no coincide con su hash. No se ha instalado.")
    if not zipfile.is_zipfile(io.BytesIO(data)):
        raise UserError("El archivo de CustomSkinLoader no es un JAR válido.")
    previous = support_marker(p)
    if previous and previous["file"] != destination.name:
        old = mod_path_in(mods_root, previous["file"])
        if (hashlib.sha512(old.read_bytes()).hexdigest() != previous.get("sha512")
                or mod_path_in(mods_root, old.name + ".disabled").exists()):
            raise UserError("La instalación anterior del mod ha cambiado. Revísala en Mods antes de actualizar.")
    if destination.exists() and hashlib.sha512(destination.read_bytes()).hexdigest() != expected:
        raise UserError("Ya hay un archivo distinto con el nombre del mod; no se ha reemplazado.")
    disabled = mod_path_in(mods_root, artifact["filename"] + ".disabled")
    if disabled.exists():
        raise UserError("CustomSkinLoader está desactivado. Actívalo en Mods antes de continuar.")
    # Snapshot the small set of files touched by activation before publishing any
    # profile assignment. Download/compatibility errors above cannot mutate it.
    skin_root = game / "CustomSkinLoader"
    paths = [destination, game / "skylauncher-skin-support.json",
             skin_root / "CustomSkinLoader.json", skin_root / "CustomSkinLoader.json.bak",
             skin_root / "SkyLauncher" / "skins" / (p["name"] + ".png"),
             DATA / "profiles.json", DATA / "profiles.json.bak", CONFIG]
    if previous and previous["file"] != destination.name:
        old = mod_path_in(mods_root, previous["file"])
        paths.extend([old, mod_path_in(mods_root, old.name + ".disabled")])
    # Reject symlink ancestors before backup reads as well as before writes.
    for path in paths:
        for component in (path, *path.parents):
            if component.is_symlink():
                raise UserError("La activación de skin contiene un enlace simbólico.")
            if component == DATA:
                break
        if path.exists() and not path.is_file():
            raise UserError("La activación de skin contiene una ruta no válida.")
    with tempfile.TemporaryDirectory(prefix=".skin-activation-", dir=game) as temporary:
        snapshots = {}
        for index, path in enumerate(paths):
            backup = Path(temporary) / str(index) if path.exists() else None
            if backup is not None:
                shutil.copyfile(path, backup)
            snapshots[path] = backup
        try:
            if not destination.exists():
                with destination.open("xb") as writer:
                    writer.write(data)
            if previous and previous["file"] != destination.name and old.exists():
                old.rename(mod_path_in(mods_root, old.name + ".disabled"))
            prepare_local_skin(p, game)
            write_json(game / "skylauncher-skin-support.json",
                       {"version": p["version"], "file": destination.name, "sha512": expected})
            # Publish only once runtime preparation has succeeded.
            write_json(managed_dir() / "profiles.json", store, backup=True)
            cfg = settings({**req, "offline": True, "offline_name": p["name"],
                            "version": p["version"], "local_profile": p["id"]})
        except Exception:
            for path, backup in reversed(list(snapshots.items())):
                if backup is None:
                    path.unlink(missing_ok=True)
                else:
                    shutil.copyfile(backup, path)
            raise
    emit("settings", value=cfg)
    emit_selected(p)
    emit_profiles()
    list_mods(context)
    status("Skin preparada con CustomSkinLoader. Solo será visible en tu cliente; reinicia la partida para verla.")


def dispatch(req):
    global profile, pending
    action = req["action"]
    if action == "init":
        DATA.mkdir(parents=True, exist_ok=True)
        GAME.mkdir(parents=True, exist_ok=True)
        value = {}
        try:
            value = json.loads(CONFIG.read_text(encoding="utf-8"))
            if isinstance(value, dict):
                if (value.get("offline") and not value.get("local_profile")
                        and re.fullmatch(r"[A-Za-z0-9_]{3,16}", value.get("offline_name", ""))
                        and value.get("version") and not profile_store()["profiles"]):
                    p = save_profile(value, legacy=True)
                    value = settings({**value, "local_profile": p["id"]})
                emit("settings", value=value)
            else:
                value = {}
        except (OSError, ValueError):
            pass
        installed()
        emit_profiles()
        if value.get("local_profile"):
            p = find_profile(value["local_profile"])
            if value.get("offline"):
                emit_selected(p)
        list_mods(value)
        if not value.get("offline"):
            restore_session(value.get("client_id", ""))
        dispatch({"action": "versions"})
    elif action == "profile_save":
        p = save_profile(req)
        selected_settings(req, p)
        status("Perfil local guardado: nombre, versión y skin.")
    elif action == "profile_select":
        p = find_profile(req["id"])
        selected_settings(req, p)
        emit_selected(p)
        emit_profiles()
        status("Perfil local seleccionado.")
    elif action == "profile_delete":
        p = find_profile(req["id"])
        store = profile_store()
        store["profiles"] = [x for x in store["profiles"] if x["id"] != p["id"]]
        write_json(managed_dir() / "profiles.json", store, backup=True)
        cfg = settings({**req, "local_profile": "", "offline_name": ""})
        emit("settings", value=cfg)
        emit("local_profile_selected", value={})
        emit_profiles()
        list_mods(cfg)
        status("Perfil eliminado. Sus mundos, mods y skins se conservan en la carpeta de datos.")
    elif action == "skin_catalog":
        catalog(req)
    elif action == "catalog_choose":
        choose_catalog_skin(req)
    elif action == "local_skin_enable":
        enable_local_skin(req)
    elif action == "versions":
        status("Consultando versiones oficiales…")
        emit("versions", versions=[{"id": v["id"], "type": v["type"]}
                                   for v in mc.utils.get_version_list()])
        status("Catálogo actualizado.")
    elif action == "save":
        settings(req)
        status("Ajustes guardados.")
    elif action == "login":
        client = req["client_id"].strip()
        if not client:
            raise ValueError("Introduce el Client ID en Ajustes")
        settings({"client_id": client, "offline": req.get("offline", False)})
        set_session_state(False, cleanup_pending=True)
        session_notice("Completa el acceso en el navegador. La sesión anterior no se restaurará automáticamente.")
        url, state, verifier = mc.microsoft_account.get_secure_login_data(client, REDIRECT)
        pending = (client, state, verifier)
        emit("login_url", url=url)
    elif action == "complete_login":
        if not pending:
            raise ValueError("Inicia un nuevo acceso Microsoft")
        client, state, verifier = pending
        pending = None
        code = mc.microsoft_account.parse_auth_code_url(req["url"].strip(), state)
        set_session_state(False, cleanup_pending=True)
        value = microsoft_call(mc.microsoft_account.complete_login, client, None, REDIRECT, code, verifier)
        accept_online_profile(value, client)
        remember_session(client)
        status("Sesión iniciada.")
    elif action == "logout":
        forget_session()
        status("Sesión Microsoft cerrada en este launcher.")
    elif action == "session_restore":
        client = req.get("client_id", "").strip()
        restored = restore_session(client)
        list_mods(req)
        status("Sesión Microsoft restaurada." if restored else "No se ha restaurado la sesión. Revisa el aviso de Microsoft.")
    elif action == "session_remember":
        renewed = ensure_online_session(req.get("client_id"))
        if not renewed:
            remember_session(profile_client)
        status("Revisa el aviso de sesión Microsoft.")
    elif action in ("install", "play"):
        cfg = settings(req)
        version_dir(cfg["version"])
        identity = launch_identity(cfg) if action == "play" else None
        game = request_game(cfg)
        if action == "play" and cfg.get("offline"):
            if not cfg.get("local_profile"):
                p = save_profile({**req, **cfg})
                cfg = settings({**cfg, "local_profile": p["id"]})
                game = profile_game(p)
            else:
                p = find_profile(cfg["local_profile"])
                # Save the chosen player name/version without replacing the profile's skin.
                path = skin_path(p)
                p = save_profile({**cfg, "skin_path": str(path) if path else "", "variant": p["variant"]})
            if p.get("skin"):
                marker = support_marker(p)
                game = profile_game(p)
                # A saved choice alone is not activation. Keep integrity checks
                # for previously prepared support (including a deleted marker).
                prepared = ((game / "skylauncher-skin-support.json").exists()
                            or (game / "skylauncher-skin-support.json").is_symlink()
                            or (game / "CustomSkinLoader" / "CustomSkinLoader.json").exists())
                if not prepared:
                    status("Skin guardada, sin soporte activado: la partida se iniciará sin la skin local.")
                elif marker.get("version") != cfg["version"]:
                    raise UserError("Prepara la skin en Skins → Guardar y activar para esta versión, o quita la skin del perfil.")
                else:
                    mod = game / "mods" / marker["file"]
                    if hashlib.sha512(mod.read_bytes()).hexdigest() != marker.get("sha512"):
                        raise UserError("El mod de skins ha cambiado. Vuelve a activar su soporte desde Skins.")
                    prepare_local_skin(p, game)
        mc.install.install_minecraft_version(cfg["version"], str(GAME), callback=installation_callbacks())
        installed()
        if action == "install":
            status("Versión instalada.")
            return
        options = {"username": identity["name"], "uuid": identity["id"], "token": identity["access_token"],
                   "jvmArguments": [f"-Xmx{cfg['ram']}G", "-Xms512M"], "launcherName": "SkyLauncher",
                   "launcherVersion": "0.3.0", "gameDirectory": str(game)}
        if cfg["java"].strip():
            options["executablePath"] = os.path.expanduser(cfg["java"].strip())
        command = mc.command.get_minecraft_command(cfg["version"], str(GAME), options)
        status("Minecraft está en ejecución…")
        game.mkdir(parents=True, exist_ok=True)
        with (DATA / "game.log").open("w", encoding="utf-8") as log:
            result = subprocess.Popen(command, cwd=game, stdout=log, stderr=subprocess.STDOUT).wait()
        status("Minecraft cerrado." if result == 0 else f"Minecraft terminó con código {result}. Consulta game.log.")
    elif action == "mods":
        list_mods(req)
        status("Mods actualizados.")
    elif action == "mod_catalog_search":
        mod_catalog_search(req)
    elif action == "mod_catalog_install":
        mod_catalog_install(req)
    elif action in ("mod_import", "mod_toggle", "mod_delete"):
        manage_mod(req)
    elif action in ("fabric_install", "loader_install"):
        cfg = settings(req)
        loader_id = "fabric" if action == "fabric_install" else req.get("loader")
        if loader_id not in LOADERS:
            raise UserError("Loader no válido. Elige Fabric, Forge, NeoForge o Quilt.")
        base, _ = base_version_and_loader(cfg["version"])
        status("Instalando " + LOADERS[loader_id] + " para " + base + "…")
        loader = mc.mod_loader.get_mod_loader(loader_id)
        try:
            version = loader.install(base, str(GAME),
                                     java=os.path.expanduser(cfg["java"].strip()) or None,
                                     callback=installation_callbacks())
        except mc.exceptions.UnsupportedVersion:
            raise UserError(LOADERS[loader_id] + " no admite Minecraft " + base + ". Elige otra versión.") from None
        req["version"] = version
        settings(req)
        if req.get("offline") and req.get("local_profile"):
            p = find_profile(req["local_profile"])
            path = skin_path(p)
            save_profile({**req, "offline_name": p["name"], "skin_path": str(path) if path else "", "variant": p["variant"]})
        installed()
        emit("selected_version", version=version)
        status(LOADERS[loader_id] + " instalado y seleccionado. Añade mods compatibles con esta versión.")
    elif action == "delete":
        if any(p["version"] == req["version"] for p in profile_store()["profiles"]):
            raise UserError("Esta versión está guardada en un perfil local. Cambia ese perfil antes de desinstalarla.")
        path = version_dir(req["version"])
        if path.exists():
            shutil.rmtree(path)
        installed()
        status("Versión desinstalada. Mundos y recursos compartidos conservados.")
    elif action == "skin_upload":
        path = Path(req["path"]).expanduser()
        # El servicio valida el PNG; limita el tamaño antes de leerlo.
        if path.stat().st_size > 2 * 1024 * 1024:
            raise ValueError("El PNG supera 2 MB")
        variant = req["variant"]
        if variant not in ("classic", "slim"):
            raise ValueError("Modelo no válido")
        with path.open("rb") as source:
            skin_request("POST", "/skins", data={"variant": variant}, files={"file": ("skin.png", source, "image/png")})
        status("Skin actualizada en tu cuenta.")
    elif action == "skin_publish":
        publish_skin(req)
    elif action == "skin_reset":
        skin_request("DELETE", "/skins/active")
        status("Skin predeterminada restaurada.")
    elif action == "skin_current":
        skins = skin_request("GET", "").json().get("skins", [])
        active = next((s for s in skins if s.get("state") == "ACTIVE"), None)
        if not active:
            raise ValueError("La cuenta no tiene una skin personalizada activa")
        url = active["url"]
        if not url.startswith(("https://textures.minecraft.net/", "http://textures.minecraft.net/")):
            raise ValueError("Servidor de texturas inesperado")
        with urllib.request.urlopen(url.replace("http://", "https://", 1), timeout=30) as response:
            data = response.read(2 * 1024 * 1024 + 1)
        if len(data) > 2 * 1024 * 1024:
            raise ValueError("Textura demasiado grande")
        path = DATA / "current-skin.png"
        path.write_bytes(data)
        emit("skin", path=str(path), variant=active.get("variant", "CLASSIC").lower())
        status("Skin actual descargada.")
    else:
        raise ValueError("Acción desconocida")


if __name__ == "__main__":
    # Rust sends UTF-8 JSON. Windows may otherwise decode redirected stdin
    # using a regional code page, terminating the process on Unicode paths.
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    for line in sys.stdin:
        try:
            dispatch(json.loads(line))
        except UserError as exc:
            emit("error", text=str(exc))
        except ValueError as exc:
            # Los errores de librerías de autenticación pueden contener respuestas privadas.
            emit("error", text=str(exc) if type(exc) is ValueError and str(exc) in (
                "Introduce el Client ID en Ajustes", "Inicia sesión para jugar", "Inicia sesión primero",
                "Nombre local: usa entre 3 y 16 letras, números o guiones bajos",
                "La cuenta no tiene una skin personalizada activa", "El PNG supera 2 MB") else "Datos no válidos o sesión caducada. Revisa los ajustes e inicia sesión de nuevo.")
        except Exception as exc:
            emit("error", text=f"No se pudo completar: {type(exc).__name__}. Consulta la ayuda del README.")
        finally:
            emit("done")

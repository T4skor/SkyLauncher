# Preparar el proyecto en otro equipo

1. Extrae el ZIP y abre una terminal en la carpeta mi-launcher.
2. Instala Rust/Cargo (https://rustup.rs/) y Python 3.10 o posterior.
3. Crea el entorno Python e instala las dependencias:

   Linux/macOS:
   python3 -m venv .venv
   .venv/bin/python -m pip install -r requirements.txt

   Windows:
   py -m venv .venv
   .venv\Scripts\python.exe -m pip install -r requirements.txt

4. Ejecuta: cargo run --release --locked
   En Windows, si el motor no encuentra Python, configura en PowerShell:
   $env:MI_LAUNCHER_PYTHON = (Resolve-Path .venv\Scripts\python.exe).Path
   Después ejecuta cargo run --release --locked en esa misma terminal.

Consulta README.md para las bibliotecas del sistema en Linux, Java,
autenticación Microsoft y los comandos de comprobación del proyecto.
Se necesita Internet para descargar las dependencias.

Contenido: fuentes Rust/Python, pruebas, Cargo.toml, Cargo.lock,
requirements.txt, .gitignore y documentación.
Los directorios .venv, target y las cachés se regeneran en el nuevo equipo.

Los datos personales del launcher están fuera del proyecto, en
~/.mi-minecraft-launcher/ (en Windows, en la carpeta de usuario).
Si también quieres conservar mundos, mods y configuración, copia esa
carpeta por separado y revisa las rutas de Java en los ajustes.

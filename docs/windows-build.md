# Ejecutable e instalador para Windows

## Estado

`packaging/windows/build-windows.sh` genera tres artefactos reproducibles para
Windows 10/11 x64 a partir del código de `mi-launcher/`:

```text
dist/SkyLauncher-0.3.0-windows-x64.zip
dist/SkyLauncher-0.3.0-windows-x64-setup.exe
dist/windows/SkyLauncher/SkyLauncher.exe
dist/windows/SkyLauncher/python/…
```

El ZIP y el `setup.exe` son las entregas al jugador. La carpeta
`dist/windows/SkyLauncher/` es el árbol de staging y sirve para probar sin
instalar. **No se publica nada automáticamente** y no se firma digitalmente.

## Requisitos de compilación

En un Arch Linux x86_64 actualizado:

```sh
sudo pacman -S --needed rustup curl unzip tar binutils python \
  imagemagick xz
rustup target add x86_64-pc-windows-gnullvm
```

Además, el script descarga y verifica por SHA-256:

- **llvm-mingw 20261104** (UCRT): aporta `x86_64-w64-mingw32-clang`, el enlazador
  y el sysroot de Windows para el target `x86_64-pc-windows-gnullvm`.
- **Python 3.12.10 embeddable amd64**: runtime que viaja junto al ejecutable.
- **NSIS 3.13**: se usa `makensis` nativo si está en `PATH` o `NSIS_DIR`; si no,
  el script extrae los paquetes Debian `nsis` y `nsis-common` en `dist/cache/`
  y ejecuta `makensis` nativo desde ahí. Como último recurso admite un
  `makensis.exe` bajo Wine (lento y con avisos de MESA/EGL en Linux).

Las descargas quedan cacheadas en `dist/cache/` y se verifican antes de usarlas.
No se necesita Wine si hay un `makensis` nativo disponible.

## Construir

Desde la raíz del proyecto:

```sh
./packaging/windows/build-windows.sh
```

El script:

1. Lee la versión de `Cargo.toml` y exige el formato `x.y.z`.
2. Compila `skylauncher.exe` con el target `x86_64-pc-windows-gnullvm`.
3. Descomprime Python 3.12 embeddable en `dist/windows/SkyLauncher/python/`.
4. Instala ruedas `win_amd64` con `pip --platform`, sin ejecutarlas:
   `minecraft-launcher-lib==8.0`, `requests`, `keyring`, `pywin32-ctypes` y las
   dependencias puras de `keyring` (`jaraco.classes`, `jaraco.context`,
   `jaraco.functools`, `more-itertools`).
5. Genera el icono `.ico` desde `packaging/arch/skylauncher.svg` si ImageMagick
   está instalado.
6. Crea el ZIP portable y el instalador NSIS.

`keyring` se instala con `--no-deps` porque pip evalúa los marcadores de entorno
(`sys_platform`) contra el Linux anfitrión y añadiría `SecretStorage`, que solo
sirve en Linux. En Windows el backend usado es `WinVaultKeyring`
(Credential Manager) y su dependencia real es `pywin32-ctypes`.

Variables útiles:

| Variable | Efecto |
|---|---|
| `LLVM_MINGW` | Ruta a un llvm-mingw ya extraído; evita la descarga. |
| `MAKENSIS` | Ruta a un `makensis` nativo concreto. |
| `NSIS_DIR` | Directorio con `usr/bin/makensis` (paquete Debian) o `makensis.exe`. |
| `WINEPREFIX` | Prefijo de Wine para el fallback `makensis.exe`. |

## Qué contiene el bundle

```text
SkyLauncher/
├── SkyLauncher.exe
├── SkyLauncher.ico
├── LEEME-Windows.txt
└── python/
    ├── python.exe
    ├── python312.dll / python312.zip
    ├── Lib/site-packages/…   (dependencias ya instaladas)
    └── …
```

`SkyLauncher.exe` embebe `backend.py` como texto y lo escribe en
`%LOCALAPPDATA%\SkyLauncher\runtime\backend-<hash>.py` antes de lanzar Python.
Se hace así porque Windows limita la línea de comandos a 32767 caracteres y el
motor actual supera ese tamaño. El archivo cacheado se reutiliza mientras el
hash coincida y se puede borrar sin perder datos.

El ejecutable busca `python.exe` junto a sí mismo. No hace falta Python instalado
en el equipo destino, ni permisos de administrador, ni variables de entorno.

## Instalador

`packaging/windows/installer.nsi` usa NSIS con `RequestExecutionLevel user`:

- Instala en `%LOCALAPPDATA%\Programs\SkyLauncher`.
- Crea accesos directos en el escritorio y en el menú Inicio.
- Registra la entrada de **Aplicaciones y características** para el usuario.
- Incluye `Uninstall.exe`.
- El desinstalador borra `%LOCALAPPDATA%\Programs\SkyLauncher`, sus accesos y
  sus claves de registro, pero **no** toca
  `%USERPROFILE%\.mi-minecraft-launcher`: mundos, versiones, mods y ajustes
  sobreviven a la desinstalación y deben borrarse a mano si se desean eliminar.
- El ZIP es portable: descomprimir la carpeta completa y ejecutar
  `SkyLauncher.exe`; la carpeta `python/` debe seguir al lado del `.exe`.

## Verificación

Comprobar el contenido y el motor con el Python embebido. En Linux se puede usar
Wine para una prueba de humo, aunque la validación final debe hacerse en Windows:

```sh
unzip -l dist/SkyLauncher-0.3.0-windows-x64.zip | head
sha256sum dist/SkyLauncher-0.3.0-windows-x64.zip \
  dist/SkyLauncher-0.3.0-windows-x64-setup.exe

cp backend.py dist/cache/test-backend.py
WINEDEBUG=-all WINEPREFIX=/tmp/opencode/wine bash -c '
  printf "%s\n" "{\"action\":\"init\"}" |
  wine dist/windows/SkyLauncher/python/python.exe -u \
    "$(winepath -w dist/cache/test-backend.py)"'
```

La última orden debe imprimir líneas JSON (`installed`, `versions`, …) y
terminar con código 0. Ignora los avisos `MESA-EGL` y `pci id` de Wine.

Pruebas del proyecto antes de empaquetar:

```sh
cargo test --locked
cargo clippy --locked -- -D warnings
.venv/bin/python -m unittest discover -s tests
```

## Limitaciones conocidas

- Los binarios **no están firmados**. Windows SmartScreen puede mostrar un aviso
  la primera vez; es esperable en una distribución sin certificado de code signing.
- Solo Windows x64. No hay build para ARM64 ni 32 bits.
- El instalador no descarga Java, Minecraft ni loaders: eso lo hace el launcher
  en el primer uso.
- El ZIP y el `setup.exe` no se incluyen en el paquete Arch ni en el snapshot de
  fuentes (`dist/` queda excluido); solo se distribuyen como artefactos aparte.

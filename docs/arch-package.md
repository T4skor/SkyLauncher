# Paquete Arch Linux y publicación en AUR

## Estado

El proyecto tiene licencia MIT y una receta para generar un paquete nativo
de Arch Linux x86_64, con ejecutable, entrada de menú e icono.
**SkyLauncher aún no está publicado en AUR.** La receta local no tiene una
URL pública inventada. `yay -S skylauncher` solo funcionará después de publicar
las fuentes y enviar la receta a AUR.

El generador local produce un archivo de fuentes con checksum real, un
`PKGBUILD` preparado y su `.SRCINFO`. No publica nada ni instala paquetes.

## Instalar el paquete local

Preparar las herramientas y la dependencia Python de AUR:

```sh
sudo pacman -S --needed base-devel rust desktop-file-utils
yay -S --needed python-minecraft-launcher-lib
```

Si usas `rustup` en lugar del paquete `rust` de Arch, `makepkg` no reconocerá
la dependencia `cargo`; instala también `cargo` con pacman o compila desde una
salida preparada con el toolchain ya disponible.

Desde la raíz del proyecto:

```sh
python packaging/arch/build-local.py
```

El comando muestra un directorio nuevo bajo `dist/arch-build-…/`. Construye
un paquete `skylauncher-0.3.0-4-x86_64.pkg.tar.zst`, pero no lo instala.
Usa la ruta concreta mostrada para instalarlo:

```sh
sudo pacman -U /ruta/al/skylauncher-0.3.0-4-x86_64.pkg.tar.zst
```

Después aparece **SkyLauncher** en el menú de juegos y se puede ejecutar:

```sh
skylauncher
```

Si se entrega un paquete ya construido, el paso de compilación no hace falta:
instala primero `python-minecraft-launcher-lib` con yay y después el archivo
con pacman. Las dependencias oficiales pendientes se resuelven mediante pacman.
Java es opcional para abrir el launcher, pero puede ser necesario para Minecraft
y los instaladores de loaders. Instala un runtime compatible con la versión de juego.
El selector de archivos requiere un backend de `xdg-desktop-portal` para tu escritorio.

## Archivos instalados

```text
/usr/bin/skylauncher
/usr/share/applications/skylauncher.desktop
/usr/share/icons/hicolor/scalable/apps/skylauncher.svg
/usr/share/licenses/skylauncher/LICENSE
/usr/share/licenses/skylauncher/Tektur-OFL.txt
/usr/share/doc/skylauncher/README.md
/usr/share/doc/skylauncher/loaders-profiles-skins.md
/usr/share/doc/skylauncher/arch-package.md
/usr/share/doc/skylauncher/microsoft-session.md
/usr/share/doc/skylauncher/windows-build.md
```

El motor Python va embebido en el ejecutable. La compilación del paquete fija
`/usr/bin/python` mediante `SKYLAUNCHER_SYSTEM_PYTHON`, en lugar de buscar una
`.venv` en el equipo donde se compiló. `MI_LAUNCHER_PYTHON` sigue disponible como
override explícito. No se ejecuta pip durante la instalación ni al iniciar la aplicación.

Los datos del jugador siguen en `~/.mi-minecraft-launcher/`. No se incluyen
perfiles, credenciales, mundos, entornos virtuales, cachés ni los binarios de Windows
en el archivo de fuentes. Desinstalar el paquete no borra esos datos:

```sh
sudo pacman -Rns skylauncher
```

La licencia MIT corresponde al código del proyecto. Tektur conserva OFL-1.1;
las dependencias y los mods conservan sus propias licencias. El paquete no incluye
Minecraft, Java, skins descargadas ni el JAR de CustomSkinLoader.

La revisión `0.3.0-4` añade la publicación anónima de skins para servidores con
SkinsRestorer, el aviso de override de CustomSkinLoader y el motor Python cacheado
en disco (necesario para el límite de línea de comandos de Windows). Mantiene el
perfil local prioritario en instalaciones nuevas, la cuenta con dos tarjetas
claras y el perfil progresivo. La sesión Microsoft sigue en el llavero del
sistema (`python-keyring` y `python-secretstorage`) y necesita un proveedor
Secret Service desbloqueado para recordarla; no guarda tokens en archivos. Más
información en `docs/microsoft-session.md`, `docs/windows-build.md` y
`docs/product-review.md`.

## Si falla python-minecraft-launcher-lib en yay

Se ha reproducido un fallo de integridad de la receta AUR 8.0-1:
el archivo de Codeberg descargado tiene SHA-256
`30c3bbe6262ed61e45215c7cc883e356db8e71bdf14f176b855d34d864c753f6`,
pero la receta espera
`ff6b5abf8f73168b7bef0bc19ba1125e76a8cc55d66ded712ae834f717e9b61b`.
No se ha determinado por qué cambió ese archivo. No uses `--skipchecksums`
ni aceptes otro hash sin verificar su procedencia.

Se incluye una alternativa local en
`packaging/arch/python-minecraft-launcher-lib/PKGBUILD`. Construye la versión
8.0 desde la distribución de fuentes publicada por el autor en PyPI,
verificando su SHA-256
`4ce173f2e1885c0d5bdf5ecebaa2f1ffc63140663c77699696689f3807b6bdb0`.
No necesita construir la documentación Sphinx; sí instala la biblioteca y su licencia.
El `pkgrel=1.1` identifica esta corrección local y permite una futura actualización
a una receta AUR con `pkgrel=2`.

Si están disponibles los dos paquetes construidos en `dist/`, instala ambos
en una misma transacción desde la raíz del proyecto:

```sh
sudo pacman -U \
  dist/arch-python-minecraft-launcher-lib-8.0-1.1/python-minecraft-launcher-lib-8.0-1.1-any.pkg.tar.zst \
  dist/arch-build-0.3.0-4/skylauncher-0.3.0-4-x86_64.pkg.tar.zst
```

Para reconstruir solo la dependencia:

```sh
sudo pacman -S --needed python-build python-installer python-setuptools python-wheel
cd packaging/arch/python-minecraft-launcher-lib
makepkg -si
```

Esta receta es un workaround local: no modifica la caché de yay ni se ha enviado
al mantenedor de AUR. La desaparición de dependencias de compilación al terminar
yay no es la causa de este fallo de checksum.

## Receta y pruebas

- `packaging/arch/PKGBUILD`: dependencias, compilación bloqueada por Cargo.lock,
  pruebas y archivos instalados.
- `packaging/arch/build-local.py`: archivo de fuentes determinista, SHA-256
  y `.SRCINFO` generado por makepkg, sin sobrescribir directorios existentes.
  Incluye los scripts de empaquetado de Windows, pero no sus binarios.
- `packaging/windows/build-windows.sh`: ejecutable, ZIP e instalador NSIS con
  Python embebido; ver `docs/windows-build.md`.
- `tests/test_packaging.py`: reproducibilidad del archivo de fuentes, exclusión
  de datos privados, checksums, sintaxis Bash y entrada de escritorio.

Preparar sin compilar:

```sh
python packaging/arch/build-local.py --prepare-only
```

Para una ruta concreta, debe ser un directorio que **no exista**:

```sh
python packaging/arch/build-local.py --prepare-only --output-dir /ruta/nueva/de/salida
```

`SOURCE_DATE_EPOCH` permite fijar la fecha del archivo de fuentes; por defecto es
0. Esto hace reproducible el **snapshot de fuentes**, no garantiza un binario
idéntico entre distintas versiones de Rust o herramientas de Arch.

Las opciones de makepkg se pasan después de `--`; si pasas explícitamente
`--install` o `--syncdeps`, makepkg puede solicitar instalar paquetes.
No se recomienda omitir pruebas
ni dependencias para un paquete que se vaya a instalar.

Antes de distribuir una release, revisar:

```sh
makepkg --verifysource
makepkg
namcap PKGBUILD skylauncher-0.3.0-4-x86_64.pkg.tar.zst
```

Ejecuta esos comandos dentro de la salida preparada. `namcap` es una herramienta
de revisión opcional. Para la validación final de AUR se recomienda construir en
un chroot limpio de Arch con las dependencias AUR previamente disponibles.
No se ha instalado ningún paquete del sistema como parte de la preparación.

## Lo que falta para `yay -S skylauncher`

1. Publicar las fuentes en un repositorio público con licencia y una release/tag
   correspondiente a `Cargo.toml` y `pkgver`.
2. Copiar la receta a un directorio separado para AUR. Establecer `url` con la
   URL real del proyecto y cambiar `source` a la URL HTTPS descargable de la release.
   Comprobar que la carpeta superior del archivo coincide con
   `skylauncher-$pkgver`, o adaptar los `cd` de la receta.
3. Regenerar el checksum del archivo público y `.SRCINFO`. No enviar `SKIP` ni
   una receta que dependa de un tarball que solo existe en tu disco:

   ```sh
   updpkgsums
   makepkg --printsrcinfo > .SRCINFO
   makepkg --verifysource
   makepkg
   ```

   `updpkgsums` pertenece a `pacman-contrib`. La línea `SKIP` del modelo local es
   solo un punto de arranque: el generador local ya la sustituye por SHA-256.
   Para AUR debe calcularse el hash del archivo **público**, que puede diferir
   del snapshot local.
4. Crear una cuenta AUR, configurar tu clave SSH y comprobar que el nombre
   `skylauncher` continúa disponible.
5. Enviar **solo la receta y `.SRCINFO`**, no el binario ni el tarball de fuentes:

   ```sh
   git clone ssh://aur@aur.archlinux.org/skylauncher.git
   cd skylauncher
   # Copiar aquí el PKGBUILD ya revisado con las URLs y hashes reales.
   makepkg --printsrcinfo > .SRCINFO
   git add PKGBUILD .SRCINFO
   git commit -m "Initial SkyLauncher package"
   git push
   ```

Solo entonces:

```sh
yay -S skylauncher
```

AUR distribuye recetas mantenidas por la comunidad; no convierte la aplicación en
un paquete oficial de Arch. Las siguientes versiones requieren actualizar
`pkgver`, el checksum público y `.SRCINFO`. La dependencia
`python-minecraft-launcher-lib=8.0` está fijada deliberadamente para coincidir
con la versión contra la que se ha probado el motor.

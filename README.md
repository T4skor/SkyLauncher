# SkyLauncher

**by T4skor** · Tu Minecraft, a tu manera.

Launcher independiente para **Minecraft Java Edition**, con interfaz gráfica en **Rust (egui/eframe)** y motor Python basado en minecraft-launcher-lib. Interfaz en español; no es todavía una implementación íntegramente Rust.

## Instalar en Arch Linux (x86_64)

Hay empaquetado nativo para Arch x86_64 con ejecutable, icono y acceso desde el
menú. **SkyLauncher no está publicado en AUR**: instálalo desde las fuentes de
GitHub con estos comandos (necesitas `yay` para la dependencia Python de AUR):

```bash
sudo pacman -S --needed git base-devel rust desktop-file-utils
yay -S --needed python-minecraft-launcher-lib
git clone https://github.com/T4skor/SkyLauncher.git
cd SkyLauncher
python packaging/arch/build-local.py --output-dir "$PWD/dist/arch-build-local"
sudo pacman -U dist/arch-build-local/skylauncher-0.3.0-4-x86_64.pkg.tar.zst
skylauncher
```

Si ya existe `dist/arch-build-local`, elige otra ruta de salida nueva. Si la
dependencia falla por el checksum de AUR, consulta la alternativa verificada en
**[docs/arch-package.md](docs/arch-package.md)**. No necesita la `.venv` del
proyecto una vez instalado.

## Interfaz Rust

Diseño propio de mundo de bloques: cielo nocturno con estrellas fijas, paneles tipo inventario, navegación en ranuras de hotbar, cubos isométricos y tipografía **Tektur** (OFL, incluida en `assets/`). El tema está centralizado en `src/theme.rs`; las páginas tienen desplazamiento para ventanas pequeñas. Atajos: `Ctrl+1`…`Ctrl+5` cambian de pestaña y `Esc` cierra los diálogos.

- La acción de **Jugar** permanece visible al pie de la ventana y resume la partida: versión, modo, jugador y RAM. Si la versión está instalada, el botón pasa a llamarse **Jugar**; debajo se explica qué falta o qué va a ocurrir.
- La cuenta se elige con dos tarjetas, **Cuenta Microsoft** o **Perfil local**, y el modo local es el predeterminado en una instalación nueva: no necesita Client ID ni cuenta.
- En **Versiones**, el filtro **Solo instaladas** permite encontrar rápidamente los perfiles locales. Cada pestaña conserva su propio desplazamiento.
- Los nombres largos de perfiles, cuentas y mods se recortan sin desplazar los controles; al pasar el ratón se muestra el nombre completo. Las confirmaciones de borrado bloquean la interacción con el fondo.

- **Jugar:** selector de versión, elección de cuenta (Microsoft o perfil local) e instalación y lanzamiento.
- **Versiones:** catálogo oficial, búsqueda, snapshots, versiones instaladas e instalación/desinstalación individual. La desinstalación conserva mundos, bibliotecas y assets compartidos.
- **Skins:** **Mi skin** para importar un PNG con vista de frente y espalda, o **Explorar skins** con el catálogo público de MineSkin. Distingue entre skin guardada y skin activada, e indica el perfil y la versión a los que se aplica. Incluye **Publicar para SkinsRestorer** para obtener una URL de textura y copiar el comando del servidor.
- **Mods:** biblioteca del perfil con búsqueda, filtros, activar/desactivar y eliminar con confirmación, más **Descubrir en Modrinth** filtrado por tu versión y loader. Instalación de **Fabric, Forge, NeoForge y Quilt**, sujeta a la compatibilidad de cada versión.
- **Ajustes:** memoria RAM, ejecutable Java y Client ID, guardados en `~/.mi-minecraft-launcher/rust-config.json`. Comparte el directorio del juego con la interfaz anterior.

### Jugar sin iniciar sesión Microsoft

En una instalación nueva, **Jugar** abre directamente en **Perfil local**. Elige una versión, escribe un nombre (3–16 letras ASCII, números o guiones bajos) y pulsa **Crear perfil**. No necesita Client ID ni conexión para configurarlo. Puedes crear varios perfiles y seleccionarlos en la lista; cada uno conserva nombre, versión y skin.

Los perfiles nuevos tienen sus propios mundos y mods. El jugador local de la configuración anterior se migra automáticamente si existe, conservando su directorio original. Eliminar un perfil no borra sus archivos. Consulta `docs/loaders-profiles-skins.md` para las rutas, los límites y las pruebas.

Para una skin local: abre **Skins → Explorar skins → Cargar catálogo**, elige una o importa tu PNG en **Mi skin** y pulsa **Guardar y activar**. Este botón guarda el perfil, instala CustomSkinLoader automáticamente si tu versión y loader son compatibles, verifica su SHA-512 y configura la skin. Si solo quieres conservarla en el perfil sin instalar el mod, usa **Solo guardar**: la skin queda guardada y podrás seguir jugando sin el soporte activado.

La skin será visible **en tu propio cliente**, sin iniciar sesión Microsoft. No hace que otros jugadores la vean automáticamente ni habilita servicios oficiales. Minecraft vanilla no carga estas skins locales: necesitas un loader y una versión estable compatible de CustomSkinLoader. Si cambias de versión, vuelve a activar el soporte desde **Skins**. **Quitar skin** retira la asignación local; no borra tu PNG original.

Para usar una skin en un servidor con **SkinsRestorer**, abre **Skins → Mi skin → Usar en un servidor con SkinsRestorer** y pulsa **Publicar para SkinsRestorer**. El launcher sube el PNG de forma anónima a MineSkin, obtiene una URL de textura pública en `textures.minecraft.net` y permite copiar `/skin url "<url>" classic|slim`, `/sr createcustom <nombre> "<url>" classic|slim` o `/skin <nombre>`. Cualquiera que conozca la URL podrá descargar la textura: no publiques una skin que quieras mantener privada. El plugin debe estar instalado en el servidor y el jugador necesita `skinsrestorer.command.set.url` o `skinsrestorer.admincommand.createcustom` según el comando. El launcher no instala ni configura el plugin y no comparte skins entre usuarios por sí solo.

Si ese perfil tiene activada una skin local con **CustomSkinLoader**, el launcher avisa en esa misma sección: ese mod puede sustituir la skin que envía el servidor. Para ver la skin de SkinsRestorer, desactiva o quita CustomSkinLoader en **Mods** y vuelve a entrar. Ocultar el aviso de SkinsRestorer con `/skin notification ignore-cracked-client` no desactiva el override del cliente.

Este modo utiliza un UUID offline estable para cada nombre. Cambiar el nombre cambia la identidad del jugador (por ejemplo, el inventario asociado en servidores). Sirve para un jugador y servidores que aceptan jugadores offline; no permite acceder a Realms ni a servidores con autenticación oficial. La gestión de skins oficiales sigue requiriendo Microsoft. La instalación y comprobación de archivos pueden necesitar Internet: «sin Microsoft» no garantiza funcionamiento sin conexión.

Necesitas Rust/Cargo, Python 3.10+ y las dependencias Python:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.txt
cargo run --release
```

En Windows usa `.venv\Scripts\python.exe -m pip install -r requirements.txt`. En Linux se necesita una sesión gráfica con OpenGL y las bibliotecas X11/Wayland; para compilar en Ubuntu/Debian puedes instalar `build-essential pkg-config libx11-dev libxi-dev libxrandr-dev libxcursor-dev libxinerama-dev libgl1-mesa-dev libwayland-dev libxkbcommon-dev`.

Si un `.exe` antiguo muestra «El motor se ha cerrado» al instalar un loader en Windows, recompílalo en Windows con `cargo build --release --locked` y sustituye el anterior. Una versión anterior enviaba el motor completo por `python -c`, superando el límite de longitud de comandos de Windows; ahora se ejecuta desde un archivo en la caché del usuario. El ejecutable sigue necesitando Python con las dependencias de `requirements.txt`; selecciona el intérprete correcto con `MI_LAUNCHER_PYTHON` si es necesario. Forge y NeoForge pueden requerir Java instalado o configurado en Ajustes.

El ejecutable queda en `target/release/skylauncher` (o `.exe`). El motor Python está incluido como texto en el binario, pero sigue necesitando el intérprete y sus dependencias. Usa `MI_LAUNCHER_PYTHON=/ruta/al/python` para seleccionar otro entorno o distribuirlo en otro equipo. Por defecto busca el entorno `.venv` del proyecto donde se compiló y, si no existe, `python3` en PATH. Conservamos la ruta de datos `~/.mi-minecraft-launcher/` para mantener tus mundos y configuración existentes.

Para servicios oficiales, cambia a **Cuenta Microsoft** en **Jugar**, configura el Client ID en **Ajustes → Cuenta Microsoft (avanzado)**, guárdalo y abre **Iniciar sesión en el navegador**. Pega la URL final y pulsa **Completar acceso**. En modo Microsoft, las acciones de cuenta en Skins se aplican a tu cuenta oficial. La vista previa es un personaje frontal 2D, no un modelo 3D.

### Mantener la sesión Microsoft

La aplicación Rust recuerda la sesión mediante el **llavero del sistema** y la
restaura al abrirla en modo Microsoft. Renueva el acceso cuando es necesario.
No guarda tokens en `rust-config.json`, perfiles ni archivos de texto.

En Linux necesita `python-keyring`, `python-secretstorage` y un proveedor
Secret Service, como GNOME Keyring o KWallet, disponible y desbloqueado en la
sesión de escritorio. El paquete Arch declara las dependencias Python; si el
llavero falla, la cuenta funciona solo durante esa ejecución y aparece un aviso.
Tras desbloquearlo, puedes pulsar **Guardar sesión** o **Restaurar sesión guardada**.
**Cerrar sesión y olvidar** desactiva la restauración y elimina la credencial
guardada cuando el llavero está disponible. No cierra la sesión del navegador.

La primera vez tras actualizar tendrás que iniciar sesión de nuevo: las versiones
anteriores no guardaban una credencial que se pueda recuperar. Detalles y pruebas:
[`docs/microsoft-session.md`](docs/microsoft-session.md).

Las operaciones se ejecutan fuera del hilo gráfico. Mientras se instala o está abierto Minecraft, las acciones quedan bloqueadas. Cerrar el launcher termina el motor, pero no cierra un juego ya iniciado. No hay cancelación de descargas.

### Gestor de mods

1. En **Jugar** o **Versiones**, elige una versión oficial compatible con tus mods.
2. En **Mods → Instalar un loader**, elige **Fabric, Forge, NeoForge o Quilt** y pulsa **Instalar…**. Con un loader ya activo, el bloque se resume en **Fabric activo** (o el loader correspondiente) y **Cambiar loader**. Puedes necesitar Java en PATH o configurado en Ajustes. Al terminar, la versión con loader queda seleccionada; si hay un perfil local activo, también se actualiza su versión guardada.
3. Abre **Mods → Descubrir en Modrinth**, busca por nombre o deja la búsqueda vacía para ver opciones populares. El catálogo se filtra por la versión de Minecraft y el loader instalados. Pulsa **Instalar mod**: se descarga la versión estable más reciente; si el autor solo publicó beta o alfa para tu versión de Minecraft, se usa esa y se avisa al terminar. Se comprueban SHA-512 y tamaño, y se añaden automáticamente las dependencias obligatorias.
4. Para un `.jar` que ya tengas, pulsa **Importar .jar**. Se copia a la carpeta de mods del perfil activo y se conserva el original; esta vía manual no comprueba la compatibilidad del archivo. En la instalación original sigue usándose `~/.mi-minecraft-launcher/minecraft/mods/`.
5. En **Mi biblioteca**, usa **Desactivar / Activar** para cambiar entre `.jar` y `.jar.disabled`, y **Eliminar** para borrar la copia tras confirmar. Los archivos existentes no se reemplazan al importar.
6. Inicia la versión con loader desde **Jugar**. Lanzar una versión vanilla no carga los mods.

La biblioteca de cada perfil local nuevo es independiente; la instalación original conserva su biblioteca compartida. El catálogo usa la API pública de Modrinth: solo acepta releases cuyo SHA-512 esté publicado, rechaza URLs y nombres fuera de lo permitido y publica la instalación de forma atómica, con vuelta atrás si algo falla. Las dependencias opcionales no se instalan. CustomSkinLoader se instala mediante su integración específica. Cambiar de versión puede requerir desactivar mods incompatibles. En Linux, los selectores usan el portal del escritorio (`xdg-desktop-portal` y el backend GTK o KDE).

### Comprobaciones

```sh
cargo check --locked
cargo test --locked
cargo clippy --locked -- -D warnings
.venv/bin/python -m unittest discover -s tests -v
```

Las pruebas Rust comprueban las cinco pestañas a 800×600, 1120×820 y 1440×900, nombres largos, vista de skin cargada, visibilidad de la acción de juego, detección de loaders, etiquetas de descargas, validación local, publicación en MineSkin, comandos de SkinsRestorer y atajos. No usan cuentas, red ni datos del jugador. Consulta la evaluación de producto y la comparación con TLauncher y PrismLauncher en `docs/product-review.md`, y las comprobaciones manuales en `docs/frontend-review.md`.

## Interfaz Python anterior

### Ejecutar

Necesitas Python 3.10 o posterior y Tkinter. En Ubuntu/Debian instala `python3-venv` y `python3-tk`.

```sh
cd /home/pablo/Projects/mi-launcher
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
python launcher.py
```

En Windows activa el entorno con `.venv\Scripts\activate`. En macOS utiliza una distribución de Python con Tkinter.

## Configurar el inicio de sesión

El launcher necesita un **Client ID propio** de Microsoft y una cuenta con acceso a Minecraft Java.

1. Registra una aplicación en Microsoft Entra, con soporte para cuentas Microsoft personales.
2. En Autenticación, añade la plataforma de aplicaciones móviles y de escritorio con la URI de redirección `https://login.microsoftonline.com/common/oauth2/nativeclient`.
3. Copia el identificador de aplicación (cliente) en el campo **Microsoft · Client ID**. Este cliente de escritorio usa PKCE, no un secreto de cliente.
4. El acceso de aplicaciones nuevas a los servicios de Minecraft puede necesitar aprobación de Mojang/Microsoft. Consulta la documentación de autenticación de minecraft-launcher-lib: https://minecraft-launcher-lib.readthedocs.io/en/stable/tutorial/microsoft_login.html . Registrar una aplicación por sí solo puede no bastar; un error `AzureAppNotPermitted` indica falta de acceso.
5. Pulsa **Iniciar sesión**, termina en el navegador y pega en el diálogo la URL final completa, que incluye `code` y `state`. Esa URL es temporal y privada. Se verifica el estado de autenticación antes de intercambiar el código.
6. Selecciona versión y RAM, y pulsa **Instalar y jugar**.

La **interfaz Python anterior** conserva la sesión solo en memoria. La aplicación
Rust instalable utiliza el llavero del sistema, como se explica arriba. Cerrar la
sesión en el launcher no cierra la del navegador.

## Archivos y Java

Los datos se guardan en `~/.mi-minecraft-launcher/`: configuración en `config.json`, juego y mundos en `minecraft/`, salida de la última partida en `game.log`. No se guardan tokens en la configuración ni se imprime el comando de lanzamiento.

Deja Java vacío para usar el runtime elegido por la biblioteca. Para versiones antiguas o plataformas sin runtime descargable, instala un Java compatible e indica la ruta de su ejecutable: Java 8 para muchas versiones antiguas, Java 17 para 1.18–1.20.4 y Java 21 para 1.20.5 y posteriores que lo requieran. Asigna RAM dejando memoria disponible para el sistema.

## Ayuda

- `AccountNotOwnMinecraft`: comprueba que tu cuenta tiene acceso a Java Edition.
- `AzureAppNotPermitted`: revisa los permisos/aprobación de tu aplicación.
- `KeyError` o `AssertionError` al autenticar: repite el inicio de sesión y pega la URL final de ese mismo intento.
- Errores de conexión: revisa tu conexión y vuelve a abrir el launcher para cargar las versiones.
- Si el juego se cierra al arrancar: consulta `game.log` y revisa Java y RAM.

La primera instalación necesita conexión y puede ocupar varios GB. El botón permanece ocupado mientras el juego está abierto; puedes cerrar el juego para iniciar otra partida. El launcher no es un producto oficial de Mojang o Microsoft.

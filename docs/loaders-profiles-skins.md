# Loaders, perfiles locales y skins

## Funciones

- Fabric, Forge, NeoForge y Quilt mediante la API de `minecraft-launcher-lib==8.0`.
  Se resuelve la versión base de un loader instalado antes de instalar otro.
  La biblioteca comprueba la compatibilidad y gestiona el instalador correspondiente.
- Perfiles locales persistentes: identificador interno, nombre de jugador, versión,
  textura copiada al almacenamiento del launcher y modelo Classic/Slim.
- Catálogo paginado de MineSkin, con ocho vistas previas por página. La consulta
  y selección no requieren sesión Microsoft. No hay pagos ni scraping.
- Vista previa 2D de frente y espalda basada en las partes del PNG y su capa exterior.
- Skins locales en el juego mediante CustomSkinLoader, únicamente para el propio cliente.
  El mod se descarga al pulsar Guardar y activar; nunca al consultar el catálogo.
- Estado explícito de la skin: guardada en el perfil y, si corresponde, activada para
  la versión y el loader actuales.
- Catálogo de mods de Modrinth filtrado por versión de Minecraft y loader, con
  dependencias obligatorias y comprobación SHA-512.

## Uso

1. En Jugar, la cuenta local viene elegida en instalaciones nuevas. Escribir nombre,
   elegir versión y pulsar Crear perfil.
2. En Mods, elegir un loader en Instalar un loader. El perfil activo recuerda la versión resultante. Cuando ya hay uno activo, el bloque muestra el loader detectado y Cambiar loader.
3. En Skins, importar PNG en Mi skin o abrir Explorar skins y cargar el catálogo.
4. Elegir skin y modelo; Solo guardar conserva una copia en el perfil aunque se borre
   el original y no bloquea Jugar.
5. Guardar y activar guarda el perfil, consulta Modrinth, descarga el mod compatible,
   verifica SHA-512 y escribe la configuración de la skin local. Si la activación falla,
   el perfil y los archivos anteriores quedan como estaban.
6. Lanzar el juego. Tras cambiar de versión, activar de nuevo el soporte.
7. Para usar la skin en un servidor con SkinsRestorer, abrir **Usar en un servidor
   con SkinsRestorer**, pulsar **Publicar para SkinsRestorer** y copiar el comando
   `/skin url` o `/sr createcustom`. La textura publicada es pública; el servidor
   debe tener el plugin y el jugador el permiso correspondiente.

Las skins 64×32 solo admiten Classic. Se aceptan PNG de 64×64 o 64×32, de ocho
bits, sin entrelazado y de hasta 2 MB. Se comprueban dimensiones, CRCs y el flujo
de imagen descomprimido. No se admiten skins HD.

## Datos y compatibilidad con los archivos anteriores

Todo sigue bajo `~/.mi-minecraft-launcher/`:

```text
rust-config.json                     Ajustes y selección de perfil
profiles.json                        Perfiles, esquema 1; sin credenciales
profiles.json.bak                    Copia anterior antes de modificar perfiles
skins/<sha256>.png                    Copias de skins elegidas
skin-catalog/<texture-hash>.png        Caché de texturas públicas
minecraft/                           Versiones, bibliotecas y assets compartidos
instances/<profile-id>/minecraft/    Mundos, mods y configuración del perfil nuevo
game.log                             Registro de la última partida
```

La identidad offline conserva el algoritmo UUID de Java basado en el nombre.
Cambiar el nombre cambia esa identidad. El ID interno del perfil no sustituye al UUID.
Se permiten hasta 100 perfiles. Los perfiles con el mismo nombre pueden usar distintas versiones.

Si la configuración anterior tenía un jugador local válido y no existían perfiles,
se crea uno marcado como legado que utiliza `minecraft/`, sin mover mundos ni mods.
La instalación Microsoft también conserva ese directorio original.
Los perfiles nuevos usan directorios propios y comparten solo los recursos de instalación.
No se copian mundos automáticamente entre perfiles.

Eliminar un perfil borra su registro, **no** sus mundos, mods o skins. Su carpeta
permanece para recuperación manual. No se permite desinstalar una versión todavía
referenciada por un perfil: hay que cambiar o eliminar primero esa referencia.

Las escrituras JSON son atómicas. Un archivo de perfiles dañado o con esquema
desconocido se rechaza sin sobrescribirlo. Los datos gestionados no siguen enlaces simbólicos.
No se guardan tokens Microsoft en perfiles ni se envían al frontend.

## Integraciones y privacidad

**MineSkin**

- API de lectura: `https://api.mineskin.org/v2/skins?size=8`.
- Paginación por cursor `after`; detalle en `/v2/skins/<uuid>`.
- Texturas únicamente desde `https://textures.minecraft.net/texture/<hash>`.
- La selección consulta el modelo real del proveedor; las vistas rápidas del catálogo
  utilizan brazos Classic hasta seleccionar el resultado.
- No se publican perfiles locales, nombres o skins del usuario en MineSkin.
- El catálogo es contenido público de terceros, no curado por SkyLauncher. Los derechos
  siguen siendo de sus autores. La disponibilidad pública no otorga una licencia comercial.
- La API puede limitar consultas, cambiar sus condiciones o dejar de responder.

**SkinsRestorer (publicación para servidores)**

- En **Skins → Mi skin → Usar en un servidor con SkinsRestorer**, el botón
  **Publicar para SkinsRestorer** sube el PNG elegido a
  `https://api.mineskin.org/generate/upload`, sin cuenta ni API key.
- La subida es anónima, pero la URL de textura es **pública** y se sirve desde
  `https://textures.minecraft.net/texture/<hash>` sin autenticación. La interfaz lo
  advierte antes de publicar. Con `visibility=0` la skin no se lista en el catálogo
  de MineSkin, pero quien tenga la URL puede descargarla.
- No se envían perfiles, nombres de jugador ni credenciales. El nombre del skin
  personalizado solo se usa en el comando local, no en la petición a MineSkin.
- La URL devuelta se valida contra el prefijo `https://textures.minecraft.net/texture/`
  y un hash hexadecimal de 32–64 caracteres.
- Comandos generados y copiables:
  - `/skin url "<url>" classic|slim` — requiere `skinsrestorer.command.set.url`.
  - `/sr createcustom <nombre> "<url>" classic|slim` — requiere
    `skinsrestorer.admincommand.createcustom`.
  - `/skin <nombre>` — aplica el skin personalizado creado; usa el permiso de
    cambio de skin del servidor.
- El launcher no instala ni configura SkinsRestorer en el servidor: el plugin debe
  estar presente y el jugador debe tener permiso para ejecutar el comando.
- La publicación no hace que otros usuarios del launcher vean la skin por sí sola;
  la comparten el servidor y su plugin.
- Si el perfil local tiene activada una skin con CustomSkinLoader, ese mod actúa como
  override del cliente y puede ocultar la skin que envía el servidor. La interfaz avisa
  y ofrece abrir Mods; para ver la skin de SkinsRestorer hay que desactivar o quitar
  CustomSkinLoader y volver a entrar. `/skin notification ignore-cracked-client` solo
  oculta un aviso, no desactiva el override.
  Referencia: `https://skinsrestorer.net/docs/troubleshooting/launcher-issues`.
- Si el endpoint anónimo dejara de aceptar subidas, se puede publicar el PNG en
  `https://skinsrestorer.net/upload` y pegar la URL resultante en el campo de textura.

**CustomSkinLoader / Modrinth**

- Proyecto oficial `idMHQ4n2`, consultado por versión de Minecraft y loader.
- Solo releases compatibles sin dependencias externas declaradas.
- Descarga desde `cdn.modrinth.com`, con comprobación SHA-512 antes de instalar.
- Las solicitudes incluyen un User-Agent propio, tiempos límite y tamaños máximos.
  Los descargadores de esta integración rechazan redirecciones y hosts arbitrarios.
- Se preservan las entradas existentes de CustomSkinLoader, añadiendo
  `SkyLauncherLocal` al principio. Classic se traduce al modelo `default` del mod.
- La ruta local usa `{USERNAME}`, no una textura fija para todos los jugadores.
  Al quitar la skin se elimina la entrada gestionada, conservando las demás.
- El soporte se registra por versión y artefacto. Desactivar, borrar o alterar
  el mod impide lanzar un perfil con skin hasta repararlo o quitar la skin.

**Catálogo de mods de Modrinth**

- Búsqueda pública en `api.modrinth.com` filtrada por versión de Minecraft y loader;
  sin loader instalado el catálogo explica qué hace falta antes de consultar.
- Solo se aceptan proyectos de tipo `mod` y archivos `.jar` con nombre seguro.
  Se descartan URLs fuera de `cdn.modrinth.com` y releases sin SHA-512.
- Se prefiere la última versión estable. Si el autor solo publicó beta o alfa para
  esa versión de Minecraft, se usa la más reciente y el estado final lo advierte.
- Las dependencias obligatorias se resuelven de forma recursiva, con versiones fijadas
  cuando la release las declara, detección de ciclos y límites de profundidad y número
  de proyectos. Las dependencias opcionales no se instalan.
- Antes de publicar se comprueban colisiones con la biblioteca existente, se descarga
  a un directorio temporal y se verifica el hash. La instalación es atómica y, si un
  paso falla, se eliminan los archivos nuevos y se restauran los que se habían movido.
- Un `.jar` importado a mano no pasa por esta validación: el launcher lo copia tal cual
  y no garantiza su compatibilidad con la versión o el loader.

Consultar servicios externos revela la IP al proveedor correspondiente. Instalar
un loader o CustomSkinLoader descarga código Java que se ejecutará al iniciar Minecraft.
El bootstrap oficial puede preparar o descargar componentes propios en el primer arranque.
No se distribuye un JAR modificado ni se eluden autenticación o firmas de los servicios oficiales.

Fuentes:

- https://minecraft-launcher-lib.readthedocs.io/
- https://docs.mineskin.org/
- https://skinsrestorer.net/docs/features/skin-upload
- https://modrinth.com/mod/customskinloader
- https://github.com/xfl03/MCCustomSkinLoader

## Pruebas

```sh
.venv/bin/python -m unittest discover -s tests -v
cargo test --locked
cargo clippy --locked -- -D warnings
cargo build --release --locked
```

Se prueban guardado, restauración, migración, aislamiento de mods, preservación de
mundos, datos dañados, PNG inválidos, URLs no permitidas, integridad del JAR,
compatibilidad, selección de versión estable o beta, retirada de skins, el catálogo
y la instalación de mods con sus dependencias, la publicación anónima en MineSkin
y los comandos de SkinsRestorer, la activación segura de skins y layouts de la galería.
Las instalaciones de loaders y el proceso de juego se simulan en las pruebas unitarias.
Resultado de esta entrega: **88 pruebas Python y 26 pruebas Rust aprobadas**,
con Clippy sin avisos. Las consultas reales de versiones de loaders para Minecraft
1.21.1 devolvieron resultados para los cuatro: Forge 52.1.2, NeoForge 21.1.266,
Fabric 0.20.1 y Quilt 0.30.2 como primeras versiones de sus respectivas respuestas.
Son resultados de la consulta, no versiones fijadas en el launcher.

Para QA aislada se puede definir `MI_LAUNCHER_DATA=/ruta/de/pruebas` al ejecutar
el launcher. Por defecto no cambia la ruta existente.

La comprobación de red verifica un catálogo real, la textura elegida y la descarga
oficial de CustomSkinLoader con su hash. Usa metadatos sintéticos únicamente
para consultar compatibilidad en un directorio temporal: **no** demuestra que un
loader esté instalado ni que la skin se haya renderizado dentro de Minecraft.
El arranque real con Java, la visualización dentro del juego, el multijugador y
la autenticación Microsoft requieren una prueba de integración separada.

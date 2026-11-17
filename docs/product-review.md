# Evaluación de producto y claridad de uso

Fecha: 2026-11-15 · Versión evaluada: 0.3.0 (interfaz Rust + motor Python)

> Evaluación histórica. La revisión del 2026-11-17 y sus fuentes verificadas están en
> [ux-review-2026-11.md](ux-review-2026-11.md). La nueva integración Modrinth
> modifica el alcance de mods descrito aquí. Las comparaciones sin fuentes de esta
> evaluación no constituyen pruebas de superioridad frente a otros launchers.

Este documento responde a tres preguntas: en qué punto está SkyLauncher, cómo se
compara con TLauncher y PrismLauncher, y qué se ha cambiado para que la interfaz
sea más clara. No es marketing: incluye lo que falta y lo que no conviene copiar.

## Resumen ejecutivo

SkyLauncher ya tiene una base técnica por encima de la media de los launchers
pequeños: cuatro loaders, perfiles locales aislados, skins con verificación de
hash, sesión Microsoft en el llavero del sistema y paquete Arch reproducible.
El problema no es la funcionalidad, es **el camino hasta la primera partida**.

Tres decisiones lastraban la claridad:

1. **El modo por defecto era Microsoft, pero Microsoft exige un Client ID de
   Azure que el usuario debe crear a mano.** Para un jugador nuevo eso es un
   muro: la primera pantalla pedía algo que no sabe obtener.
2. **La elección de cuenta eran dos chips dentro de una tarjeta llamada
   "Jugador"**, con textos largos alrededor. No se leía como una decisión.
3. **El perfil local aparecía con cuatro controles a la vez** (selector,
   "Nuevo perfil", nombre, guardar) incluso cuando no existía ningún perfil.

En esta revisión se ha invertido el orden: **local primero, Microsoft cuando se
pide**, elección de cuenta como dos tarjetas grandes y el perfil progresivo.
El resultado es que un usuario nuevo puede jugar sin configurar nada más que su
nombre.

## Modelo de cada launcher

| | SkyLauncher | TLauncher | PrismLauncher |
|---|---|---|---|
| Licencia y transparencia | MIT, código y recetas visibles | Propietario, no auditable | GPL-3, auditable |
| Propuesta | Launcher ligero, local-first, empaquetado nativo | Instalación masiva sin cuenta | Gestión avanzada de instancias y mods |
| Cuenta Microsoft | Sí, con **Client ID propio** | Inicio con cuenta Mojang/Microsoft y modo "nickname" | Sí, **Client ID integrado + código de dispositivo** |
| Sin conexión | Sí, por defecto | Sí | Sí, como cuenta separada |
| Instalación de versiones | Sí | Sí | Sí |
| Loaders | Fabric, Forge, NeoForge, Quilt | Forge/Fabric/OptiFine según versión | Fabric, Forge, NeoForge, Quilt, LiteLoader |
| Mods | Importar `.jar` a mano | Gestor propio con catálogo | Catálogo Modrinth/CurseForge dentro de la instancia |
| Modpacks | No | Packs preparados y creación de packs propios según TL MODS | Sí, importación y exportación |
| Skins | Catálogo MineSkin + CustomSkinLoader local | Skins propias y catálogo | Solo skin de la cuenta Microsoft |
| Instancias/perfiles | Perfiles locales con mundos y mods separados | Gestión de packs propios y backups según TL MODS | Instancias completas, duplicables y exportables |
| Java | Manual o el del sistema | Gestionado | Descarga y selección automática por versión |
| Registros y diagnóstico | `game.log`, estado en la barra inferior | Limitado | Visor de logs y informes de fallo |
| Actualizaciones | Manuales (paquete/compilación) | Autoactualización | Comprobación integrada |
| Idiomas | Español | Muchos | Muchos |
| Empaquetado | PKGBUILD Arch + binario | Instalador propio | Flatpak, AppImage, paquetes de distro |

## Dónde gana y dónde pierde SkyLauncher

### Gana

- **Paquete Arch nativo y verificable.** El proyecto incluye su receta de
  empaquetado; esta revisión no verificó todos los formatos de los competidores.
- **Skins locales sin cuenta.** SkyLauncher verifica la integridad de su mod de
  soporte. No se auditó la implementación interna de TLauncher y no se afirma
  que carezca de verificaciones o aislamiento.
- **Sesión Microsoft en el llavero del sistema**, sin tokens en JSON. Es más
  cuidadoso que muchos launchers pequeños.
- **Perfiles locales aislados**: mundos y mods por perfil, sin mover los del
  usuario al migrar. Prism lo resuelve mejor con instancias, pero para un
  launcher ligero es suficiente.
- **Código pequeño y legible**: ~2 200 líneas de Rust y ~1 000 de Python. Se
  puede auditar en una tarde.

### Pierde

- **Primer arranque.** Prism abre, pulsas "añadir cuenta", aceptas en el
  navegador y ya está: su Client ID va dentro. SkyLauncher pedía crear una app
  de Azure. Es la mayor barrera de entrada y la razón principal de esta revisión.
- **Catálogo de mods.** Importar `.jar` a mano no compite con el buscador de
  Modrinth de Prism ni con el catálogo de TLauncher. Un usuario espera buscar
  "Sodium", pulsar instalar y que resuelva dependencias.
- **Modpacks.** No hay importación de `.mrpack`/CurseForge. Es el uso mayoritario
  de Prism.
- **Java.** Los loaders pueden necesitar una versión concreta; hoy se indica que
  hace falta Java, pero no se descarga ni se elige por versión.
- **Instancias.** Los perfiles sirven para jugar, no para duplicar una
  instalación entera con sus mods y configuración.
- **Actualizaciones.** Sin comprobador, el usuario depende de `yay` o de
  recompilar. Prism avisa; TLauncher se actualiza solo.
- **Diagnóstico.** Hay `game.log`, pero no un botón "ver registro" ni un informe
  de fallo legible cuando el juego no arranca.

## Salud técnica

Fortalezas:

- **Separación clara**: Rust dibuja y el motor Python hace red, archivos y
  lanzamiento. El protocolo JSON-lines es fácil de probar y de sustituir.
- **Validación defensiva**: rutas de versiones y perfiles contra traversal y
  symlinks, HTTPS con allowlist, hashes SHA-512 para mods y skins, límites de
  tamaño de PNG, errores de autenticación sin filtrar respuestas privadas.
- **Pruebas reales**: 56 pruebas Python y 16 Rust cubren sesión, perfiles,
  loaders, skins, empaquetado y layout. No hay pruebas de integración con
  Minecraft real, pero la lógica crítica sí está cubierta.
- **Empaquetado reproducible**: snapshot determinista, checksum real, `.SRCINFO`
  generado y pruebas que comparan ambos.

Deuda que conviene atacar:

- **Dos monolitos**: `main.rs` (~2 200 líneas) y `backend.py` (~1 000) mezclan
  dominio, UI y persistencia. Separar por módulos (cuentas, perfiles, skins,
  loaders) reducirá el coste de cada cambio.
- **El motor es síncrono**: una descarga larga bloquea todas las acciones y no
  se puede cancelar. La barra de estado lo comunica, pero el usuario no puede
  parar ni seguir navegando.
- **Timeout por parche**: `minecraft-launcher-lib` 8.0 no acepta timeout en sus
  ayudantes de autenticación; se sustituye su módulo `requests` durante la
  llamada. Funciona y está probado, pero es frágil ante actualizaciones de la
  librería. Lo correcto es que la librería acepte timeout.
- **Dependencia exacta `python-minecraft-launcher-lib=8.0`**: el paquete AUR
  tenía un checksum incorrecto y se resolvió con una receta local. Fijar la
  versión exacta es correcto para reproducibilidad, pero obliga a mantener el
  paquete de dependencia.
- **Sin internacionalización**: todos los textos están en español dentro del
  código. Añadir inglés multiplica la audiencia, pero requiere extraer cadenas.
- **Sin actualizaciones**: no hay comprobador de versión ni canal de paquete.

## Diagnóstico de claridad de la interfaz

Lo que se observó antes de los cambios:

- La tarjeta "Jugador" mezclaba modo, cuenta, perfiles, nombre, skin y avisos de
  sesión. El usuario tenía que leer seis párrafos para decidir algo.
- El botón principal podía quedar lejos de la decisión que lo desbloqueaba.
- "Instalar y jugar" no decía qué se iba a instalar ni con qué identidad.
- El modo local se presentaba como una alternativa menor ("Local / sin
  Microsoft"), aunque es el camino que funciona sin configuración.
- El Client ID aparecía como requisito previo, no como opción avanzada.
- El perfil local mostraba controles de gestión antes de que existiera nada.

## Cambios aplicados en esta revisión

### 1. Local primero

Un arranque nuevo empieza en **perfil local**. No requiere Client ID, cuenta ni
Internet para configurar el jugador. Microsoft sigue disponible y, si hay una
sesión guardada, se puede restaurar desde la propia tarjeta de cuenta.

Por qué: elimina el único requisito que el usuario no puede cumplir sin ayuda.
Quien quiera servidores oficiales cambia a Microsoft en un clic.

### 2. Elección de cuenta como dos tarjetas

Se sustituyen los chips por dos tarjetas de superficie completa con título, una
frase y un indicador de selección (círculo relleno, no solo color):

- **Cuenta Microsoft** — servidores oficiales, skins y logros.
- **Perfil local** — sin cuenta, un jugador y servidores compatibles.

El estado seleccionado se entiende de un vistazo y el área clicable es toda la
tarjeta, no una etiqueta pequeña.

### 3. Perfil progresivo

Sin perfiles guardados solo se muestra el nombre y **Crear perfil**. El selector
y "Nuevo perfil" aparecen cuando ya hay algo que elegir. Con un perfil
seleccionado, el botón pasa a **Guardar cambios** y aparece **Eliminar perfil**.

Por qué: en el primer arranque había cuatro controles para un concepto que
todavía no existía.

### 4. Barra de lanzamiento con identidad

La barra inferior muestra ahora `versión · modo · jugador · RAM`, por ejemplo
`1.21.5 · Local · Alex · 4 GB de RAM`, y debajo el motivo exacto por el que no
se puede jugar o la confirmación de que está listo. Ya no hay que adivinar con
qué cuenta se va a lanzar.

### 5. Textos de acción concretos

- "Activar en el juego" → **Ver en mi partida** (instala el mod si hace falta).
- "Guardar en perfil" → **Solo guardar** (no instala nada).
- "Guardar perfil" → **Crear perfil** o **Guardar cambios** según el estado.
- "Mod loader" → **Instalar un loader**.
- El Client ID pasa a **Cuenta Microsoft (avanzado)** con la explicación de que
  el modo local no necesita configuración.

### 6. Navegación más descubrible

Cada sección de la barra lateral muestra su atajo (`Ctrl+1`…`Ctrl+5`) al pasar
el ratón. El atajo ya existía pero no se anunciaba.

## Hoja de ruta recomendada

Ordenada por impacto en la primera experiencia, no por dificultad técnica.

### P0 — quitar la barrera de Microsoft

1. **Registrar una aplicación pública de Azure y compilar su Client ID** en el
   binario (`option_env!`, sin secreto). Es un cliente público con PKCE; el ID no
   es un secreto. Prism hace exactamente esto.
2. **Inicio por código de dispositivo** en lugar de abrir el navegador y pegar
   una URL. Es el flujo que usan Prism y la propia Microsoft para TVs: muestras
   un código, el usuario lo escribe en `microsoft.com/link`, y el launcher
   consulta el estado. Elimina el paso más frágil del flujo actual.
3. Mantener el Client ID personal como opción avanzada para quien quiera su
   propia aplicación.

Sin esto, SkyLauncher siempre pedirá ayuda para la primera cuenta.

### P1 — completar el bucle de juego

4. **Buscar e instalar mods desde Modrinth** dentro del perfil, con resolución
   de dependencias y hash. Ya se usa Modrinth para CustomSkinLoader; el cliente
   HTTP y la verificación existen.
5. **Botón "Ver registro"** y un resumen legible del último fallo de arranque.
6. **Java por versión**: detectar el Java requerido y ofrecer descarga o
   selección, en lugar de solo avisar.
7. **Instalación en un clic**: elegir versión → instalar loader → jugar, sin
   pasar por tres pestañas.
8. **Cancelar descargas** y permitir navegar mientras se instala; el motor
   síncrono actual bloquea toda la interfaz.

### P2 — alcanzar a Prism en gestión

9. **Instancias duplicables/exportables** (los perfiles actuales son el germen).
10. **Importar modpacks** `.mrpack` y CurseForge.
11. **Comprobador de actualizaciones** y canal de paquete.
12. **Internacionalización** (español/inglés) con textos fuera del código.

### P3 — pulido

13. Copias de seguridad de mundos antes de actualizar.
14. Informe de fallo sin telemetría (el usuario decide qué adjuntar).
15. Documentación de ayuda dentro de la app para loaders y skins.

## Cómo medir si mejora

- **Tiempo hasta la primera partida** desde una instalación limpia.
- **Número de acciones** antes de pulsar Jugar por primera vez.
- **Abandono en la tarjeta de cuenta** (cuántos abren Ajustes y se van).
- **Preguntas de soporte** sobre Client ID, skins o loaders.
- **Porcentaje de partidas lanzadas en local frente a Microsoft**, para saber si
  el modo por defecto es el correcto.

No se propone telemetría automática: estas métricas se pueden obtener de pruebas
de usabilidad y de los informes que el usuario decida compartir.

## Lo que no conviene copiar

- **Cuentas "premium" gratuitas o credenciales de terceros.** TLauncher
  popularizó el acceso sin compra; además de problemas legales, acostumbra al
  usuario a entregar credenciales a software cerrado. SkyLauncher no lo hace.
- **Publicidad y paquetes de regalo en la pantalla de juego.** Aumentan ingresos
  y destruyen la confianza.
- **Autoactualización silenciosa desde un servidor propio.** En un launcher que
  ejecuta Java y descarga JARs, la cadena de suministro importa. Mejor paquete
  firmado o actualización verificada.
- **Catálogo de mods sin verificación de hash.** La velocidad de instalación no
  compensa ejecutar un JAR alterado.

## Limitaciones de esta evaluación

- No se ha probado una partida real, la autenticación Microsoft real ni la
  visibilidad de una skin en el juego; requieren la cuenta del usuario.
- TLauncher y PrismLauncher se comparan por su comportamiento público documentado
  y su interfaz, no por una auditoría de su código o telemetría.
- Los tiempos y el abandono son hipótesis hasta hacer pruebas con usuarios.

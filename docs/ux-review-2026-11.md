# Revisión UX — refinamiento de SkyLauncher

Fecha: 2026-11-17. Estado: IN_REVIEW; integración y aprobación corresponden al director.
Método: revisión de diseño y comportamiento mediante código/documentación, aplicación nativa egui; detector DOM no aplicable.

## Alcance y límites de evidencia

Se consultaron primero `docs/frontend-review.md`, `docs/product-review.md`,
`docs/loaders-profiles-skins.md`, después `docs/microsoft-session.md`, README,
`src/main.rs`, `src/theme.rs` y las rutas relevantes de `backend.py`.
No se encontró `docs/game-studio/` en el workspace ni PRODUCT.md/DESIGN.md en la búsqueda.
No se ejecutaron aplicación, pruebas ni Minecraft; no se inspeccionó un render actual.
Los riesgos visuales requieren comprobación gráfica, no constituyen fallos visuales medidos.
La utilidad de contexto de la skill no se ejecutó: no había herramienta de shell disponible.

**Evidencia** = código leído o contenido publicado en las URLs citadas.
**Opinión/propuesta** = interpretación UX y comportamiento recomendado, todavía no implementado.
Las fuentes de TLauncher son declaraciones del proveedor, no una auditoría independiente de su binario.

## Dictamen

**Conservar cielo nocturno, estrellas fijas, cubos, paneles, Tektur y verde de acción.**
No hace falta cambiar de identidad ni copiar la apariencia de TLauncher.
La oportunidad es convertir funciones técnicas en recorridos completos y recuperables.

Fortalezas verificables en código:
- Tektur está separada del texto proporcional de cuerpo y botones (`theme.rs:26–101`).
- Barra de lanzamiento persistente, validación de nombre y perfiles progresivos ya existen.
- Estados de mods llevan texto, no solo color; botones personalizados tienen borde de foco
  y `WidgetInfo` (`theme.rs:370–380`). No equivale a certificar lector de pantalla.
- Skins locales y oficiales distinguen su alcance en el texto; conservar esa honestidad.

## Comparación factual con TLauncher

Fuentes oficiales consultadas el 2026-11-17:

1. https://tlauncher.org/en/install-skins.html
   Documenta registro en su web, subida o catálogo, login en el launcher y elección de
   una versión con icono TL. Indica que ese icono representa soporte de su sistema.
   La guía reserva capas y skins HD a Premium.
2. https://tlauncher.org/en/news_26/modpack-system_5100.html
   Artículo fechado 2018-07-17, todavía enlazado desde la ayuda actual. Documenta TL MODS,
   packs preparados, creación de packs propios, descripciones, imágenes, versiones,
   categorías, instalación de dependencias API, activación/desactivación, importación
   manual y backup/restauración. No prueba cobertura universal ni funcionamiento actual
   de cada versión.
3. https://tlauncher.org/en/
   Publicita catálogo y comprobación de compatibilidad, instalación Forge/Fabric/OptiFine
   y skins visibles entre usuarios de su sistema. El pie declara que contiene anuncios.
   No tomar sus superlativos ni promesas de seguridad como resultados de auditoría.
4. https://tlauncher.org/en/help.html
   Enlaza guías de skins, mods, Fabric, NeoForge, backups y diagnóstico.
   Que exista una guía NeoForge no demuestra instalación integrada equivalente a los otros loaders.

| Recorrido | TLauncher documenta | SkyLauncher observado | Implicación UX |
|---|---|---|---|
| Skin personalizada | Cuenta propia + web + versión TL | PNG/MineSkin + perfil local + loader/CustomSkinLoader; o cuenta Microsoft | Menos necesidad de cuentas para lo local, pero no prometer la misma visibilidad multijugador |
| Saber si funcionará | Indicador TL en versiones | Aviso de bloqueo posterior y texto de ayuda | Mostrar estado de soporte antes de aplicar |
| Encontrar mods | Catálogo, categorías, detalles | Búsqueda únicamente en JARs instalados | Hacer inequívoco “Buscar en instalados”; catálogo real es trabajo funcional |
| Dependencias | Instalación automática de APIs según su guía | Importación manual; integración específica para CustomSkinLoader | No presentar un JAR importado como compatibilidad comprobada |
| Packs | Crear, instalar y respaldar packs | Perfiles aislados; sin catálogo/importación general de packs | Mantener aislamiento; añadir packs en una fase distinta |

### Correcciones necesarias a la comparación anterior

`docs/product-review.md:33–58` no debe reutilizarse como tabla factual sin revisión:
- “Una instalación principal” y “modpacks curados” son incompletos frente a la creación
  de packs propios documentada por TL MODS.
- No hay evidencia en las fuentes consultadas para afirmar que TLauncher carece de
  hashes o aislamiento por perfil. Ausencia de documentación no demuestra ausencia.
- Tampoco se verificaron aquí sus afirmaciones sobre Prism, licencia, empaquetado,
  Java, actualización o seguridad comparativa. No repetirlas como conclusiones de esta auditoría.
- No equiparar skin local del propio cliente con el sistema compartido de TLauncher.
- Evitar afirmaciones jurídicas, de malware o superioridad de seguridad sin evidencia independiente.

## Recorridos del jugador y prioridades

### 1. P1 — Guardar una skin no debe impedir jugar

**Recorrido actual:** crear perfil → elegir PNG → Solo guardar → Jugar.
**Evidencia:** `main.rs:318–325` bloquea un perfil con skin si falta soporte para la
versión; `main.rs:1285–1297` ofrece Solo guardar y dice que sin loader el juego no
la mostrará. `backend.py:756–771` guarda el perfil antes de comprobar loader y
disponibilidad del mod. Una activación fallida puede dejar cambios persistidos.
**Fricción:** una opción presentada como no invasiva introduce un bloqueo inesperado;
el error puede modificar el perfil aunque la acción parezca haber fracasado.

**Propuesta para systems-programmer (responsable):**
- Separar “skin guardada” de “skin activada para esta versión”.
- Guardar conserva la elección sin exigir soporte ni impedir jugar.
- Antes de activar, comprobar requisitos y presentar destino + descarga necesaria;
  persistir activación solo al completar la operación.
- Si falla, conservar la skin anterior y el borrador elegido; mostrar Reintentar.
- Si se rompe un soporte ya activado, mantener controles de integridad y ofrecer
  Reparar o Desactivar skin local, sin borrar el PNG guardado.
- No quitar simplemente la comprobación de integridad para ocultar el problema.

**Validación:** guardar PNG en vanilla permite jugar sin skin; fallo de red, ausencia
de release o hash erróneo no cambia el estado activo; reparar no modifica otros perfiles.
Un usuario nuevo debe explicar la diferencia guardar/activar sin consultar README.

### 2. P1 — Requisitos de skin visibles antes de actuar

**Recorrido actual:** Skins → Ver en mi partida → error → Mods → instalar loader → Skins.
**Evidencia:** el botón solo comprueba nombre, versión y ruta no vacía
(`main.rs:1280–1283`), no el loader. La compatibilidad se descubre en backend.

**Propuesta para gameplay-programmer (responsable; depende del estado del backend):**
- Mostrar junto a la previsualización: “Perfil X · Minecraft Y · loader Z”.
- Estados textuales: Sin skin / Cambios sin guardar / Guardada, no activada /
  Soporte necesario / Comprobando / Activa para esta versión / Error recuperable.
- Sin loader, acción “Preparar soporte de skins” que explica qué se instalará,
  permite elegir una opción compatible y vuelve a Skins. No cambiar versión ni
  loader silenciosamente; no recomendar Fabric universalmente sin comprobar disponibilidad.
- Mantener el aviso visible “Solo la verás en este cliente” junto a la acción local.
- Microsoft: identificar la cuenta destino y decir “Cambiar skin de [cuenta]”;
  ausencia de sesión tiene enlace directo al acceso, no un botón muerto.

**Validación:** en vanilla, loader compatible, versión sin release y sesión ausente,
el usuario identifica qué falta y qué ocurrirá antes de pulsar. Cambiar versión
invalida el estado de soporte mostrado y nunca conserva un falso “Activa”.

### 3. P1 — Mods: aclarar alcance y no prometer compatibilidad inexistente

**Recorrido actual:** perfil → versión → Mods → loader → importar JAR →
buscar dependencias fuera de la app → iniciar.
**Evidencia:** `main.rs:1334–1469` dice “mods compatibles”, muestra “Actualizar”
para refrescar la lista y pide importar dependencias por separado.
`backend.py:125–150` lista nombres/estado/tamaño y comprueba ZIP/JAR al importar,
pero no verifica compatibilidad del mod con Minecraft o loader.

**Propuesta inmediata para gameplay-programmer (responsable):**
- Cabecera explícita: “Mods de [perfil] · Minecraft [base] · [loader/sin loader]”.
- Cambiar “Actualizar” por “Recargar lista”: no actualiza mods.
- “Instalados” y “Buscar en instalados” separan biblioteca de catálogo.
- Sustituir promesa de compatibilidad por “Gestiona los mods de este perfil”.
- JAR manual: estado “Compatibilidad no comprobada”; “Activo” significa habilitado,
  no verificado ni cargado con éxito. Aviso visible si la versión es vanilla.
- Estado vacío contextual: Crear perfil si falta; Preparar loader si falta;
  Importar .jar cuando ya está listo. Conservar importación manual para expertos.
- Confirmación breve: “Importado en [perfil]. Se aplicará al próximo inicio”.

**Fase funcional posterior, systems-programmer responsable:**
catálogo Modrinth filtrado por versión+loader; buscar → detalle/origen →
plan de instalación con dependencias obligatorias y tamaño si se conoce →
confirmar → descargar/verificar → instalado. No mostrar “Compatible” antes de
resolver metadatos; separar “sin resultados compatibles” de fallo de red.
Cambiar perfil durante una operación no debe cambiar su destino. No instalar
dependencias incompatibles ni dejar una instalación parcial presentada como éxito.
No dibujar buscador remoto o instalar ficticio antes de implementar contrato y backend.

**Validación:** el usuario distingue biblioteca de catálogo y recarga de actualización;
dos perfiles reciben únicamente sus propios JARs. Para el futuro catálogo,
probar dependencia ausente/conflictiva, disco lleno, red fallida y cambio de perfil.

### 4. P2 — Galería de skins recuperable y sin lenguaje comercial innecesario

**Evidencia:** `main.rs:447–523` solo tiene actualizar/siguiente; elegir cierra la
galería antes de recibir el resultado. Las miniaturas se dibujan Classic hasta
consultar detalle. “Tienda” no implica pagos reales.

**Propuesta para gameplay-programmer (responsable):**
- Renombrar “Tienda de skins” a “Catálogo de skins”, sin cambiar su aspecto.
- Añadir Anterior mediante historial local de páginas/cursors; indicar página
  visitada sin inventar un total que la API no entrega.
- No cerrar la galería hasta completar selección; mantener resultados y selección
  ante error, con Reintentar junto al mensaje.
- Mantener primera carga explícita y aviso breve de proveedor/red; no introducir
  consultas automáticas silenciosas como supuesto “pulido”.
- Mostrar procedencia y miniatura seleccionada; modelo Classic/Slim descrito
  como “brazos anchos/estrechos”. Si PNG es 64×32, desactivar Slim con motivo.
- Búsqueda/categorías solo si la API las soporta: no prometer búsqueda global
  con un filtro limitado a las ocho skins de la página.

**Validación:** avanzar dos páginas y volver recupera la página anterior; fallo al
elegir no pierde contexto; nadie interpreta catálogo como compra ni miniatura como
prueba de que la skin ya está activada.

### 5. P2 — Legibilidad y accesibilidad sin cambiar identidad

**Evidencia:** `theme.rs:236–248` expone ayudas con `Sense::hover`; información de
bloques puede quedar únicamente en tooltip. `theme.rs:388–406` encoge texto de
botones hasta 11 px para hacerlo caber. Son riesgos, no resultados de prueba visual.

**Propuesta para gameplay-programmer; ui-artist revisa composición:**
- Mantener Tektur en títulos; cuerpo 15 y ayudas 13 como base existente.
- No resolver nuevas etiquetas largas reduciendo fuente: ampliar/envolver filas o
  usar redacción corta. Mantener acción local junto a vista previa.
- Ayuda esencial visible; ayuda extensa mediante control enfocable, activable
  con teclado y cerrable con Esc. Tooltips no son su único canal.
- Conservar verde principal, oro de foco, texto de estado y bordes de selección.
  No añadir nuevas gamas, ilustraciones, grandes héroes ni tarjetas estadísticas.
- Verificar nombre accesible de campos, cuenta seleccionada, cambios de estado y
  preview; WidgetInfo en botones no resuelve toda la semántica.

**Validación gráfica pendiente:** 800×600, 1120×820 y 1440×900, escala 100/150/200%;
Tab/Shift+Tab/Enter/Espacio/Esc; contraste de texto normal ≥4.5:1 y controles/foco
relevantes ≥3:1 en su contexto real. No clipping, acciones fuera de pantalla ni
información esencial accesible únicamente con ratón. Probar lector de pantalla
del entorno objetivo antes de declarar accesibilidad soportada.

## Secuencia sugerida y pruebas de comprensión

1. Resolver modelo guardada/activada y persistencia ante fallo (backend).
2. Estados de skin + requisitos/destino y claridad de biblioteca de mods (frontend).
3. Navegación de catálogo + ayudas accesibles + ajuste de etiquetas (frontend).
4. Catálogo de mods/dependencias como tarea funcional independiente.
5. Modpacks/importación/exportación después; 3D de skins no es requisito para
   competir en claridad. La vista 2D actual es suficiente para esta iteración.

Prueba moderada propuesta con cinco usuarios nuevos, sin telemetría automática:
- Preparar perfil y nombrar versión/loader/destino antes de importar un mod.
- Guardar skin sin activar; después activarla y explicar quién la verá.
- Recuperarse de selección de skin fallida y de soporte incompatible.
- Volver a una skin de una página anterior.
- Repetir tareas clave con teclado y con texto ampliado.
Objetivo inicial (no resultado obtenido): 4/5 completan cada recorrido sin ayuda
y explican correctamente su alcance; cero cambios en el perfil equivocado.
Registrar pasos, dudas y errores; separar tiempo de interacción del de descarga.

Novato: riesgo principal, guardar bloquea jugar y loader aparece tarde.
Usuario experto: faltan retorno de página, catálogo de mods y operaciones recuperables.
Usuario de teclado/baja visión: ayuda hover y texto encogido requieren validación prioritaria.

## Entrega y coordinación

- **Hecho:** auditoría estática de recorridos, contraste factual con fuentes oficiales
  y especificación priorizada con criterios de aceptación.
- **Archivo creado:** `docs/ux-review-2026-11.md`. Ningún código ni documento previo editado.
- **Decisiones:** refinamiento conservador, verdad de estados antes que decoración,
  separar arreglos pequeños de nuevas integraciones; no presentar opinión como medición.
- **Dependencias:** systems-programmer para estado/compatibilidad/transacciones;
  gameplay-programmer para comportamiento egui; ui-artist para composición;
  QA para regresión y director para asignación/integración.
- **Pendiente:** ejecución gráfica, pruebas automatizadas y de usuario, integración
  Minecraft, accesibilidad real y corrección de tabla anterior por su responsable.
- **Cómo probar:** criterios por hallazgo y matriz anterior usando un
  `MI_LAUNCHER_DATA` de pruebas, nunca mundos ni credenciales personales.

Nota de investigación: websearch falló con HTTP 402; las fuentes se obtuvieron
mediante webfetch y enlaces de la ayuda oficial. Los intentos iniciales
`/en/install-skin.html` y `/en/mods.html` devolvieron 404; no son fuentes del informe.

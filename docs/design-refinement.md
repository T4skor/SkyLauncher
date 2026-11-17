# Refinamiento del tema — 2026-11-17

Estado: IN_REVIEW. Alcance: `src/theme.rs`; sin cambios en `main.rs`,
`backend.py`, navegación funcional, protocolo o datos del jugador.

## Decisiones y guía visual

- Se conserva la identidad nocturna, las siluetas de bloque, Tektur en marca
  y títulos, y el verde de la acción principal. Modo de uso: herramienta.
- Navegación en proporcional de 15 puntos, botones de 14, descripciones de
  tarjetas de 13 e insignias de 12. Las tarjetas calculan su altura según
  título y descripción: no recortan instrucciones por una altura fija.
- Las etiquetas largas de botones mantienen 14 puntos y usan elipsis,
  clipping local y tooltip completo; no se reducen a 11 puntos.
- Paneles `#171F33`, superficie elevada `#1F2942`, texto `#E9EFF7`,
  secundario `#AAB7CC`. Borde interactivo `#70819D`, separado del borde
  discreto de panel `#33405C`.
- Verde: selección/acción. Dorado: foco. Rojo: peligro. El borde dorado de
  2 puntos también alcanza los controles nativos en estado activo/foco.
  Se conservan los anillos de foco de los controles personalizados.
- Selección de chips: subrayado además del color; su galley usa color
  placeholder para que egui aplique realmente el color del estado.
- Se conservan ranuras exteriores de navegación y cubos de marca; se retiran
  marcos redundantes alrededor de iconos en navegación/tarjetas y se reduce
  a la mitad la opacidad de estrellas. Los controles prevalecen sobre el fondo.
- Se mantienen alturas de botones (mínimo 38; habitual 44 puntos), navegación
  de 46, escala de espacios y radios pequeños del tema.

## Entrega y pipeline

Archivo importable: módulo Rust existente, mismas firmas públicas y dependencias.
Arte vectorial generado por egui; sin nuevos atlas, texturas o fuentes.
Unidades en puntos lógicos: escalado por egui, sin slicing ni resolución fija.
El atlas de skins y su sampling no cambian.

## Verificación y límites

Lectura de README, revisión previa, tema completo y reglas craft-floor.
Compatibilidad de `LayoutJob::simple`, wrapping y selección de visuales de foco
contrastada con fuentes locales de egui/epaint 0.31.1.
Contrastes WCAG calculados a partir de RGB opacos:

| Par | Relación |
|---|---:|
| Secundario / panel | 8.09:1 |
| Secundario / superficie elevada | 7.12:1 |
| Borde interactivo / superficie elevada | 3.66:1 |
| Foco dorado / panel | 9.77:1 |
| Texto oscuro / botón verde | 9.11:1 |

No había herramienta de terminal ni captura de escritorio disponible en esta
sesión: **no se ejecutaron Cargo, rustfmt ni revisión visual del render**.
Estos cálculos no equivalen a una auditoría completa de accesibilidad.

## Pendiente / cómo probar

El integrador debe formatear solo el módulo (`rustfmt --edition 2021 src/theme.rs`;
no ejecutar `cargo fmt` global mientras otros agentes editan `main.rs`), y ejecutar
`cargo check --locked`, `cargo test --locked`, `cargo clippy --locked -- -D warnings`.

Revisión UX pendiente: cinco pestañas a 800×600, 1120×820 y 1440×900, nombres
largos de mods/perfiles, Skins con/sin textura y tarjetas de cuenta con texto
envuelto. Recorrer con Tab y Shift+Tab; comprobar selección verde frente a foco
dorado, activación y tooltip completo de etiquetas truncadas. Revisar estados
deshabilitados y confirmar que la barra de Jugar sigue visible.

Revisión tech-art pendiente: contraste y clipping con escala de escritorio
100/150/200 %. No se necesitan cambios de pipeline.
Ayudas accesibles por teclado y mejoras de flujo de Skins/Mods corresponden
al responsable de `main.rs`; este refinamiento no añade comportamientos nuevos.

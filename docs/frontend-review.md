# Revisión del frontend de SkyLauncher

## Alcance y criterio

Mejora de la interfaz existente de Rust/egui. Se mantiene su identidad: cielo nocturno,
bloques isométricos, Tektur para títulos y acento verde para la acción principal.
Es una herramienta para preparar y lanzar Minecraft, no una landing comercial.
No se modifica el backend, el protocolo, los archivos del jugador ni la interfaz Python anterior.

## Diagnóstico

- La cabecera ocupaba 198 px y repetía información que ya estaba en los controles.
  Los textos se pintaban en coordenadas fijas y podían solaparse con perfiles largos.
- La acción principal quedaba dentro del contenido desplazable: en ventanas bajas
  era necesario buscarla. El estado deshabilitado daba una explicación poco específica.
- La tipografía de exhibición se usaba también en botones largos, dificultando su lectura.
  La navegación daba un peso visual parecido a todas las opciones.
- Las filas con nombres largos podían desplazar sus acciones. El campo de skin
  desbordaba el ancho mínimo de ventana.
- Versiones tenía dos regiones de desplazamiento anidadas y no permitía aislar instalaciones.
- Las confirmaciones de eliminación no bloqueaban el fondo y faltaba foco visible en la navegación.

## Cambios

- Cabecera compacta con composición de egui y texto que puede ajustarse al ancho.
  Se elimina el eslogan y la información duplicada.
- Barra fija de lanzamiento con versión, memoria, disponibilidad y acción contextual.
  Se conservan las comprobaciones y el comando de instalación/lanzamiento.
- Cuerpo y ayudas más legibles, botones en tipografía proporcional y navegación más tranquila.
  El foco de teclado se distingue de la selección de pestaña.
- Acceso directo a Ajustes cuando falta el Client ID, sin intentar autenticar una configuración vacía.
- Filtro de instalaciones, contador de resultados y una sola región de desplazamiento en Versiones.
- Nombres largos truncados con texto completo en tooltip; filas de mods y herramientas con ajuste.
- Confirmaciones modales para eliminar mods o versiones.
- Estados de error coherentes al fallar la comunicación o la previsualización de una skin.

## Archivos y dependencias

Implementación: `src/main.rs` y `src/theme.rs`. Documentación: `README.md` y este archivo.
No se añaden dependencias, fuentes, descargas de recursos ni requisitos de ejecución.
Los tests de interfaz usan el Python ya requerido por el launcher como proceso inerte,
sin ejecutar el backend ni escribir configuración.

## Cómo probar

```sh
cargo test --locked
cargo clippy --locked -- -D warnings
.venv/bin/python -m unittest discover -s tests -v
cargo run --locked
```

Comprobación manual:

1. Reducir la ventana a 800×600. Confirmar que Jugar permanece visible al desplazar el contenido.
2. Sin sesión Microsoft, comprobar la explicación del bloqueo y el acceso a Configurar Microsoft.
3. Elegir modo local: un nombre inválido debe impedir jugar; uno válido debe permitirlo
   cuando exista una versión y el backend no esté ocupado. No hace falta lanzar el juego para verificar el estado.
4. Probar búsquedas con y sin resultados y Solo instaladas; seleccionar un perfil Fabric.
5. Probar Skins con textura cargada y sin textura, y un nombre largo de mod.
6. Recorrer controles con Tab, activar con teclado y cambiar pestañas con Ctrl+1…Ctrl+5.
7. Abrir una confirmación de borrado sin aceptarla: el fondo no debe responder.
   Cancelar con Esc. No usar archivos reales del jugador para probar eliminaciones.

## Límites de validación

Validado entonces: 7 pruebas Rust, 12 pruebas Python, Clippy sin avisos y compilación release.
Revisión visual de la aplicación ejecutada a 1120×820 y 800×600; los tests de layout
añaden cobertura a 1440×900. El detector mecánico no emitió hallazgos sobre los archivos Rust;
su cobertura de una interfaz nativa no equivale a una auditoría de accesibilidad.

Las pruebas de layout no sustituyen una revisión con lector de pantalla.
La autenticación real de Microsoft, las descargas, el arranque de Minecraft y la aplicación
de skins a una cuenta requieren validación de integración con credenciales y red.
No se añaden ni simulan esas capacidades.

## Revisión de claridad (2026-11-15)

La evaluación completa y la comparación con TLauncher y PrismLauncher están en
[`product-review.md`](product-review.md).

Problema: el primer arranque pedía un Client ID de Microsoft —algo que un jugador
nuevo no puede obtener sin ayuda— y la tarjeta "Jugador" mezclaba modo, cuenta,
perfiles, nombre, skin y avisos de sesión.

Cambios:

- Instalación nueva en **perfil local**; Microsoft es una elección explícita.
- Dos tarjetas de cuenta con superficie completa, título, descripción y estado
  seleccionado que no depende solo del color.
- Perfil progresivo: sin perfiles solo se pide el nombre y **Crear perfil**; el
  selector aparece cuando hay algo que elegir.
- La barra de lanzamiento resume `versión · modo · jugador · RAM` y mantiene el
  motivo del bloqueo o la confirmación de disponibilidad.
- Acciones renombradas: **Ver en mi partida**, **Solo guardar**, **Crear perfil**,
  **Guardar cambios**, **Instalar un loader**, **Cuenta Microsoft (avanzado)**.
- Atajos de navegación visibles en tooltip.

Pruebas nuevas: las tarjetas de cuenta y el resumen de lanzamiento aparecen a
800×600; sin perfiles no se muestra el selector ni "Guardar cambios"; con un
perfil se muestra el selector, "Guardar cambios" y "Eliminar perfil".

Validado: 16 pruebas Rust, 56 pruebas Python, Clippy sin avisos. No se ha probado
visualmente esta revisión en una sesión gráfica; los tests comprueban límites de
layout, no el render final. Queda pendiente una captura y una prueba con usuario
nuevo.

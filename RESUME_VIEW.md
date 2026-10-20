> Corrección visual aprobada como dirección: ambas vistas comparten habitación lofi completa y paneles oscuros cálidos (véase DESIGN_SYSTEM.md). Resume tiene un acceso enfocable a EmotionalUI además de L/R. Las microestadísticas incorporan fecha de última sesión cuando existe una fecha válida; la demostración se rotula. El lanzamiento real continúa pendiente.

# Resume y Emotional

El inicio integrado es el predeterminado (`--concept integrated`). Los cuatro modos anteriores siguen accesibles: `legacy`, `quiet`, `scene`, `editorial`. No se han eliminado pruebas ni composiciones anteriores.

Si existe una sesión propia cuyo juego sigue disponible, arranca Resume. Sin historial arranca Emotional. `--view resume` permite comprobar el vacío útil. `--demo-history` añade únicamente un registro ficticio **en memoria**, rotulado «Una partida de muestra»; no lo guarda en SQLite.

Resume presenta nombre, arte propio en catálogo de demostración, Continuar, biblioteca, microestadísticas opcionales y acceso L/R a descubrir. En importados se omite la ilustración ficticia: la carga de carátulas locales autorizadas sigue pendiente. No compiten recomendaciones con el juego reciente.

Las sesiones (`activity`), finalización explícita (`progress`), elecciones (`preferences`/`habits`), editorial y catálogo son entidades distintas. Solo `--mark-completed ID` para un juego disponible marca finalización. No se deduce del tiempo ni playcount. La línea omite contadores vacíos, última fecha relativa y duración antes que afirmar datos dudosos. Se puede ocultar desde Start.

## Interacción

L/R (teclado Q/E) alterna las vistas; cada una conserva foco. Cruceta mueve entre acciones disponibles. Biblioteca queda inmediatamente después de la acción principal; la alternativa discreta ocupa la siguiente posición. A confirma; B cierra el panel y restaura su contexto; Start abre opciones. Select accede directamente a biblioteca. Las asignaciones evdev se conservan sin cambios en dArkOS.

Los cambios lógicos son síncronos y las animaciones finitas/interrumpibles. Movimiento reducido elimina el desplazamiento y los fundidos. El foco tiene un marcador sólido y subrayado, no depende de una animación.

## Límite del prototipo

Continuar dispara confirmación y explica que falta un emulador configurado. **No abre todavía un juego real**. `services::launcher::Plan` prepara ejecución argv sin shell, valida raíz y disponibilidad y se prueba con un hijo sintético; no está conectado a Slint. Falta ceder DRM/evdev, esperar el emulador, recuperar backend y registrar una sesión con hora fiable. No se promete cargar un savestate.

El esquema de historial y su consulta/borrado están implementados; la captura automática de sesiones reales espera ese ciclo de lanzamiento. `--history`, `--clear-history` y las opciones Start operan solo sobre la base propia. Borrar historial requiere dos confirmaciones en el menú, conserva catálogo, editorial y finalizaciones explícitas. No toca partidas.

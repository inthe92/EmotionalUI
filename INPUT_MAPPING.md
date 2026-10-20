# Entrada · fase 5

La capa `core::controls` convierte botones y ejes en acciones abstractas. `State::action` aplica esas acciones al mismo modelo Slint. El lector evdev, el teclado y los eventos de prueba permanecen separados de la composición visual.

| Teclado predeterminado | Mando simulado / acción |
|---|---|
| Flechas | Cruceta |
| Z / Enter / espacio | A · confirmar |
| X / Escape | B · volver |
| S / F1 | Start · menú |
| A | Select · biblioteca |
| Q / E | L1 / R1 · inicio / biblioteca |
| N | Otra recomendación |
| M | Movimiento reducido |
| F2 / F3 | Escenario anterior / siguiente, desarrollo |
| [ / ] | Franja anterior / siguiente, desarrollo |

`--keyboard-map platform/simulator/keyboard.example.toml` carga sobrescrituras TOML. Las acciones esenciales deben seguir accesibles. Alias antiguos no conflictivos B/Y/L/R permanecen admitidos. Las indicaciones del teclado y del mando son diferentes; un mapa personalizado puede requerir recordar las teclas que se han cambiado: las ayudas visuales muestran el mapa predeterminado.

El teclado normaliza las acciones antes de limitar la repetición: 320 ms iniciales y 120 ms posteriores para navegación; confirmar y menú requieren liberación. Dos alias pulsados simultáneamente no liberan la acción hasta soltar ambos. La pérdida de foco limpia teclas retenidas. El teclado de escritorio depende de los eventos de repetición del sistema; evdev usa su propio reloj y descarta EV_KEY=2.

El controlador del núcleo mantiene las fuentes por separado, aplica histéresis a los ejes, ignora duplicados, libera acciones al desconectar y descarta el intervalo entre SYN_DROPPED y SYN_REPORT. Las teclas opuestas mantenidas no generan oscilaciones en la repetición. Las animaciones no bloquean acciones posteriores. Menú y biblioteca conservan el índice de foco al volver.

## Simulación comprobada

`platform/simulator/scenarios/keyboard.toml`: eventos del FocusScope Slint, 111 pasos. `observed-buttons.toml`: 115 pasos sintéticos con códigos obtenidos anteriormente. `synthetic-stick.toml`: ejes deliberadamente sintéticos, no una calibración física. `moments.toml`: 12 pasos que actualizan contexto, paleta y recomendaciones. El escenario generado por la batería recorre nueve filas de una biblioteca de doce títulos y vuelve desde el menú conservando el foco.

Los tiempos de esos archivos son entradas deterministas, no latencias medidas. La reproducción no crea dispositivos, no usa uinput y no inyecta eventos en otros procesos.

## Preparación física histórica

La captura anterior `docs/phase5/guided-controls-c.json` y la confirmación del orden del usuario identificaron: ↑544, ↓545, ←546, →547, A305, B304, X307, Y308, Start315, Select314, L1 310, R1 311 y L2 312. El perfil parcial está en `platform/darkos/profiles/rg353v-observed-prefix.toml`. **R2 y el stick izquierdo siguen sin calibrar.** Su presencia en las capacidades no identifica la posición física.

El adaptador Linux abre únicamente dispositivos de entrada de caracteres, de solo lectura, por nombre y capacidades; no usa EVIOCGRAB. Descubrimiento y validación no equivalen a navegación física probada. Esta entrega no se ha desplegado en la consola.

## Ajustes sonoros y panorama · fase 8

Sin modificar mapas evdev físicos: Start/S abre opciones, cruceta abajo selecciona Sonido y A/Z abre sus ajustes. Arriba/abajo recorre filas; izquierda/derecha ajusta en pasos de cinco, A cambia la opción, B/X vuelve al mismo foco del menú. L/R (Q/E) conserva la alternancia ya existente entre vistas y cierra el panel de ajustes. El modo reducido M conserva la respuesta funcional y anula el panorama. Los replays usan WindowEvent de Slint; no sustituyen una validación física.

## Continuar inteligente · fase 10

Se conservan códigos/perfiles físicos. La nueva Action::Details tiene atajo D configurable y alternativa completa con Start → Esta partida. A confirma, B cierra el panel al mismo contexto, cruceta elige. Las dos entradas secundarias tienen índices lógicos 4/5; no son códigos evdev. Con dos/tres partidas, abajo entra en las compactas y después biblioteca; izquierda/derecha las recorre. La acción Biblioteca sigue abriendo directamente la colección. No se ha probado este mapeo en hardware durante esta fase.

# Fase 13 · UX, navegación y renderizado · 21-10-2026

**Estado: mejora de rendimiento confirmada por el propietario en RG353V; Fase 13 PARCIAL. Parallax/transiciones y acceso a cápsula por cruceta siguen abiertos.** Base Git `d65c03c`, árbol modificado conservado; sin commit/push. No se modifica el SO, ROMs, guardados, audio ni motor editorial.

## Cambios y comprobaciones

- L/R circular: Continuar → Tu momento → Biblioteca → Ajustes. Cada pulsación actualiza la sección lógica; sin cola de animaciones. Memoria de foco por sección e identidad al refrescar catálogo.
- Modal exclusivo de partida/audio/sistema; B cierra y restaura foco inmediatamente, sin propagación. L/R queda consumido dentro del modal. Lanzamiento conserva su bloqueo/cancelación existentes.
- Pestañas superiores 400×32 en (24,16), cápsula existente en (440,16). Cursor/foco independiente del destino activo; se retira el dock inferior. Pistas discretas al pie. Cápsula informativa, sin interruptores de hardware fingidos.
- Continuar conserva tres partidas: tarjeta 448×192 y dos secundarias de 216 px. Tu momento usa una tarjeta 400×184 y una alternativa de 248 px, con más paisaje a la derecha. Títulos largos expanden la altura dentro de pantalla; CTA compartido 188×36, cuerpo 13 px y materiales ambientales conservados.
- Transición finita e interrumpible de 240 ms; tarjetas recorren 28 px y habitación ±8 px sobre un encuadre 664×498, alineado a píxeles. No se inventan planos artísticos separados: la pintura sigue siendo un solo plano. Movimiento reducido fija inmediatamente el estado final.
- Primera revisión encontró una máscara blanca sobre la ventana con clima desconocido: Slint interpreta `colorize` alfa cero como ausencia de colorización. Se oculta explícitamente esa máscara sin efecto; no se cambia el arte ni se inventa clima. Segunda revisión corrigió el desbordamiento de títulos largos en la alternativa.

## Rendimiento: evidencia y límites

La comparación física previa SVG/caché no resolvió el lag según el propietario: CPU de navegación ~69/77 % de un núcleo, picos RAM 103/110 MiB, recorridos distintos. **No es una mejora comparable.** En reposo se observaron intervalos sin repintados; el contador de Slint no mide presentación LCD.

Se comprobó en el código de Slint 1.13.1 que KMS componía directamente sobre mmap de un dumb buffer. Sus tres buffers también provocaban repintados completos por una edad no cubierta por los modos reutilizables. El simulador trabajaba sobre RAM: no eran rutas equivalentes.

Prueba física aislada, sin tomar DRM master, crear framebuffer ni hacer pageflip: 640×480, cinco pasadas alfa, tres muestras. RAM **10,675 / 5,994 / 6,531 ms**; mmap propio **65,784 / 73,992 / 74,638 ms**; RAM+copia **7,273 / 7,043 / 6,809 ms**. Checksums completos iguales; EmulationStation mantuvo PID y DRM. Esto demuestra penalización de memoria para esa operación, **no demuestra todavía FPS ni fluidez de la GUI nueva**.

Corrección: override local del backend KMS 1.13.1 ya utilizado (212 KiB, avisos/licencias conservados), un buffer RAM retenido y una copia lineal al buffer DRM elegido. Se mantienen DRM, triple buffer y pageflip; ~1,17 MiB adicionales a 640×480 XRGB. Sin GPU, shaders ni nuevas bibliotecas. Control con el mismo ELF: `EMOTIONAL_PERF=full EMOTIONAL_KMS_DIRECT=1` usa la ruta anterior. Default: RAM. Métricas optativas acotadas de raster/copia/present scheduling; no equivalen a escaneo LCD ni pulsación-fotón.

## Validación ejecutada

- Rust workspace con simulator: **245 pruebas superadas**. Incluye siete nuevas de navegación, máscaras/caché, continuidad, audio y supervisor existentes.
- Python: **101 casos, 99 superados y 2 pendientes de arte privado legado**. Se actualiza el contrato geométrico 512/560/dock inferior al diseño expresamente solicitado, manteniendo legibilidad y añadiendo comprobaciones de espacio/máscara. No se eliminan pruebas.
- Tres replays de controlador: **160 pasos superados**, ciclo en ambos sentidos, ráfagas, foco, menú contextual, audio y cápsula. Seed SQLite original intacto.
- Comprobación del shadow sobre esos replays: **121 frames** iguales byte a byte a repintado completo, incluidas copias en tres buffers RAM. No es comprobación de flicker físico. Un test adicional del empaquetado XRGB pasa.
- Build release macOS/simulator y ARM64 KMS directo: correctos. Avisos no fatales conocidos del macro de Slint y linker Zig. `fmt` y `git diff --check` correctos. No se repiten benchmarks generales.
- Capturas RGB auténticas 640×480: cuatro secciones, contextual, foco, cápsula, movimiento reducido, vacío, sin arte y títulos largos. Dos revisiones de errores observados; sin aprobación artística automática.

## Paquete y siguiente puerta

ELF final SHA-256: `f04c8668e33f64d0903d88fa26f314e1bbe46f7e5d00946dfdc17d64fafb1b25`.
Paquete privado: `~/Desktop/EmotionalUI-Fase13-20261021/Paquete-privado/payload.tar.gz` (11.086.891 bytes); SHA-256 `716f615e0ab758ad4a2497cc9a961b4850c37618271d448bea93ecc0b6c00f74`. Restaurado localmente: 51 archivos/verificaciones y permiso ejecutable correctos. Arte privado embebido: **no redistribuir**.
Capturas accesibles: `~/Desktop/EmotionalUI-Fase13-20261021/Capturas/`. Evidencia privada: `~/Library/Application Support/EmotionalUI-Validation/phase13-20261021-104948/` y `f13-render-targets-20261021-controlled/`; incluye antes, dos pasadas, fallos previos y logs.

## Dos ensayos físicos supervisados del ELF f04c8668…

Ambas rutas del mismo ELF se ejecutaron ~60 s sobre biblioteca de 8.676 entradas y la misma DEMO invierno/nieve/Navidad. Dos SSH independientes, recuperación `true` correcta, preflight de ABI y hashes, backlight 130 sin cambios. Al terminar no queda EmotionalUI, ES recuperó DRM y el propietario confirmó ver de nuevo su menú. No se lanzaron juegos ni audio.

| Medida | A: directo a mmap | B: RAM+copia |
|---|---:|---:|
| CPU durante ventana manual activa, % de un núcleo | 77,1 | 27,8 |
| Acciones registradas | 28 | 77 |
| Pico RSS, MiB | 109,73 | 110,54 |
| Repintados por intervalo que informa Slint durante interacción | normalmente 1–2 | valores 1–12 |
| Tiempo del modelo por acción, p95 ms | 0,820 | 0,699 |

Los recorridos manuales fueron distintos: **no se calcula un porcentaje causal de mejora de CPU ni se llama FPS físicos a los repintados**. El contador de Slint divide renders por intervalos de su timer, que también puede retrasarse. No hay latencia evdev→fotón medida ni tiempos de presentación LCD. Los agregados KMS previstos en Drop no se emitieron al cerrar el backend: **raster/copia/present scheduling quedaron sin medición en estas sesiones**, no se inventan sus percentiles.

Valoración humana A: «lento, con animaciones pobres, muy poco fluido». B: «ahora sí va bastante rápido», «muy buena mejora de rendimiento». Confirmó L/R, táctil y Start; el diario de estas sesiones no contiene previous/next, por lo que la comprobación L/R física es testimonial, además de los replays instrumentados.

Pendientes observados: mayor sensación de parallax, transiciones más trabajadas y cápsula inaccesible de forma natural con cruceta. Se reproduce el enlace faltante: → desde Ajustes se saturaba en esa pestaña aunque la cápsula está a su derecha. Corrección local posterior: → alcanza la cápsula, A abre, B cierra, ← devuelve. Conserva el atajo ↑ anterior. **Esta corrección posterior no está incluida en el ELF físico f04c8668…; requiere nueva validación y autorización gráfica.** No se proclama finalizada la fase ni aprobados movimiento/diseño, 60 FPS, <100 ms, audio o lanzamiento físico.

Evidencia completa privada: `phase13-20261021-104948/physical-4f8e302a/summary.json`. Se conserva el paquete y ELF exactos probados para comparación; no se sobrescribe su manifiesto.


Candidato posterior de cápsula: SHA-256 `ec75dddea8dac8fd5ec4034de9c0beb2ba2eb1e83d9ea630013206aeb3850d9f`; paquete `~/Desktop/EmotionalUI-Fase13-20261021/Candidato-capsula/payload.tar.gz`. Compilación macOS/simulator y ARM64 correcta; replay adicional de 82 pasos pasa. Integridad de todos los archivos comprobada también dentro del archivo comprimido. **No desplegado ni probado físicamente**; el paquete físico anterior permanece intacto.

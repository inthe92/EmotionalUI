# Fase 14 · movimiento espacial — 21-10-2026

**Primera revisión probada en RG353V: fluidez confirmada por el propietario. Corrección posterior de entrada por los bordes comprobada localmente, pendiente de LCD.**
Base Git `d65c03c`, cambios previos conservados. Sin commit/push ni cambios del SO.

- Trayectorias de cuatro páginas persistentes; destino lógico inmediato, inversión desde posición actual, generación que descarta callbacks anteriores. Sin cola ni fade global.
- `MotionDesign`: micro110/pulsación80, secciones260, funcionales200, panel180/cierre130, secundarios45 ms. Son duraciones del modelo, no mediciones del LCD.
- Habitación estable con desplazamiento lejano ±6 px y franja inferior derivada ±12 px; dos recortes preparados al cambiar claves de iluminación. Es profundidad aproximada de la pintura existente, no separación artística real de objetos. Pestañas/reloj/cápsula permanecen anclados.
- Continuar conserva tres partidas; Tu momento usa arte horizontal, explicación y alternativa desplazada a la derecha. Protagonistas distintos si hay otro candidato válido; ranking/persistencia originales intactos. Cinco slots de arte acotados conservan ambas imágenes durante la transición.
- Menús visualmente retenidos durante cierre breve, propiedad modal lógica inmediata y foco restaurado. Movimiento reducido elimina desplazamientos/parallax; regresión detectada y corregida antes del paquete.
- Render RAM+copia F13 conservado. Métricas KMS agregadas cada ≥2 s cuando hay frames: resuelve la ausencia del resumen Drop; no añade repintado ni logs por frame.

## Evidencia

Privada: `~/Library/Application Support/EmotionalUI-Validation/phase14-20261021-115641/`.
Entrega: `~/Desktop/EmotionalUI-Fase14-20261021/` — 48 PNG reales, secuencias intermedias y estados largos/vacíos/sin arte. Las capturas proceden del buffer software Slint, no del LCD; codificarlas altera el muestreo temporal.

Pruebas: 256 Rust y 101 Python (99 correctas, 2 omisiones históricas justificadas por arte privado) en primera integración; 174 pasos de navegación correctos. Tras las correcciones: build simulator/ARM64, fmt/diff-check, 7 tests de motion y 32 pasos de interrupción/modal/reducido pasan; el replay comprueba viewport completo y los tests consideran la anchura total de las tarjetas por ambos lados. No se retiran pruebas anteriores.

Mac offscreen release, mismo catálogo/escena y seis cambios espaciados 500 ms, sin capturas durante perfil (`release-profile/report.json`):

| Métrica | F13 local | F14 local |
|---|---:|---:|
| CPU total / ensayo ~8 s | 1,127 s | 1,014 s |
| Pico RSS | 96,08 MiB | 102,73 MiB |
| Raster mediana / p95 | 7,26 / 13,16 ms | 8,09 / 24,23 ms |
| Entrada despachada → fin raster mediana / p95 | 24,35 / 25,59 ms | 19,27 / 76,31 ms |
| Rasterizaciones totales | 173 | 101 |

Sin rasterizaciones en segundos finales 5–7 de reposo. Los counts no son FPS presentados; las ventanas dibujadas y el movimiento cambiaron. Una pasada no demuestra mejora ni preservación de fluidez física: p95 de raster mayor y pico RAM +6,65 MiB requieren revisión LCD. No se dispone de input-to-photon ni FPS físicos.

## Candidato privado

ELF corregido AArch64 LinuxKMS/software: `1e635ac6b1bc9e1b1ac33ad6aa90264d5f88a75a3fac56f486b371d0c8b656d8`.
Paquete: `~/Desktop/EmotionalUI-Fase14-20261021/Paquete-bordes/payload.tar.gz`.
Arte provisional embebido con derechos pendientes: no distribuir. Avisos conocidos de macro upstream y optimización obsoleta Zig; compilación correcta.

## Prueba física y corrección de borde

Ensayo autorizado de120 s del ELF anterior `f2193b4e…`: salida0, RAM máxima muestreada113,84 MiB, CPU del servicio80,332 s /120,453 s.90 acciones registradas; trayectoria manual, sin comparación controlada con F13. Último resumen KMS (295 muestras): raster87,63/112,33 ms mediana/p95; copia0,847/1,081 ms. Son tiempos CPU, no FPS ni presentación LCD. Cola de muestreo: normalmente3–4% de un núcleo, último valor12%; no se controla exactamente la ventana de reposo.

El propietario confirma fluidez y retorno visible a ES, pero detecta aparición desde el interior. Causa comprobada: el viewport de tarjetas medía496 px y escalaba el recorrido640→496. Corrección mínima: viewport y recorrido640 px completos; sin cambiar las posiciones finales, duraciones, curva, parallax ni anclajes superiores. La inversión L conserva dirección y posición visible actual. No se ha iniciado un segundo ensayo.

Evidencia local posterior: `edge-final-sequences/`, `edge-geometry.json`, `edge-viewport-regression/`. Comparación inspeccionable: `~/Desktop/EmotionalUI-Fase14-20261021/Capturas/correccion-borde.png`. Las dos trayectorias empiezan/terminan con rectángulos completamente fuera de pantalla; secundarios y paneles funcionales incluidos en tests. Se conservan las capturas anteriores.

**Pendiente:** valoración LCD del candidato corregido `1e635ac6…`, profundidad/ritmo artístico y cápsula por cruceta. No hay aprobación artística automática ni60 FPS demostrados. ES activo, DRM recuperado, brillo130 intacto, SSH de diagnóstico cerrado. No se ha alterado dArkOS, arranque, ROMs o guardados.

## Ensayo de cinco minutos y ritmo solicitado

ELF1e635ac6:300,458 s, salida0,195 acciones, CPU acumulada98,500 s, pico RSS126,14 MiB (muestreo1 Hz). Último resumen KMS423 muestras: raster87,50/111,61 ms mediana/p95; copia0,845/1,091 ms. Sin FPS LCD; recorrido manual distinto al anterior, no comparación controlada. Retorno visible a ES confirmado por el propietario, brillo130 intacto; ambas conexiones SSH cerradas. Evidencia privada: `physical-5min-1e635ac6/summary.json`.

El propietario solicita animaciones más lentas, fluidas y naturales; no se registra aceptación artística. Después del ensayo se ajusta sólo el movimiento local: secciones360 ms, funcionales240 ms, curva espacial ease-in-out con velocidad inicial/final cero; cierre de menús, posiciones, límites640 y parallax se conservan.8 tests de motion,32 pasos, fmt/diff-check y build simulator/ARM64 pasan. Secuencias: `rhythm-sequences/`, copia en escritorio `Ritmo-relajado/`.

Nuevo ELF local: `ec124ea072be174efaf625f358d8ff9ac504c1d9ffdc943295c44b144c660bef`. No desplegado ni probado en LCD; no confundirlo con el1e635ac6 del ensayo. No se inicia otra prueba automáticamente.

## Ritmo confirmado y búsqueda de más fotogramas

Ensayo60,375 s del ELFec124ea0 completado con salida0 y retorno visible confirmado; brillo130 intacto, pico RSS115,35 MiB. El propietario indica que el ritmo se siente mejor y solicita más FPS. Último agregado KMS200 renders: raster96,02/121,27 ms mediana/p95, copia0,860/1,114 ms; no son FPS físicos. No se promete duplicarlos.

Optimización posterior mínima en XRGB: omitir mezcla para alpha0 y sustituir directamente para alpha255; conservar fórmula Slint para el resto. Mismo RAM→DRM, sin cambiar ambiente/ritmo/temporizador.65.536 pares de alpha coinciden con referencia; tres capturas reales Slint XRGB (Continuar, Tu momento, Biblioteca) son idénticas píxel a píxel al binario anterior. Build macOS/ARM64 y fmt/diff-check pasan.

Perfil local XRGB idéntico de seis cambios, sin capturas: mediana raster9,716→7,815 ms; p9513,568→11,382 ms; CPU1,613→1,322 s; pico RSS103,17→102,84 MiB. Una pasada Mac, no mejora física demostrada ni doble FPS. Diagnóstico separado full/simple reduce mediana7,963→5,905 ms quitando capas sólo para medir; no se elimina ninguna del producto. Evidencias privadas `pixel-fast-path-profile/`, `pixel-fast-path-equivalence/`, `fps-diagnostic/`.

Nuevo ELF candidato: `2f0ed1698ee1fbd6dfb01251747f1c3512497e22ae60af21a2bdf8297e1e6164`. Paquete privado: `~/Desktop/EmotionalUI-Fase14-20261021/Prueba-render-optimizado/payload.tar.gz`. **Pendiente de prueba física autorizada.** Consola devuelta a ES; SSH cerrado. Sin commit/push.

## Validación del XRGB optimizado y objetivo60 FPS

ELF2f0ed169 probado durante60,409 s, salida0 y retorno visible a ES confirmado; brillo130 intacto, CPU acumulada71,154 s. Último agregado207 renders: raster82,050/104,335 ms mediana/p95; copia0,872/1,086 ms. Ruta manual distinta a la anterior, sin FPS de presentación medidos. El propietario confirma más suavidad, pero desea60 FPS; su impresión de30 FPS no se presenta como medición. Evidencia: `physical-pixel-2f0ed169/summary.json`. Mantener360/240 ms.

Se evaluó localmente `EmbedForSoftwareRenderer` del Slint1.13.1 instalado: mediana XRGB8,086→7,207 ms, p9512,716→16,940 ms, pico RSS103,66→42,67 MiB. Cambia métricas tipográficas y saltos de línea; **opción descartada**, `app/build.rs` restaurado a EmbedFiles. Prueba en `texture-build-profile/` y `texture-build-visual/`, sin prueba física ni adopción. No se cambian dependencias ni el diseño para fabricar FPS.

**60 FPS no alcanzados ni medidos.** El presupuesto de16,7 ms por fotograma queda por debajo del coste de raster observado. Siguiente mejora concreta a evaluar: caché de composiciones estáticas antes de desplazar las capas; evitar ampliar efectos o reducir duración. Conservar el candidato2f0ed169 como referencia física. No se inicia otra prueba ni se cambia dArkOS; SSH cerrado, sin procesos propios activos ni commit/push.

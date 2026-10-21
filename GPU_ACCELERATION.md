# Investigación dirigida · texturas y capas · preparación

Nine-slice sí está en uso, pero la rama de textura temporal requiere repetición; los materiales actuales usan None. Dos tests de fit9slice1.13.1 confirman nueve regiones sin tiling y control positivoRepeat; B2 descartada. Clipping redondeado ya usa capas automáticas. A mínima ARM64 preparada, sin cambio de VSync ni UI; C pendiente de A. Usuario no está junto a la consola: no se ha conectado, copiado ni ejecutado nada nuevo. [Auditoría y límites](GPU_RENDER_PIPELINE_AUDIT.md).

---

# Fase 15.3 · tres ensayos completados · 21-10-2026

Se conservan los ELF09f71dfb GPU y ea717b33 software y los cambios locales. Respaldo incremental privado verificado en `~/Library/Application Support/EmotionalUI-Validation/phase15-3-20261021-160136/`.

- Referencia15.2 recuperada:13,015→21,602Hz, p95100,058→83,396ms; valoración humana GPU favorable y retorno visible confirmados en la petición actual.
- La espera previa de DRM era mínima; render/swap sigue siendo el área de coste, sin prueba aún de saturación GPU o doble VSync. EGL por defecto usa intervalo1; el driver puede limitar el intervalo solicitado. [API Glutin0.32.3](https://docs.rs/glutin/0.32.3/glutin/surface/enum.SwapInterval.html).
- Se añaden trazas acotadas por fotograma en RAM: solicitud→inicio→swap→fin espera previa→fin swap→fin GBM/framebuffer/submit. Una escritura al cerrar, sin cambiar la secuencia gráfica por defecto. No miden ejecución GPU ni fotones.
- **IntentoA ejecutado una vez, resultado limitado:** `EMOTIONAL_GPU_SWAP=immediate`, opt-in con diagnósticos. Solicita intervaloEGL0 mediante API oficial; mantiene los pageflips DRM con EVENT, espera de vida de buffer, duración360/240ms y recursos. No limita artificialmente a30FPS ni cambia drivers. El controlador aceptó la API; se comparó con la misma ruta y binario sin esa variable. Baja la mediana de swap, pero no el p95 de presentación: no se considera resuelto el pacing.
- Candidato de perfilado209d68e5dc0eaced18e2b0baff84abfcb5ef9b56bd26bf98bd847513795a0966: ARM64 y loader correctos; mismos recursos propios y UIprevia. Paquete aislado en `/home/ark/emotional-ui-tests/gpu15-3-20261021/payload`. DosSSH, ESactivo, brillo130 y recuperación PID1 comprobados por comandos. Tras autorización expresa se ejecutaron software/RAM, GPUreferencia e intentoA,60s cada uno. Todos finalizaron con salida0; el usuario confirmó EmulationStation visible. DRMmaster volvió a ES y brillo130 se conservó.
- IntentoB no seleccionado ni implementado: sólo se elegirá si los resultados deA justifican una segunda hipótesis. Máximo dos correcciones de rendimiento.
- Tipografía: el software usa avances `i16`, FemtoVG `f32`. DejaVuSans-Bold14 de «Entre montañas» suma125,556px en avances fuente y116px truncando glifo a glifo, frente a120px disponibles. Se conserva geometría y fuente; no se promete equivalencia de wrap. El recorte superior se debe a centrar varias líneas sin limitar overflow en CompactGame; corregido con `overflow:elide`, sin cambiar fuente, tamaño ni tarjeta. Nueve casos locales antes/después:54capturas640×480/27pares, cero cambios fuera de las tarjetas secundarias. El título100caracteres sigue completo en Start. Evidencia válida en `title-verified/`; la primera captura con fixtures pendientes en WAL se marcó REJECTED y no acredita estos casos. GPU/LCD de la corrección todavía NO PROBADOS.
- Pruebas actuales: tres casos nuevos del analizador de etapas, nueve del protocolo/fallback y trece herramientasF12 pasan; compilación GPU de perfilado y simulator correcta. Nueve casos de títulos y27comparaciones focalizadas pasan; el ELF de perfilado mantiene intencionadamente la UIprevia para la comparación A/B. Analizador de cadencia15.2 sin cambios. Las tres pasadas físicas y CPU/RSS comparables ya están medidos abajo. Estabilidad prolongada y prueba manual del candidato con títulos corregidos siguen pendientes; software continúa por defecto y GPU experimental.

## Resultados de los tres ensayos autorizados

Mismo protocolo13acciones, fixtureseed restaurado, escena DEMO y audio apagado. Antes de cada pasada se conservó la caché propia anterior fuera de su ruta y se arrancó sin caché de carátulas; no se borraron recursos. GPUreferencia e intentoA usan el mismo ELF209d68e5, con UIprevia a la corrección de overflow para no mezclar cambios.

| Medida de animación cubierta | Software | GPU referencia | GPU A, intervaloEGL0 solicitado |
|---|---:|---:|---:|
| Intervalos medidos | 26 | 45 | 51 |
| Cadencia agrupada | 13,10Hz | 19,84Hz | 22,00Hz |
| Intervalo mediano | 66,743ms | 50,040ms | 33,361ms |
| p95 | 100,043ms | 100,069ms | 100,068ms |
| CPU en reposo, un núcleo | 3,31% | 3,23% | 3,19% |
| RSS mediana en reposo | 83,06MiB | 156,20MiB | 156,74MiB |
| Pageflips confirmados30–55s | 0 | 0 | 0 |

p99 omitido por menos de100 intervalos. El analizador de cadencia sigue intacto. Un último flip no observado por pasada se excluye; timers tempranos1–2ms e inversión tardía hasta55ms quedan declarados. No se interpreta el promedio global incluyendo reposo como FPS de animación.

**Cuello de botella acotado:** solicitud de redibujado→inicio GPU mediana0,222ms/p951,051ms en referencia: no hay evidencia aquí de una cola larga de solicitudes. Espera previaDRM mediana0,063ms/p950,183ms; GBM/framebuffer/submit0,301ms/p950,765ms. Antes del swap:15,606ms/p9546,975ms. Dentro del swap:30,944ms/p9548,679ms. La cola de presentación previa no explica el coste principal observado; el camino comandos/swap sí concentra las esperas. No se separa ejecución real GPU de sincronización interna del driver.

**IntentoA:** swap mediano13,672ms/p9548,607ms; render→submit31,801ms/p9594,646ms frente a51,103ms/p9593,874ms de referencia. Mejora la mediana, **no elimina la cola larga** ni demuestra30FPS sostenidos. Sólo hay una pareja, no una ganancia reproducible porcentual establecida. El gobernador existente varió frecuencias: GPU200/300MHz observados en referencia y200/800MHz en A; no se cambió. No atribuir toda diferencia exclusivamente al intervaloEGL. Se conserva como experimento explícito optativo; no pasa a predeterminado ni se declara candidato mejor establecido. IntentoB no se ejecutó: no hay autorización nueva ni hipótesis adicional suficientemente aislada para iniciarlo automáticamente.

**Memoria/estabilidad:** ahora existe referencia software equivalente. RSS software83,06MiB y GPU156,20/156,74MiB durante ventanas de24,1s de reposo. El crecimiento inicial GPU149→156MiB coincide con visitar vistas; después permanece constante dentro de la ventana. No prueba ausencia de fugas a largo plazo. CPU de interacción30,73% software y14,52/14,84% GPU de un núcleo. Servicios:7,516s/60,327s,5,601s/60,241s,5,617s/60,383s CPU/runtime. Cero errores DRM/EGL observados, retorno correcto en las tres pasadas, sin procesosfrontend restantes. Mandos libres, audio, juegos, estabilidad prolongada y corrección de títulos GPU/LCD no se validaron en este recorrido automático.

**Valoración humana:** regreso visible a ES confirmado. Tercera pasada: «La he sentido bien no diría a30 ni a60 pero bien». No hay aprobación de GPU predeterminada ni demostración de mejora perceptible A frente a referencia. Se mantiene **opciónB: software predeterminado, GPU experimental**, con fallback externo. Binarios anteriores y209d68e5 conservados; ninguna release preparada para publicar por este resultado solo. No se realizaron más ensayos, commits, push ni cambios de sistema.

Evidencia privada: `trial-results/comparison-summary.json`, tres trazasDRM/etapas y tres muestreos1Hz; archivo verificado SHA-256 `5bf0a7aa1b5358a0809e34ce812e03afe8493efecfe1ad61ba604845f75ae95c`. Confirmación humana guardada en `human-confirmation-three-trials.json`.

---

# Fase 15.2 · Mali-G52 / FemtoVG · 21-10-2026

Estado: aceleración real, prototipo y frontend completo verificados en RG353V; dos recorridos de rendimiento autorizados terminados. Mejora medida, sin30/60FPS sostenidos ni equivalencia tipográfica exacta. Software continúa por defecto; nada publicado.

## Stack observado en la RG353V

RK3566, Linux 5.10.226, dArkOS/Debian 13.6, glibc 2.41. DRM de pantalla `rockchip-drm`; GPU `mali`, no Panfrost. EGL/GLES/GBM enlazan con `libmali-bifrost-g52-g29p1.so`. Existen `card0`, `renderD128` y `mali0`; el usuario ya dispone de permisos. No se han instalado ni sustituido controladores, bibliotecas, servicios o configuraciones.

Sonda offscreen sobre GBM/render node: EGL_VENDOR ARM, EGL 1.5, GL_VENDOR ARM, GL_RENDERER Mali-G52, GLES 3.2 g29p1. Readback de un píxel y GL_ERROR=0. El frontend confirma de nuevo Mali-G52; no llvmpipe/softpipe. La sonda offscreen no acredita presentación física por sí sola.

## Candidatos y fallback

Slint permanece en **1.13.1**. Feature adicional `linuxkms-gpu` habilita FemtoVG con el backend LinuxKMS existente; `linuxkms-direct` conserva software. Fuentes TTF y EmbedFiles originales, escena, geometría y motion360/240 conservados.

| Artefacto | SHA-256 | Alcance |
|---|---|---|
| Prototipo GPU | `e637d3ce74b301c8f15eccb1b02968853ed6e073044ae64059e196f2a33038fc` | Panel físico móvil, salida correcta y ES visible confirmados por usuario |
| Frontend GPU | `09f71dfb5271fb641ce83e6156a3712e1b0d3cd9d0ad7b56fa41e3bf272c6103` | Cuatro secciones y capturas en RG353V |
| Frontend software equivalente | `ea717b33d7d45efa47c9a8294a28e775dd0ff056aedc6a318fa280b6f4c67695` | Mismas fuentes/escena/replay de diagnóstico |

Origen HEAD `d65c03c8306ed95e42a3110d734133a1dbba04f9` más cambios locales conservados: los hashes ELF, no HEAD solo, identifican estos ensayos. Auditorías ELF/loader pasan en este dispositivo; GLIBC máxima requerida GPU2.35/software2.34. No certifica otros firmwares.

El SDK de compilación GPU es una copia privada del SDK base y bibliotecas ya existentes del dispositivo. No se distribuyen esas bibliotecas. Cargo.lock añade dependencias de FemtoVG sin actualizar las versiones anteriores. Compilar con `EMOTIONAL_KMS_FEATURE=linuxkms-gpu tools/arm64/build.sh`; el SDK GPU se selecciona mediante `EMOTIONAL_ARM64_SYSROOT`. `EMOTIONAL_ARM64_BIN=gpu-probe` compila únicamente el prototipo.

La selección GPU es explícita: `SLINT_BACKEND=linuxkms-femtovg` y ejecutable separado. El helper admite `--renderer software|femtovg`, software por defecto. El ejecutable software independiente no necesita Mali. Si GPU falla, cerrar la unidad temporal restaura ES mediante ExecStopPost de PID1; un límite externo de ensayo protege incluso si el frontend se bloquea. Otra sesión SSH permanece disponible. Software se ejecuta externamente en otro proceso; no se presupone que un proceso EGL averiado pueda recuperarse internamente.

## Comparación visual

Ocho capturas reales del renderer, 640×480, cuatro secciones de cada variante; no fotografías del LCD. Biblioteca, carátulas e historial sintéticos propios, mismo seed restaurado antes de cada recorrido, contexto DEMO nieve/Navidad16:00. Capturar y codificar PNG se separa de la medición de rendimiento.

El usuario no apreció diferencias en los dos ensayos. La inspección de capturas conserva familia, jerarquía, imágenes, materiales y geometría, pero **no son píxel idénticos**: antialiasing y composición difieren, y «Entre montañas» ocupa una línea en una tarjeta secundaria software y dos en FemtoVG. No hay solapamiento en ese ejemplo. Slint1.13.1 utiliza algoritmos distintos de medición/composición de texto en software y FemtoVG; no se ha cambiado el tamaño, fuente ni contenido para esconderlo. Equivalencia tipográfica estricta pendiente antes de escoger renderer predeterminado. El recorte de títulos señalado en F15 sigue siendo otro pendiente, no resuelto por esta fase.

Evidencia privada: `~/Library/Application Support/EmotionalUI-Validation/phase15-2-319bef33/visual-results/`, pares `comparison-0.png` a `comparison-3.png` y logs íntegros. Recursos provisionales y capturas no se añaden a una publicación.

## Protocolo reproducible y límites de medición

`EMOTIONAL_GPU_REPLAY=fixed-v1 EMOTIONAL_PERF=full EMOTIONAL_PERF_TRACE=1`, solamente con `--trial`:13 acciones por tiempos absolutos, cuatro secciones en ambos sentidos, inversión rápida, Details/Back y reposo30–55s. Los controles reales y motion de producción reciben las acciones; no se emula el dibujo. No pulsar botones en este recorrido. Mismo seed, contexto, caché, audio desactivado y recursos; sin juegos ni HTTP. La comparación no mide audio, launcher, meteorología real o controles humanos.

DRM registra confirmaciones con secuencia y timestamp kernel monotónico, acotadas en RAM; una sola línea de traza al cerrar. `tools/analyze_gpu_trial.py LOG` verifica integridad y separa reposo de grupos de animación cubiertos. Un flip enviado sin confirmación no se cuenta como presentado; los huecos de reposo no son FPS de animación. CPU/RSS se muestrean externamente a1Hz. Una llamada CPU a render/swap no mide ejecución GPU ni input-to-photon. p99 se omite con menos de100 intervalos. Frecuencia devfreq no equivale a utilización GPU; no hay contador de utilización validado.

## Resultados y decisión

Dos recorridos automáticos de60s,13 acciones cada uno; nueve grupos de movimiento con fin de modelo y presentación posterior observados. Ambos terminaron con salida0, sin errores DRM y con ES activo; brillo130 antes/después. El propietario confirmó en la petición de Fase15.3 la mejora de suavidad, las cuatro secciones y el regreso visible a ES de estos ensayos.

| Medida | Software | FemtoVG / Mali-G52 |
|---|---:|---:|
| Confirmaciones DRM totales, incluido inicio/reposo | 59 | 76 |
| Intervalos dentro de grupos de animación cubiertos | 28 | 49 |
| Intervalo mediano de esos intervalos | 66,726ms | 33,376ms |
| p95 de esos intervalos | 100,058ms | 83,396ms |
| Cadencia agrupada: intervalos / suma de sus tiempos, excluyendo pausas entre grupos | 13,015Hz | 21,602Hz |
| Cadencia media dentro de cada grupo, mínimo–máximo | 11,992–13,838Hz | 13,324–29,979Hz |
| Repintados confirmados durante reposo30–55s | 0 | 0 |
| CPU acumulada del servicio / duración | 7,526s / 60,368s | 5,434s / 60,246s |
| CPU media del servicio, equivalente a un núcleo, arranque incluido | 12,47% | 9,02% |
| CPU proceso en reposo medido24,10s | NO MEDIDA | 3,19% de un núcleo |
| RSS proceso en reposo | NO MEDIDA | 155,61MiB |

La GPU presenta aproximadamente66% más actualizaciones en estos grupos, pero la mediana33,4ms **no demuestra30FPS sostenidos**: hay intervalos hasta116,7ms en un grupo y cadencias medias desiguales. No se alcanzan60FPS. p99 no se publica: menos de100 intervalos de animación por variante. No se extrapola a bibliotecas reales, audio o sesiones prolongadas. Este protocolo sustituye la comparación manual desigual histórica, no la reinterpreta.

Límites concretos: un último flip enviado de cada variante no fue observado como confirmado y se excluye. Los temporizadores entregaron varias acciones1–2ms antes del objetivo; inversión rápida llegó hasta20ms tarde en software y27ms en GPU. Se conservan los tiempos observados y el analizador informa PARTIAL con advertencias, sin fabricar confirmaciones ni cambiar el ejecutable. La ventana estable no contiene repintados, pero eso no demuestra CPUcero.

El muestreador externo no arrancó en software; no se inventan RSS ni CPU de reposo equivalentes y no se repite una sesión gráfica sin autorización. CPU acumulada del servicio sí está medida en ambas. GPU completó60 muestras externas;156MiB no se compara directamente con el histórico de otra biblioteca/binario. El render+swap+submit de CPU en GPU tiene mediana37,484ms/p9599,264ms, frente a72,456ms/p95101,900ms del camino software; incluye espera/sincronización y no es tiempo de ejecución GPU. El swap CPU mediano14,070ms tampoco prueba coste puro GPU. No hay medida fiable de utilización GPU ni latencia desde botón físico a fotón. El próximo cuello de botella está acotado al camino render/swap/presentación irregular; no se atribuye a shaders, texto o driver sin más aislamiento.

Evidencia íntegra privada: misma raíz anterior, `performance-results/`, trazas, reportes de unidad, `gpu-analysis.json`, `cpu-analysis.json`, `comparison-summary.json` y muestras JSONL. Archivo recogido verificado SHA-256 `f10d073c1a2a3cbbb9ad8501312a114ee1ed0739f237e563fb04876e6f0e9a2c`.

Decisión: conservar09f71dfb como **candidato GPU experimental medido**, acompañado del softwareea717b33 y recuperación externa. No convertir GPU en predeterminado todavía: diferencias de texto, estabilidad prolongada, mandos humanos de este ELF y memoria comparada siguen pendientes. Para una futura preview, software continúa por defecto y GPU queda optativo sujeto a revisión; ninguno queda certificado para publicación aquí. Persisten derechos de arte provisional, título alternativo de F15 y validación de distribución. No hay release, push ni commit.

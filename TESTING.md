# Orden de publicación · comprobación local del candidato · 21-10-2026

Publicación expresamente autorizada tras verificaciones; no se solicita otra aprobación genérica. GitHub confirmado PUBLIC tras autorización específica; página accesible sin autenticación. Supervisión/autorización gráfica actual también pendiente, por lo que no se ha iniciado sesión gráfica ni prueba de instalación física.

**VERIFICADO:** compilaciones ARM64 software y FemtoVG 0.1.0-preview.1, remapeo de rutas Rust/C (sin rutas personales del Mac en ambos ELF), 27 tests focalizados de distribución/instalador y git diff --check. Avisos completos recibidos conservados; dos declaraciones MIT sin archivo/license propio llevan suplemento estándar identificado y atribución original sin copyright inventado. Escaneo completo de284blobs históricos y301fuentes candidatas sin patrones comprobados de secretos, rutas personales, IP privadas, ciudad configurada o ROM/BIOS; metadatos normales de autor Git conservados.

**ZIP LOCAL:** EmotionalUI-RG353V-v0.1.0-preview.1.zip, 29720773 bytes; SHA-256 `5d7f6a499e349f9be97095f0a3a9293384ea13d7876f463c85f841996058eded`. Integridad ZIP, hashes del manifiesto y de sus301fuentes, permisos ejecutables y ciclo de copia propia/restauración/desinstalación comprobados en directorio aislado del Mac. Estas operaciones no ejecutan el ELF ni certifican EmulationStation.

**NO PROBADO:** instalación gráfica del ZIP en RG353V, recorrido de las cuatro vistas/controlador/audio, salida, restauración visible y desinstalación. No se publica antes de verificarlo. No se ha cambiado visibilidad, creado tag ni publicado Release. Source snapshot y candidatos previos preservados; no hay nueva optimización.

---

## Preparación anterior conservada — el estado actual está arriba

# Fase 16 · pruebas de distribución · 21-10-2026

**VERIFICADO LOCAL:** 11 tests nuevos de distribución +16 existentes del instalador pasan. Cubren dArkOS con identidad Debian y marcador de dispositivo, rechazo de paquete privado, manifiestos/hashes, permisos ejecutables tras extraer ZIP, allowlist de fuentes sin archivos privados ajenos, integridad, instalación propia/restauración/desinstalación, recuperación independiente en el plan systemd, selección software/GPU, exclusión de variables diagnósticas, procesos de emuladores y rechazo de rutas/ejecución root. Los fallos iniciales del test de árbol y de rutas temporales macOS se corrigieron y se repitió la suite afectada.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p test_preview_distribution.py -v
PYTHONPATH=tests PYTHONDONTWRITEBYTECODE=1 python3 -m unittest test_phase11_tools.Installer -v
```

La prueba ZIP usa ELF sintéticos de cabecera, NO los ejecuta, NO certifica ARM64 ni gráficos y NO produce el ZIP real para usuarios. `tools/package_preview.py` rechaza la preparación real mientras estén pendientes habitación, notices y privacidad; no hay ZIP público ni SHA inventado. No se reejecutan compilaciones ni replays de renderizado sin cambios Rust/Slint durante esta preparación.

**EXPERIMENTAL:** acceso gráfico desde Ports preparado con servicio transitorio y recuperación PID1; software por defecto y GPU optativa. Rechaza dependencias faltantes antes de ceder pantalla; --trial bloquea juegos. No cambia servicios persistentes, arranque ni datos de biblioteca. El límite de 15 minutos es sólo de la demo.

**PROCEDENCIA ACTUALIZADA:** el propietario confirma Foto 1 propia; declaración registrada para preparar las siete pinturas. No se afirma verificación independiente ni nueva prueba gráfica. **BLOQUEADO:** revisión de dos avisos upstream todavía incompletos, privacidad, ZIP exacto y autorización final de inventario/historial. La actualización de README conserva los dos commits remotos documentales, sin checkout/merge ni publicación.

**NO PROBADO:** ZIP exacto, instalación desde ES, audio audible, recorrido de mandos, salida/restauración/desinstalación y duración de instalación en consola. Requiere presencia actual, autorización específica, dos conexiones SSH y recuperación comprobada; ninguna prueba gráfica se ha iniciado en esta Fase16. No se anuncian 30/60 FPS estables ni diez minutos. [Puertas](RELEASE_READINESS.md).

---

## Evidencias previas conservadas

# Verificación vigente · investigación del pipeline GPU

Dos tests Rust llaman fit9slice1.13.1 en seis tamaños reales y controlRepeat; cuatro Python verifican conteo DRM/grupos incompletos/seguridad y un test de duración del prototipo pasa con desktop+linuxkms-gpu. A mínima ELF64ARM64 compilada y auditada:ecd6ed82…; hashes del paquete correctos. Sin métricas físicas nuevas, sin modificación del analizador del replay13acciones y sin ensayo mientras el usuario está ausente. Ver [GPU_RENDER_PIPELINE_AUDIT](GPU_RENDER_PIPELINE_AUDIT.md).

---

# Verificación vigente · Fase 15.3 en preparación

Compilación GPU ARM64 de perfilado y simulator correcta; tres tests de etapas CPU, nueve del protocolo/fallback y trece herramientasF12 pasan. Corrección de overflow: nueve casos/54capturas/27pares locales640×480; cero diferencias fuera de las tarjetas secundarias. Título100 completo en Start. Primera pasada rechazada porfixturesWAL, reemplazada por `title-verified/`; GPU/LCD de esta corrección pendiente. Hash209d68e5… y loader verificados por SSH, recursosfixture intactos. Instrumentación por etapas sólo con diagnóstico; no mide ejecuciónGPU. Tres ensayos físicos de60s autorizados y completados, con confirmación humana de ESvisible. Software/GPUreferencia/A: cadencia agrupada13,10/19,84/22,00Hz; mediana66,743/50,040/33,361ms; p95100,043/100,069/100,068ms. Ninguna demuestra30FPS sostenidos. RSS en reposo83,06/156,20/156,74MiB, CPU3,31/3,23/3,19% de un núcleo; cero pageflips en reposo y errores DRM observados. Mejora de mediana A no elimina cola larga ni está repetida; frecuenciasGPU distintas declaradas. Misma UI y ELF209d68e5 en ambas pasadasGPU. Capturas de títulos son locales: aún no incluyen pruebaGPU/LCD. El protocolo/analizador DRM15.2 se conserva; resultados anteriores permanecen debajo. [Detalle](GPU_ACCELERATION.md).

---

# Verificación vigente · Fase 15.2

macOS simulator y ARM64 software/FemtoVG compilan con Slint1.13.1 conservado; loader y hashes verificados en dArkOS.3 pruebas replay,4 caché,8 motion,3 estadísticas DRM,9 Python GPU/analizador y13 herramientas F12 pasan; replay local32 pasos pasa. fmt/diff-check correctos. Prototipo GPU físico con retorno visible confirmado;8 capturas Slint reales640×480 de las cuatro vistas, comparación revisada. Dos recorridos físicos automáticos13 acciones/60s terminan sin errores y ES activo:28/49 intervalos de animación, mediana66,726/33,376ms,p95100,058/83,396ms, cadencia agrupada13,015/21,602Hz. Sin30/60FPS sostenidos. Cero confirmaciones DRM en reposo30–55s; GPU3,19% CPU de un núcleo y155,61MiB RSS en24,10s. Muestreador software no arrancó: CPU/RSS de reposo comparables NO MEDIDOS. Último flip no confirmado excluido y timers−2..+27ms declarados; análisis PARTIAL, sin errores de traza. Tipografía no píxel idéntica y un salto de línea distinto; no se ha rediseñado. Valoración física final pendiente. [Protocolo, hashes y límites](GPU_ACCELERATION.md).

---

# Verificación vigente · Fase 15

4tests de caché,8motion,2máscaras,3simulator,2estadísticas DRM y32pasos de navegación pasan. Build simulator/ARM64, fmt y diff-check correctos.18 comparaciones Slint/XRGB640×480 pixelidénticas, con cuatro contextos y fotogramas R/L fijados al mismo tiempo. Sonda sólo simulator; no mide FPS.

`tools/check_phase15_cache.py --binary BIN --seed SEED --fixtures FIXTURES --out NUEVA --mode visual|profile` reproduce el A/B mediante `EMOTIONAL_PERF=full EMOTIONAL_SCENE_CACHE=off|layers`; `base` conserva el experimento intermedio. Perfil sin capturas ni generación por foco: una preparación de cada composición en7pintados. Mac6,1% menos mediana, no extrapolar al LCD. Ensayos físicos A/B completados y retorno visible confirmado. Últimos agregados: raster76,146→65,402ms; intervalos kernel medianos83,395→66,716ms.39/36acciones manuales distintas: no benchmark idéntico ni60 FPS. Recorte de título alternativo señalado, pendiente de reproducir/corregir. Véase [PHASE15_REPORT.md](PHASE15_REPORT.md).

---

Ensayo físico2f0ed169 de un minuto: cierre correcto y ES visible confirmado; propietario nota más suavidad.60 FPS no medidos/alcanzados. Experimento EmbedForSoftwareRenderer descartado por cambios de tipografía y p95 peor; configuración original conservada.

Actualización XRGB: test65.536 pares alpha y tres buffers RGB640×480 idénticos al baseline. Perfil comparado de ruta fija Mac; nuevo ARM64 no probado físicamente. Ver PHASE14_REPORT.md.

Actualización tras ensayo de300 s: retorno visible a ES confirmado; ajuste local posterior de ritmo360/240 ms pasa8 pruebas motion y32 pasos. ARM64ec124ea0 no probado en LCD. Evidencias en PHASE14_REPORT.md.

# Fase 14 · comprobación vigente · 21-10-2026

Build release simulator y LinuxKMS AArch64 correctos; fmt/diff-check pasan. Primera integración: 256 Rust; 101 Python (99 pasan, 2 skips de arte privado históricos); navegación existente174 pasos. Corrección final: 7 pruebas motion (incluye anchura completa y bordes L/R) y replay32 pasos de retarget/reversión/wrap/modal/reducido pasan; no se repiten suites ajenas.

`tools/check_phase14_motion.py --mode regression|capture|profile --binary ELF_HOST --seed SQLITE_FIXTURE --fixtures RAIZ --out CARPETA_NUEVA` reproduce regresión, seis secuencias o perfil. `--baseline` compara el binario anterior en profile/capture. Capturas de buffer real Slint RGB640×480; no son evidencia LCD ni tiempos de presentación. Perfil sin codificación de PNG; métricas/caveats en [PHASE14_REPORT.md](PHASE14_REPORT.md).

Fallo encontrado: reducido mantenía cámara±1; se corrigió la condición, conservando la aserción pan=0. Las pasadas fallidas siguen en evidencia privada. Ensayo F14 f2193b4e completado con fluidez y retorno confirmados por el propietario. Candidato posterior1e635ac6 con viewport640 corregido: todavía NO PROBADO en LCD; regresión exige tamaño igual al viewport real del simulador.

---

Corrección posterior de cápsula: replay focalizado de 82 pasos pasa; macOS/ARM64 compilan. No repetir suite global por este enlace de foco. Prueba física pendiente.

# Fase 13 · resultados vigentes · 21-10-2026

- `cargo test --locked --workspace --features simulator`: 245 pasan.
- `python3 -m unittest discover -s tests -p 'test_*.py'` (runtime con PIL/tomllib): 101 casos, 99 pasan, dos skips explícitos de arte legado privado.
- `cargo build --locked --release -p emotional-ui --no-default-features --features simulator` y `tools/arm64/build.sh` con SDK existente y `EMOTIONAL_KMS_FEATURE=linuxkms-direct`: correctos. `cargo fmt --check` y `git diff --check`: correctos.
- `tools/check_phase13_navigation.py --binary RUTA --out CARPETA_NUEVA --seed SEED --fixture-root FIXTURES`: 160 pasos en tres replays, seed intacto. Fixtures propios de `tools/visual_pair.py`; no ROMs.
- `tools/capture_phase13.py --binary RUTA --seed SEED --rom-root FIXTURES --out CARPETA_NUEVA`: RGB Slint 640×480, casos de interfaz/foco/vacío/arte/título largo. `--cases` limita regeneraciones a casos afectados.
- Diagnóstico simulator optativo: `EMOTIONAL_PERF=full EMOTIONAL_PERF_TARGET=xrgb EMOTIONAL_PERF_VERIFY_SHADOW=1` compara RAM retenida contra repintado completo y tres copias RAM. Verificados 121 frames, sin certificación LCD.
- Diagnóstico físico `tools/profile_drm_memory.c`: buffer propio y ejecución acotada, sin master/framebuffer/pageflip. Mide la memoria, no FPS de la GUI. Para GUI hace falta autorización/supervisión/recuperación actuales.

Resultados, hashes y limitaciones en [PHASE13_REPORT](PHASE13_REPORT.md). Dos ensayos físicos del ELF f04c8668…: B mucho más rápido según propietario; retorno visible a ES confirmado. La corrección posterior de acceso lateral a cápsula aún no tiene prueba física. Los resultados inferiores son históricos.

---

# Revisión definitiva de diseño y navegación · 19-10-2026

Resultados de esta intervención; los informes inferiores son históricos. No se ha accedido ni modificado la RG353V.

| Comprobación | Resultado real |
|---|---|
| fmt / Clippy all-targets con -D warnings / test / build --locked | Pasan en macOS ARM64: 178 Rust; Docker ARM64: 180 Rust |
| Docker feature simulator | Clippy, test y build pasan; 181 Rust |
| Python existente | 71 pasan |
| Navegación final en Docker | 21 casos pasan, incluidas las cuatro rutas del dock, cápsula, vacío, foco entre secciones desde tercera partida, pulsaciones durante transición y movimiento reducido |
| Arte editorial | 12 casos pasan; cuatro modalidades, imágenes corruptas/ausentes, título largo, caché, tres juegos y worker retrasado |
| Revisión visual local | Dos vistas + dos focos de dock; diez casos adicionales, PNG RGB opacos 640×480 y revisión manual de composición, Biblioteca, Ajustes, cápsula, título largo y vacío |
| Material actual | Cinco PNG interiores: contraste conservador mínimo 4,508:1 sobre blanco/negro con tinte activo; no certifica el LCD |
| Compilación cruzada KMS directo | ELF64 AArch64 enlazado, auditoría de dependencias SDK pasa; compatibilidad física sin verificar |

Evidencia en `docs/visual-unification-2026-10-19`: `focus-verified-docker-build.log`, `final-{fmt,clippy,tests,build}.log`, `focus-final-navigation/report.json`, `verified-runtime/artwork/report.json`, `final-verified/{review,visual-review}.json`, `material-contrast.json`, `final-arm64-elf.json`. Capturas entregadas en el escritorio: `EmotionalUI-Revision-Definitiva-2026-10-19/Final`.

Se conservan fallos anteriores: esquinas con gradiente/SVG, anclaje inicial del encabezado, Clippy y expectativa antigua del retardo. El test de transición conserva la comprobación de aparición posterior y añade el tramo de espera; no se reduce el umbral de contraste. La regresión nueva exige restaurar el foco de cada sección tras partir de la tercera partida. Las pruebas de los antiguos destinos visibles se actualizan al dock de cuatro destinos; los tests puros anteriores permanecen.

No se ha repetido la suite externa completa histórica de 52 comprobaciones ajenas a esta revisión; se han ejecutado las suites de compilación/unidad y navegación/arte indicadas. No hay nuevos resultados de FPS, latencia física o audio en consola. Los indicadores de sistema permanecen sin verificar/deshabilitados, el arte de juegos y la actividad de las capturas son fixtures propios y falta aprobación visual humana. El linker cruzado mantiene su aviso no fatal de optimización de Zig obsoleta.

---

# Validación vigente · arte editorial y navegación · 19-10-2026

Implementación local y Docker terminada hasta los límites documentados en [ARTWORK_IMPLEMENTATION_REPORT.md](ARTWORK_IMPLEMENTATION_REPORT.md). **Esta revisión no se ha probado físicamente ni ha modificado dArkOS.** La evidencia física histórica de Fase 12 corresponde a su versión anterior.

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` y `cargo build --workspace --locked`: pasan en macOS ARM64 (178 Rust) y Linux ARM64 Docker (180 Rust). Feature simulator: Clippy/test/build pasan, 181 Rust. 71 Python existentes pasan.
- Runtime final: 52/52 comprobaciones externas, 19/19 de navegación y 12/12 de arte. Dos capturas adicionales comprueban el CTA principal enfocado en ambas vistas.
- 15 capturas de arte RGB opaco 640×480, casos antes/después y foco. CRC/descompresión, contenido y revisión visual. Gráficos y actividad de prueba sintéticos, no imágenes de juegos comerciales ni sesiones reales.
- Caché: miss/hit, persistencia entre procesos, invalidación por bytes, eliminación, corrupción, límites, pixel art, rutas/symlink fuera de raíz, temporales y limpieza conservadora. Regresión del alias /var en macOS preservada. Demora de 1,2 s del worker con controles activos; no emula una microSD.
- Composición fría 324,074 ms; hit disco 2,666 ms; memoria con validación de fuente 0,054 ms: medianas del benchmark dev Docker, no hardware. GUI ociosa ~76 MiB y 0,5–1 % de un núcleo en muestras cortas. No FPS ni latencia física.
- ELF LinuxKMS release AArch64 completo y dependencias comprobadas con el SDK. Compilar no certifica el LCD, audio, mandos o rendimiento de esta revisión.

Evidencia: `docs/editorial-artwork-2026-10-19/{docker-final-build.log,mac-tests.log,arm64-elf.json,docker-verified}`. Se conservan fallos previos (caché /var, foco antiguo y parada de panorama); la repetición final pasa sin eliminar comprobaciones. Reproducir con `platform/simulator/validate.sh` y después `platform/simulator/artwork_validate.sh CARPETA_NUEVA FIXTURES`; ver ARTWORK_RENDERING.md para generar fixtures.

---

# Fase 12 en curso · 19-10-2026

Hay ahora evidencia física de GUI 640×480, controles y retorno temporal a EmulationStation confirmados por el usuario; todavía **no** hay audio audible confirmado ni lanzamiento real de un juego. Véanse PHASE12_REPORT.md, PHASE12_AUDIO.md y PHASE12_LIBRARY.md. Las cifras de fase 11 que siguen son históricas.

Estado de código actual: fmt, Clippy con `-D warnings`, test y build pasan en macOS; 156 tests Rust (76 core / 38 servicios / 42 app), 67 Python con el runtime que incluye tomllib. Se genera otro ELF64 AArch64 completo con LinuxKMS directo; la auditoría del SDK no certifica su ejecución física. La vista gráfica de calibración reutiliza deliberadamente el ELF previamente probado, no presenta las últimas optimizaciones locales como validadas en consola.

La primera suite de ejecución Docker de esta continuación no pasa completa: foco esperado antiguo, disponibilidad de carátula a 900 ms e importación que no llega antes de la salida del test. Registros en docs/phase12/docker-runtime-audio-diagnostic. La comprobación de foco se amplía al recorrido nuevo, sin retirar las comprobaciones de biblioteca/menú. La repetición y cualquier límite de tiempo se documentarán aparte.

# Evidencia histórica · fase 11 · 19-10-2026

No se ha ejecutado este build en RG353V ni se ha lanzado un emulador real. Se conservan los informes siguientes como evidencia histórica.

| Comprobación ejecutada | Resultado final |
|---|---|
| cargo fmt --all -- --check | pasa, macOS y Docker ARM64 |
| cargo clippy --workspace --all-targets --locked -- -D warnings | pasa, ambos entornos |
| cargo test --workspace --locked | 151 macOS (75/37/39); 153 Linux por defecto (75/39/39) |
| cargo build --workspace --locked | pasa, ambos entornos |
| Feature simulator Linux | Clippy/build y 154 pruebas (75/39/40) |
| Python test_phase*_tools.py | 58 macOS y 58 Linux; conexiones SQLite cerradas explícitamente |
| Suite de ejecución Docker completa | 52 comprobaciones externas pasan; suites anidadas 40/14/10/16/11 y bibliotecas de cuatro tamaños |
| ELF release LinuxKMS directo | ELF64 AArch64 completo; auditoría SDK, CLI y rechazo esperado al faltar framebuffer en Docker |
| Capturas | renders reales RGB opacos 640×480; datos e historial de prueba, no ROMs |
| Candidato privado | dos paquetes byte a byte idénticos; no instalado ni aprobado para distribución pública |

No sumar las suites anidadas como si fueran pruebas unitarias independientes. Logs actuales: docs/phase11/{fmt,clippy,tests,build}-focus-final.log, docker-focus-final-build.log y python-closed-connections-{macos,linux}.log. Suite: docker-verified/report.json. Resumen y hashes: delivery-verification.json, arm64-verified-elf.json y package-verified-verification.json. Se mantienen los fallos anteriores y el aviso no fatal del linker Zig.

## Importación progresiva medida en Docker

Se lee XML real de fixtures de texto originales, no inserciones SQL que simulen importación. 100/1.000/5.000/10.000 archivos propios/hardlinks; dos procesos por tamaño. Frío sin caché de aplicación; caliente con firma XML y catálogo SQLite. Caché de SO no controlada, observación del padre cada 20 ms, pasadas únicas. El tiempo es hasta observar catálogo publicado, no encendido ni primer píxel. La sonda de event loop incluye 250 ms deliberados.

| Entradas | Catálogo frío ms | Catálogo caliente ms | Pico RSS muestreado MiB | Retraso máximo de timer ms |
|---|---|---|---|---|
| 100 | 438.1 | 463.9 | 76.7 | 324 |
| 1,000 | 451.1 | 518.1 | 80.0 | 319 |
| 5,000 | 630.8 | 787.4 | 87.6 | 389 |
| 10,000 | 1223.6 | 1257.8 | 94.3 | 420 |

Los ocho cambios de navegación y sus liberaciones se procesan en cada caso. Hay picos iniciales de timer superiores a 100 ms; no se declara respuesta visual inferior a ese objetivo. No son latencia pulsación-fotón ni FPS presentados. La primera lectura del catálogo cacheado y el primer rasterizado aún tienen coste. El modo estable conserva escena sin animación permanente; no hay nueva medición aislada de CPU en reposo ni del LCD.

Comparación descriptiva: antes de eliminar la búsqueda cuadrática/lectura duplicada, 10.000 entradas dieron 970,4 ms en frío y 1.828,4 ms en reapertura. La última pasada da 1223.6/1257.8 ms respectivamente. Son observaciones únicas con cachés de SO no controladas, no una prueba de causalidad ni mejora general. Antes: performance-focused/measurement/report.json; después: docker-verified/phase11-performance/report.json. Se conserva también docker-complete como pasada previa.

## Atlas, pipeline e instalación

21/52 fichas investigadas con interpretaciones propuestas; canal experimental opt-in, cero aprobaciones humanas inventadas. Dos de diez selecciones cambian, siete títulos distintos frente a cinco, con repetición persistente. Hoja humana sin puntuaciones en evaluation-review/human-review.csv. No se afirma mejor calidad humana.

Benchmark separado con los 52 perfiles reales experimentales repetidos sobre IDs locales sintéticos: a 10.000, ranking anterior 10,3119 ms, Atlas 11,9945 ms, selección de reserva 1,6615 ms y preparación 142,3817 ms (medianas de siete, Docker ARM64). La reserva reduce la selección puntual pero añade preparación y no resulta más barata en todos los tamaños. No mide partidas instaladas reales. Datos completos: atlas-performance-linux.json.

Pruebas nuevas: fuentes/estados de propuesta, opt-in, confianza, relaciones/IDs, reproducción SQLite, no autoaprobación, explicación y nombre con '=', biblioteca vacía/desconectada, PNG/JPEG ausente, no demos ni emuladores en trial, importación incremental, decisiones durante refresco, instalador idempotente, modelo incorrecto/dependencia ausente, disco, manifest/hash, symlink, conflictos, interrupción recuperable/parcial, restore y uninstall. Originales XML/fixtures se verifican por hash.

Un primer benchmark excedió el timeout exterior de 40 s porque ocho retenciones de cinco segundos requieren más tiempo con su preparación. Se corrigió solo el presupuesto exterior a 180 s. Las aserciones y límites por proceso se conservan. Una ruta de confirmación en modo trial fue corregida tras fallar el test gráfico; no se eliminaron pruebas. Python 3.13 mostró ResourceWarning de conexiones del generador/tests: se cerraron explícitamente, conservando el log anterior.

Reproducir: platform/simulator/validate.sh, esperar su cierre y después platform/simulator/run.sh CARPETA_NUEVA. Se reutilizan imágenes/volúmenes; sin red/dispositivos en ejecución y código montado solo lectura. No reemplazar binarios durante la suite. Compilación cruzada con tools/arm64/build.sh, Zig/SDK documentados. El paquete privado está en dist/phase11-verified/EmotionalUI-private-v1; no ejecutar cesión gráfica sin supervisión/recuperación independiente verificadas.

Pendientes: juicio editorial humano, perfilado del arranque inicial, entrada/retorno físico dArkOS, mandos/DRM/audio, un juego legal con guardados aislados, recuperación independiente y derechos/notices antes de publicar. El objetivo de instalación en diez minutos sigue sin medir.


Regresión gráfica final: insertar un título antes del juego reciente hacía saltar el foco de Continuar a Biblioteca. Reproducido en focus-before (fallo), corregido preservando el ID tras recalcular continuidad y verificado en focus-after y la suite completa: once casos nuevos pasan. No se eliminó la comprobación fallida.

---

# Verificación vigente · fase 10 · 2026-10-18

Evidencia actual separada de los informes históricos que siguen. **Sin acceso a RG353V, sin cambios en dArkOS, sin partida real.**

| Comprobación ejecutada | Resultado actual |
|---|---|
| `cargo fmt --all -- --check` | pasa, macOS y Docker ARM64 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pasa, ambos entornos |
| `cargo test --workspace --locked` | 142 macOS (74 núcleo/37 servicios/31 app); 144 Linux (74/39/31) |
| `cargo build --workspace --locked` | pasa, ambos entornos |
| Feature simulator Linux | Clippy/build y 145 tests pasan (74/39/32) |
| Python `test_phase*_tools.py` | 35 tests pasan en host y Docker |
| Suite Docker final | 50 checks externos; incluye suites anidadas 40/14/10/16 y 4 tamaños de biblioteca |
| ELF KMS cruzado release | 13.150.552 bytes; auditado, CLI ejecutada en Docker, gráfico rechazado al faltar dispositivos |
| Capturas | PNG RGB opacos/JPEG 640×480, inspección real de tres partidas, explicación y retorno |
| Atlas | JSON y SQLite reproducibles; 52 identidades, propuestas emocionales aún inactivas |

No sumar suites anidadas como tests unitarios independientes. Los ciclos usan procesos Python controlados y ficheros .fixture originales, no emuladores/ROMs. Las identidades enlazadas en el benchmark son deliberadamente sintéticas, no identificación real de dumps.

Logs finales: docs/phase10/{fmt,clippy,tests,build}-polish-verified.log y docker-build-polish-verified.log. Suite: docker-verified/report.json, phase10/summary.json y phase10-performance/report.json. Se conservan la primera suite pasada en docker-final y los fallos de compilación intermedios corregidos. Ninguna prueba anterior se eliminó. El aviso del linker Zig se mantiene en arm64-polish-verified.log; no impidió enlazar.

Regresiones añadidas: sesiones largas/cortas/arcade, regresos, pin, retirada, completado, reloj inválido/desconocido, guardado desconocido/confirmado como fixture, salida inesperada, vacíos, tres posiciones y foco; controles y persistencia de decisiones, corrección de afinidad, explicación, inicio/retorno de un proceso desde secundaria; Atlas readonly, enlaces explícitos, overrides locales, fuentes, estados de revisión, duplicados, relaciones, campos ausentes, versiones, SQLite reproducible y prohibición de autoaprobar resultados IA. Se mantienen audio, launcher, escenas, RGB/contraste y controles anteriores.

Medidas de cálculo: docs/phase10/performance-linux.json compara ranking anterior/con perfiles/con preferencia, preparación, selección pequeña, fingerprint, cache SQLite y clon para 100/1.000/5.000/10.000. La reserva no mejora todos los costes: a cien juegos su selección tarda más que ordenar directamente; preparar requiere trabajo previo. No se ejecuta ese cálculo al cambiar foco.

Medidas GUI finales con 10.000 entradas: sonda event_loop_alive 985 ms sin cache/878 ms con cache/1.028 ms tras cambiar disponibilidad; pico muestreado ~82 MiB. Son pasadas únicas con cachés de SO no controladas, incluyen un timer intencional de 250 ms y no representan encendido ni primer píxel. La memoización lee/parsea cada perfil distinto una vez durante la carga; para el fixture de 52 identidades son 53 SELECT previstos, no una cuenta de I/O de disco. Primera pasada antes de memoizar conservada para comparación, sin afirmar causalidad estadística.

La navegación del benchmark conserva los ocho focos programados por ejecución. Máximo retraso de timer observado en el fixture grande: 72 ms; **no es latencia de mando ni pulsación-fotón**. FPS presentados/latencia LCD permanecen desconocidos. Escaneo XML completo y actualización física de una biblioteca no se miden con el contador de inserción SQL del fixture. Tampoco se mide un emulador/ALSA real.

La evaluación editorial automática de diez escenarios coincide con el ranking anterior porque hay cero perfiles editoriales SNES revisados activos. No se presenta como mejora de calidad humana. Los campos de manual-review.csv siguen vacíos; ver RECOMMENDATION_EVALUATION.md. El piloto no completa SNES.

Reproducir: esperar cierre de `platform/simulator/validate.sh` y después `platform/simulator/run.sh CARPETA_NUEVA`, sin reemplazar binarios en ejecución. Se reutilizan imágenes/volúmenes ARM64; runtime sin red, sin dispositivos, código en bind solo lectura. Las pruebas nuevas se integran en los scripts existentes. Atlas: `python3 tools/atlas.py validate FUENTE.json` y `build FUENTE.json --output BASE_NUEVA.db`; no reconstruye Atlas durante el arranque de GUI. Toolchain cruzado previo en ARM64_BUILD.md; último ELF en dist/phase10-verified/emotional-ui. `cross-runtime.json` solo confirma CLI/cierre sin dispositivo en Debian ARM64.

Próximas validaciones: revisión de datos/selecciones por una persona y luego una prueba física supervisada con core/rutas/guardados aislados verificados. El procedimiento actualizado está en GAME_LAUNCHER.md; no ejecutar el watchdog antiguo de tiempo fijo con un juego activo.

---

# Evidencia histórica · fase 9 · 2026-10-18

Esta sección es la evidencia actual; las secciones posteriores se conservan como historial. **Sin SSH, sin cambios en dArkOS y sin juego/emulador real en esta fase.**

| Comprobación ejecutada | Resultado |
|---|---|
| `cargo fmt --all -- --check` | pasa, macOS y Docker ARM64 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pasa, ambos entornos |
| `cargo test --workspace --locked` | 124 macOS (61/37/26), 126 Linux (61/39/26) |
| `cargo build --workspace --locked` | pasa, ambos entornos |
| Feature simulator | Clippy/build y 127 tests pasan en Docker (61/39/27) |
| Python `test_phase*_tools.py` | 25 pruebas pasan en host y Docker |
| Frontend LinuxKMS directo release | ELF64 AArch64 completo, 12.763.200 bytes; SDK de referencia auditado, hardware pendiente |
| Música | dos candidatas MIDI/WAV/Ogg/M4A reales; seis preescuchas decodificadas sin errores |
| Producción | WAV/MIDI reproducidos byte a byte en el mismo host; muestras originales CC0 con checksums |

Logs: docs/phase9/{fmt,clippy,tests,build}-final.log, python-verified.log, docker-build-complete.log y arm64-build.log. Inventario ELF: arm64-elf.json. El aviso de linker Zig sobre optimización obsoleta se conserva y no impidió enlazar; Clippy host/Docker pasó con warnings prohibidos.

Nuevas regresiones: perfiles/argumentos seguros, texto ejecutable sin shebang rechazado, ambigüedad de raíces, ROM ausente, historial de procesos separado de actividad fechada, reloj no confirmado, errores sin partida registrada y borrado de historial conservando preferencias. Recursos musicales con PCM válido, picos sin clipping, silencios, MIDI reproducible, procedencia y exports reales. Se mantienen todas las pruebas heredadas.

El ciclo integrado ejecuta la GUI real, cierra su proceso antes de iniciar un programa Python controlado, espera su resultado y abre otra GUI. Nunca sustituye ese programa por RetroArch en la descripción de resultados. Los fixtures son archivos propios; se verifican hashes del XML y de un guardado sintético, ausencia de ejecución de comandos XML y argv literal con caracteres de shell. La doble confirmación, cancelación con B, bloqueo de dos supervisores y retorno sin ROM/raíz tienen comprobación específica. Resultados finales de ejecución más abajo, en el cierre de fase 9.

Banco de audio candidato: 3.616.200 bytes PCM. Sonda muda temporizada del host: cesión 206,542 ms, dos aperturas/dos cierres, sin solapamiento con proceso simulado; 5.120.000 frames de mezcla offline en 935,093 ms. No mide audibilidad, ALSA real ni latencia pulsación-fotón. Picos de preescucha A/B 0,1294/0,0890 FS; silencios 61,66/66,34 %. El banco anterior sigue predeterminado hasta revisión humana.

Benchmark Docker previo a la ejecución gráfica, sin compilación concurrente: ranking mediano 1.000/5.000/10.000 juegos en 1,1130/5,3363/10,8857 ms; carga SQLite con clon 2,7317/13,8268/27,3816 ms. Son tiempos de userspace en VM, no RK3566; ver catalog-performance-linux.json. No se alteró el recomendador para esta fase.

Reproducción: ejecutar `platform/simulator/validate.sh` hasta su salida correcta y después `platform/simulator/run.sh CARPETA_NUEVA`. No reemplazar binarios en uso. El runtime reutilizado es ARM64, sin red, sin dispositivos físicos y con bind del código de solo lectura. El build cruzado KMS se obtiene con tools/arm64/build.sh y el SDK/Zig previos; véase ARM64_BUILD.md. La producción musical necesita NumPy y FFmpeg de desarrollo, no del frontend.

Pendiente: escucha artística A/B, audio musical nativo de estas candidatas, resolución de perfiles físicos, DRM/evdev/ALSA y retorno de un juego en RG353V. Los wrappers deben esperar al juego completo; no se garantiza seguimiento de scripts daemonizados ni recuperación física automática. No usar un watchdog fijo que restituya EmulationStation con el juego abierto. Véanse GAME_LAUNCHER.md y LAUNCH_LIFECYCLE.md.

## Cierre verificado de fase 9

Ejecución definitiva: `docs/phase9/docker-final/report.json`: **48 comprobaciones externas pasan**, incluyendo las suites conservadas fase 7 (**40**) y fase 8 (**14**), y la nueva fase 9 (**10**). Son comprobaciones anidadas, no cantidades que sumar como tests unitarios. Los diez casos nuevos: retorno normal/Silencio con rechazo de otro supervisor, reloj desconocido, salida con error, salida demasiado rápida, ejecutable inválido, ROM ausente, lanzador no autorizado, cancelación antes de spawn, biblioteca vacía y raíz desconectada al regresar. En todos se conservaron los bytes del XML/guardado sintético y no se ejecutó la etiqueta command.

Ambos bancos nuevos se cargaron también en el audio-probe ARM64: 3.616.200 bytes PCM, dos aperturas y dos cierres por prueba, sin solapamiento simulado. Cesión A/B 200,455/171,686 ms; mezcla offline de 5.120.000 frames 669,992/668,051 ms. Son una salida muda temporizada y un bucle offline; no prueban latencia, sonido físico ni CPU de reproducción real (`audio-candidates-docker.json`).

| Medida Docker debug | Observación y límite |
|---|---|
| GUI inactiva | 75.432 KiB RSS; ~0,50 % de un núcleo durante muestras de ~2 s, Silencio/Minimalista antes de la primera frase |
| Frames rasterizados en reposo | 2 en cada ejecución de 4 s; no equivalen a FPS presentados |
| Supervisor mientras corre fixture | 8.504 KiB RSS; CPU redondea a 0,0 % en una muestra de 0,260 s; no prueba consumo nulo |
| Panorama entre vistas | 240 ms declarados; destino observado a los 324 ms desde cambio observado, límite superior por muestreo del event loop debug |
| Ticket de cesión en GUI simulator | observado en 63,889–95,095 ms; usa salida no disponible, **no mide el fundido del dispositivo** |
| Ranking 1.000 / 5.000 / 10.000 juegos | 1,1130 / 5,3363 / 10,8857 ms, mediana de benchmark local ARM64 |
| FPS LCD / pulsación-fotón | sin medir |

El ELF cruzado release también ejecutó --help dentro de Debian ARM64. Su inicio gráfico sin /dev/dri ni framebuffer falló de forma controlada, como corresponde; el log conserva avisos Fontconfig por caché no escribible del contenedor. No es una prueba de compatibilidad con pantalla ni ABI instalada de dArkOS (`cross-runtime.json`).

Capturas de cierre: `docs/phase9/final-captures/{continuar,retorno-emotional,raiz-desconectada}.{png,jpg}`. PNG RGB opacos, 640 × 480, no vacíos; se abrieron e inspeccionaron los tres renders. Copias en `Escritorio/EmotionalUI-Fase9/Capturas`. Continuar usa una partida de muestra rotulada; retorno utiliza la biblioteca sintética del test. No se presentan como fotos del dispositivo. Prueba de conservación de Slint en visual-preservation.json. Las dos composiciones y esta revisión no se consideran aceptadas por el usuario por haber pasado pruebas técnicas.

Comandos ejecutados: fmt/Clippy/test/build con --locked, validación Docker reutilizada con `platform/simulator/validate.sh`, seguida secuencialmente de `platform/simulator/run.sh docs/phase9/docker-final`; build cruzado con `EMOTIONAL_KMS_FEATURE=linuxkms-direct tools/arm64/build.sh` y Zig/SDK fijados previamente, auditado con tools/arm64/verify_elf.py. Probes adicionales Docker invocan dist/simulator/audio-probe con cada --assets y el ELF cruzado sin dispositivos. Logs, perfiles y replays completos están conservados; no se ejecutaron pruebas físicas.

---

# Evidencia histórica · fase 8 · 2026-10-18

Se conserva la habitación lofi y la arquitectura anterior. Resultados de esta revisión separados de la evidencia histórica que aparece más abajo. **No se ha ejecutado este build en la RG353V ni se ha modificado dArkOS.**

| Comprobación ejecutada | Resultado observado |
|---|---|
| `cargo fmt --all -- --check` | pasa en macOS y Docker ARM64 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pasa en macOS y Docker ARM64 |
| `cargo test --workspace --locked` | 117 macOS (61 núcleo + 33 servicios + 23 app); 119 Linux (61 + 35 + 23) |
| `cargo build --workspace --locked` | pasa en ambos entornos |
| Feature simulator Linux ARM64 | Clippy/build y 120 tests pasan (61 + 35 + 24) |
| Python `test_phase*_tools.py` | 22 pruebas pasan en host y Docker; incluye reproducción byte a byte de WAV/MIDI |
| Suite Docker completa | 47 comprobaciones externas pasan; contiene fase 7 con 40 y fase 8 con 14 comprobaciones propias |
| Simulator release cruzado en Docker | 12 comprobaciones UI de fase 8 pasan; las dos sondas de audio se ejecutan por separado en la suite debug |
| Compilación LinuxKMS ARM64 release | ELF completo enlazado, 12.553.696 bytes, auditado contra SDK; no ejecutado en consola |
| Salida nativa macOS | abre, escribe cuatro bloques de PCM **cero**, drena y cierra; audibilidad musical no comprobada |
| Salida nativa en contenedor sin ALSA | ausencia de biblioteca detectada; fallback a silencio, salida controlada |
| Preescucha | MIDI fuente, WAV, Ogg Vorbis y AAC/M4A reales; exportación reproducida y Ogg decodificado |
| Capturas | seis renders RGB opacos a 640 × 480, JPEG adicionales, inspección visual de ambas vistas y Sonido |

No sumar las comprobaciones anidadas como si fueran tests unitarios independientes. Los ejemplos con salida Probe mezclan PCM real y simulan su tiempo sin sonido; `/usr/bin/true` es un proceso sintético, no RetroArch.

## Evidencia y regresiones

Logs finales del host: `docs/phase8/{fmt,clippy,tests,build}-verified.log` y `python-verified.log`. Validación Docker: `docker-build-complete.log`. Ejecución completa: `docker-final/report.json`, `phase7/summary.json`, `phase8/summary.json`; release: `docker-release-final/phase8/summary.json`. El entorno Docker ARM64, imágenes y volúmenes anteriores se reutilizaron sin exponer DRM, evdev ni audio físicos.

Nuevas regresiones: modos/silencio y volúmenes; recursos ausentes o WAV inválido; contexto que conserva una frase activa; caída de dispositivo durante escritura, cierre y ausencia de reintentos ocupados; cesión exclusiva y ticket obsoleto; retorno con preferencias actuales; persistencia SQLite y borrado de historial que conserva ajustes; controles durante la transición y paneles, L/R, foco retenido, movimiento reducido, movimiento ambiental desactivado, catálogos vacíos y capturas opacas. Se preservan las pruebas anteriores de input/editorial/importación/biblioteca.

Se detectó y corrigió una animación que saltaba al destino porque la duración del binding se evaluó a cero en la inicialización de Slint. El test final observa posiciones intermedias. La medición empieza tras calentar el primer render: los eventos de arranque pueden ser directos y no deben esperar una introducción. Se registran tiempos programados y observados para no confundir backlog con tiempo real.

Se preservan los intentos fallidos: error inicial de animación, suposiciones incorrectas del nuevo test sobre alternancia L/R y foco retenido, y una ejecución concurrente que rechazó reemplazar un binario en uso (`Text file busy`). La validación definitiva se ejecutó en orden, sin compilaciones concurrentes. No se eliminó ninguna prueba heredada.

## Medidas locales, sin equivalencia con la consola

| Medida | Resultado y alcance |
|---|---|
| Recursos PCM de ejecución | 3.087.000 bytes decodificados; no incluye binario, copias WAV incluidas ni RSS de la app |
| Preescucha de 100 segundos | 65,315 % de muestras cero, pico 5.207/32.767 |
| Cesión simulada, Docker | 192,215 ms hasta salida cerrada; dos aperturas/dos cierres; sin solapamiento con el proceso sintético |
| Mezcla de 5.120.000 muestras, debug Docker | 689,411 ms de bucle offline; no mide CPU de reproducción/dispositivo |
| Panorama nominal | 240 ms declarados; destino observado a los 325 ms en debug y 253 ms en release; límites superiores por muestreo |
| RSS idle debug | 75.212–75.224 KiB; muestras de dos segundos; 2–3 frames rasterizados durante los cuatro segundos de ejecución |
| RSS idle release | 67.800–67.824 KiB; dos frames rasterizados en cuatro segundos |
| CPU idle | redondeó a 0,0 % en muestras de unos dos segundos, Silencio/Minimalista antes de su primera frase; no prueba consumo nulo |
| FPS presentados / latencia pulsación-fotón | sin medir; Docker no tiene LCD ni presentación DRM de la RG353V |

Datos sin redondear: `docker-final/phase8/{pan-timing,idle-performance,audio-lifecycle}.json`, `docker-release-final/phase8/{pan-timing,idle-performance}.json`, `audio-lifecycle-macos.json`, `native-silent.json`, `audio-asset-metrics.json`. Contraste conservador de Sonido sobre fondo blanco: texto 8,60:1, secundario 5,89:1, borde de foco 5,96:1 (`audio-panel-contrast.json`); no certifica panel físico ni transitorios.

El benchmark existente se volvió a ejecutar con 1.000/5.000/10.000 juegos; inventario inmutable en `docs/phase8/catalog-performance-linux.json`. Las medianas de ranking de esa pasada fueron 1,09/5,59/11,06 ms. La música no se añadió al ranking ni se reescribieron sus algoritmos.

## Reproducir

Con el Docker ya configurado: `platform/simulator/validate.sh`, esperar su cierre correcto, después `platform/simulator/run.sh CARPETA_NUEVA`. Contexto `colima-emotional-ui`; no compilar/reemplazar artefactos mientras se están ejecutando. El builder usa el bind del proyecto de solo lectura y volúmenes previos; el runtime no tiene red ni dispositivos del host. Capturas definitivas en `docs/phase8/final-captures/` y copia en `Escritorio/EmotionalUI-Fase8/capturas/`.

Para release optimizado se reutilizó `tools/arm64/build.sh` con `EMOTIONAL_KMS_FEATURE=simulator`, se copió el ELF a `dist/phase8-cross/emotional-ui-simulator` y después se ejecutó `platform/simulator/phase8-release.sh CARPETA_NUEVA`. Variante de consola: `EMOTIONAL_KMS_FEATURE=linuxkms-direct`, ELF `dist/phase8-cross/emotional-ui`; ambos hashes e inventarios en `docs/phase8/arm64*-elf.json`. El aviso conocido de linker Zig por la opción -O1 está preservado. No es un paquete instalado en dArkOS.

Audio: `python3 tools/compose_audio.py --output-root CARPETA_NUEVA`; `tools/export_audio.sh CARPETA_NUEVA` (FFmpeg instalado con libvorbis); `cargo run -p emotional-services --example audio_probe --locked` (salida muda temporizada), y `-- --native-silent` para escribir exclusivamente ceros por la API nativa. La escucha artística, salida física ALSA, volumen/calidad de altavoces, presentación y regreso de un emulador real siguen pendientes. Continuar todavía no lanza un juego; la cesión es una interfaz preparada y comprobada por simulación.

---

# Verificación vigente — corrección visual de la referencia original

Fecha: 18-10-2026. Esta sección describe la revisión actual. Los apartados posteriores son evidencia histórica de las fases anteriores.

## Resultado observado

| Comprobación ejecutada | Resultado real |
|---|---|
| `cargo fmt --all -- --check` | pasa |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pasa, macOS y Docker ARM64 |
| `cargo test --workspace --locked` | 96 pruebas en macOS; 98 en Linux ARM64 con backend de escritorio |
| `cargo build --workspace --locked` | pasa, macOS y Linux ARM64 |
| Feature simulator en Docker ARM64 | Clippy, build y 99 pruebas pasan |
| Herramientas Python | 19 pruebas pasan con Python del runtime y en Docker |
| Suite de ejecución Docker | 46 comprobaciones externas pasan; contiene una suite integrada con 40 comprobaciones propias |
| ELF LinuxKMS directo | enlazado AArch64; inventario dinámico y versión en dist/simulator |
| Renderizado sin DRM en contenedor | software real; el binario KMS rechaza correctamente la ausencia de dispositivo |
| Hardware RG353V de esta revisión | **no ejecutado** |

Registros completos: `docs/visual-correction/{fmt-final,clippy-final,tests-final,build-workspace,docker-validation-final,python-final}.log`. Ejecución: `docs/visual-correction/docker-run-verified/report.json`, subcarpeta `phase7/summary.json`. No se suman las dos suites como si fueran pruebas unitarias independientes.

## Interacción y aspecto

Se inspeccionaron capturas reales de Continuar, EmotionalUI y foco sobre el acceso a EmotionalUI después de cerrar un menú. El nuevo recorrido incluye estados sin historial y catálogos de 0/1/2/6 juegos, cambios rápidos L/R, cruceta, confirmación, retorno de paneles, personalización y movimiento reducido. Se preservaron las pruebas de la interfaz anterior mediante modo legacy y los prototipos archivados.

Nuevas regresiones: primera confirmación en Resume sin historial; foco del acceso de descubrimiento y su conservación; direcciones que omiten alternativas inexistentes; exportación Retina rechazada antes de escribir; PNG nativo RGB opaco. Se mantienen captura con alfa cero, importación no destructiva, editorial, persistencia, repetición de controles y escenarios.

`final/continuar.png` y `final/emotional-ui.png` en `docs/visual-correction`: **640 × 480 físicos, RGB opaco**, no mockups de imagegen. Comprobación de dimensiones, modo, contenido no uniforme y hashes en `final/captures.json`. El fondo es un recurso artístico adaptado de la referencia, incorporado después al renderizador real. Se conserva un intento Retina de 1280 × 960 fuera de la entrega nativa, en `retina/`.

Contraste declarado conservador: principal 8.97:1, secundario 6.14:1, foco 6.22:1 y botón 9.43:1. `docs/visual-correction/contrast.json`. No valida calibración/contraste del panel físico ni conformidad completa WCAG.

## Rendimiento: alcance

La muestra final `docker-run-verified/phase7/performance.json` observó 67.948 KiB de RSS, 2 fotogramas rasterizados durante una escena estable y creación de ventana a los 19 ms del diario. La sonda de evento añade deliberadamente 250 ms, por lo que su tiempo no es el arranque visible. CPU en reposo se muestreó durante aproximadamente dos segundos y puede redondear a cero: no prueba consumo nulo.

FPS presentados y latencia pulsación-fotón permanecen sin medir. Docker no reproduce el RK3566 ni el LCD. El benchmark 1.000/5.000/10.000 se conserva en `dist/simulator/performance-phase7.json`; separa ranking, carga SQLite, consulta de historial, proyección visible y encolado/flush de escrituras. Las optimizaciones previas se conservaron.

## Reproducción y límites

`tools/capture-reference.sh CARPETA_NUEVA` compila el simulador, produce ambos renders al atardecer y se niega a sobrescribir una carpeta existente. Se ejecutó y comparó su salida con los archivos finales en `docs/visual-correction/reproduction.json`. Los juegos, historial y contexto son ficticios/simulados; el original del usuario no se copia con sus carátulas comerciales.

Docker: `platform/simulator/validate.sh`, después `platform/simulator/run.sh CARPETA_NUEVA`, usando el contexto existente colima-emotional-ui. No se reconstruyeron imágenes existentes ni expusieron dispositivos físicos. El frontend actual no se desplegó ni ejecutó en la consola.

Continuar aún no lanza un emulador real, y el ciclo de regreso/historial automático sigue pendiente. No se modificaron arranque, servicios, ROMs, partidas ni configuraciones críticas.

---

# Verificación · fase 5 Docker · 2026-10-18

## Corrección posterior: capturas transparentes

La regresión posterior pasa: Clippy en macOS, prueba específica de exportación, 72 tests Rust Linux normales, 73 con simulator, 15 Python y 45 comprobaciones Docker, incluidas opacidad y dimensiones. Evidencias: [validación Linux](docs/simulator/alpha-fix/linux-validation.log), [batería gráfica](docs/simulator/alpha-fix/docker-runtime/report.json), [captura opaca](docs/simulator/alpha-fix/docker-runtime/halloween.png).

La revisión del PNG al abrirlo en un visor convencional detectó que los 307.200 píxeles tenían alfa 0. Los canales RGB sí contenían la interfaz; la herramienta de inspección mostraba esos colores, y las comprobaciones anteriores solo validaban dimensiones. Por tanto, los resultados anteriores no probaban visibilidad en un visor normal.

Se corrigió la exportación de la ventana opaca para guardar RGB sin canal alfa (`app/src/capture.rs`). Una prueba de roundtrip verifica colores y opacidad partiendo de píxeles RGBA con alfa cero; las baterías de capturas exigen PNG RGB. Se conservan los originales y el diagnóstico en `docs/simulator/alpha-fix/`. La imagen nueva del escritorio, EmotionalUI-corregida.png, tiene alfa efectivo 255 y 3.635 colores distintos. También se exportó EmotionalUI-visible.jpg y se corrigieron las 13 copias del escritorio.

Esta corrección afecta a la exportación de imágenes, no demuestra un fallo de presentación de la ventana en pantalla ni constituye una prueba nueva de hardware. Los resultados de la regresión posterior están en esa carpeta; los informes de run-003 permanecen como evidencia histórica anterior a la corrección.

La entrega actual está comprobada en macOS ARM64 y Docker Linux ARM64. No se ha ejecutado este build en la RG353V ni se han hecho cambios en ella. Los informes de fase 3 se conservan como historial; las observaciones físicas anteriores no validan estos controles ni estos artefactos.

| Comprobación ejecutada | Resultado |
|---|---|
| `cargo fmt --all -- --check` | Pasa en host y Docker |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pasa en host y Docker |
| `cargo test --workspace --locked` | 69 macOS; 71 Linux |
| `cargo build --workspace --locked` | Pasa en host y Docker |
| Variante `--no-default-features --features emotional-ui/simulator` | Clippy, build y 72 tests Linux pasan |
| Herramientas Python | 15 tests pasan, sin eliminar los anteriores |
| Contraste | Pasa; mínimo conservador 4,57:1 |
| Gráficos macOS | UI 6/0/1/2, cuatro paletas, seis estados auxiliares, errores y cierre externo |
| Gráficos Docker | Renderer Slint software, cuatro paletas y seis escenarios a 640 × 480 |
| Entrada simulada | 115 pasos de botones, 111 de teclado, ejes sintéticos y 12 pasos de contexto |
| Importación → SQLite → GUI | 12 juegos ficticios, foco en fila 9, retorno desde menú, hashes originales intactos |
| ELF ARM64 y CLI KMS en contenedor | Pasa; el inicio gráfico KMS falla explícitamente por ausencia de dispositivos |
| LCD / botones / táctil / recuperación de este build | Pendiente de prueba física |

Linux añade dos pruebas del lector evdev: ABI de input_event y rechazo de archivos ordinarios. Simulator añade validación de escenarios. Los tests del núcleo (37) permanecen independientes del renderizador. No se ejecuta EmulationStation dentro del contenedor ni se le ofrece un dispositivo gráfico falso.

## Reproducir

```sh
./platform/simulator/validate.sh
./platform/simulator/run.sh
EMOTIONAL_UI_GRAPHICAL_TESTS=1 ./tests/verify.sh
```

Los dos primeros usan el contexto Docker existente; el tercero exige un escritorio nativo. Detalles y configuración: [SIMULATOR.md](SIMULATOR.md).

Logs: [Linux completo](docs/simulator/linux-validation-final-success.log), [macOS completo](docs/simulator/macos-final.log), [tests Python](docs/simulator/python-tests-final.log). Batería final: [report.json](docs/simulator/run-003/report.json), [entorno y hashes](docs/simulator/run-003/environment.json). La ejecución anterior satisfactoria permanece en run-002; run-001 conserva los fallos de lectura iCloud. Los logs anteriores también conservan el fallo inicial de importación Rgb8Pixel, calendario sin reglas y caché incompleto cuando corresponde al comando.

Capturas finales: [Despertar](docs/simulator/run-003/despertar.png), [Horizonte](docs/simulator/run-003/horizonte.png), [Refugio](docs/simulator/run-003/refugio.png), [Silencio](docs/simulator/run-003/silencio.png), seis escenarios con sus nombres, hora desconocida, clima caducado y [biblioteca importada](docs/simulator/run-003/imported-library.png). Son PNG de la aplicación, no ilustraciones fabricadas para representar una captura.

## Alcance de la evidencia

El host usa fuentes macOS y Docker DejaVu; sus diferencias tipográficas no son una medida de rendimiento o de mejora visual. Los tests no certifican accesibilidad del LCD. No se miden FPS presentados, input-to-photon ni rendimiento RK3566. Objetivos pendientes: 60 FPS, respuesta <100 ms y RAM contenida.

La adaptación de biblioteca comprueba disponibilidad al cargar, preservación del XML y los archivos sintéticos, favoritos, metadatos neutrales y preferencias independientes. No implica lanzamiento real, savestates ni vigilancia de cambios posterior a la carga. Un futuro lanzamiento deberá revalidar el archivo.

La hora simulada es civil, no reproduce DST ni la zona del sistema de destino. Festividades usan reglas TOML regionales. Clima disponible/caducado/futuro se prueba localmente sin HTTP. La puesta en hora por botones y NTP integrado continúan pendientes.

Historial completo anterior: [TESTING de fase 3](docs/simulator/history/TESTING-phase3.md), [informe fase 3](PHASE3_REPORT.md), [auditoría fase 2](AUDIT_REPORT.md). La confirmación humana histórica de imagen y restauración está en docs/phase5/human-confirmation.json y corresponde exclusivamente al hash allí registrado.

## Cierre de esta continuación de Fase 12

Registro final: docs/phase12/docker-runtime-calibration-final/report.json, 52 comprobaciones externas correctas, suites anidadas sin sumar. Rust 156 macOS / 158 Linux / 159 simulator; 71 Python en macOS y Linux ARM64. fmt, Clippy/build correctos y ELF release ARM64 auditado. Fuentes originales y banco normal conservados; banco diagnóstico reproducido byte por byte.

La carátula se valida mediante local_art_ready dentro del plazo original de proceso de tres segundos; catálogo/controles a 900 ms se conservan. Las pasadas fallidas anteriores permanecen. El usuario se marcha: no hay escucha del banco calibrado confirmada dentro de EmotionalUI, no se repite automáticamente ni se lanza un emulador. ES activo; las pruebas físicas quedan detenidas.

# Tarea activa · Fase 16 — First Public Preview

Distribución, seguridad, integración y documentación de v0.1.0-preview.1. Sin nuevas funciones ni optimizaciones. Software predeterminado, Mali-G52 experimental; no swap inmediato por defecto. Código local conservado; README remotos preservados. Propietario confirma Foto 1 propia; procedencia despejada para preparar el paquete. Orden de publicación recibida: avisos y selección revisados; falta decisión específica de visibilidad PRIVATE→PUBLIC (ya autorizada y realizada) y prueba física del ZIP exacto. Instalador gráfico preparado localmente, pero ZIP exacto/instalación/audio/desinstalación físicos todavía no probados. Hay autorización expresa de commit/push/tag/pre-release tras superar las verificaciones; presencia y autorización gráfica actual siguen pendientes. [Preparación y puertas](RELEASE_READINESS.md).

La investigación de pipeline queda pausada, no es un requisito para esta preview y no autoriza más ensayos. La orden vigente autoriza commit/push/Release verificados, sin nueva aprobación de publicación. No cambiar visibilidad sin decisión específica ni habilitar install-main.

---

## Estado anterior conservado — no es la tarea activa

# Tarea activa · aislar capacidad mínima del pipeline GPU

Publicación pausada por la nueva petición. Nine-slice-temporal por repetición descartada mediante inspección y tests de la versión real; no crearB2. PruebaA mínima preparada y pendiente de supervisión/autorización nueva; no tocar consola mientras el propietario esté ausente. SegúnA, cerrar o medir una tarjetaC. No abrir otra investigación, cambiar VSync o activar caché global. Software predeterminado/GPUexperimental se conservan. [Puertas y evidencia](GPU_RENDER_PIPELINE_AUDIT.md).

---

# Fase 15.3 · tres ensayos cerrados; GPU sigue experimental

Dos intentos como máximo. Trazas por etapa y experimentoA de esperaEGL preparados; ARM64/loader pasan y software se conserva. Tres ensayos autorizados completados y ESvisible confirmado. GPUreferencia19,84Hz frente a22,00Hz enA; p95~100ms sin mejora, no30FPS estables. RAMsoftware83,06MiB frente a156,20/156,74MiBGPU. IntentoA sólo optativo; no hay segundo intento seleccionado ni nuevas pruebas autorizadas. Usuario siente la tercera bien, sin afirmar30/60FPS. Software por defecto y GPUexperimental; estabilidad prolongada/manual y derechos de distribución pendientes. No iniciar tercer intento ni publicar. Recorte secundario corregido por overflow y validado en nueve casos locales; GPU/LCD pendiente, fuentes y geometría conservadas. [Estado y evidencias](GPU_ACCELERATION.md).

---

# Fase 15.2 · GPU experimental medida; software conservado

Mali propietario g29p1 y FemtoVG/Slint1.13.1 funcionan en RG353V. Dos rutas automáticas idénticas de13 acciones: cadencia agrupada de animación13,015→21,602Hz, mediana66,726→33,376ms; sin30/60FPS sostenidos. Prototipo y cuatro vistas capturados; usuario no apreció diferencias, pero un título cambia de salto de línea. Recuperación por PID1 y dos SSH conservadas. Software por defecto;09f71dfb GPU experimental. Falta valoración humana final y confirmación visible de ES, tipografía estricta, memoria software comparable y estabilidad prolongada. No abrir otra fase ni publicar. [Decisiones y límites](GPU_ACCELERATION.md).

---

# Fase 15 · optimización parcial validada; cierre visual pendiente

Dos composiciones estáticas acotadas implementadas, sin cambiar fuentes, apariencia ni movimiento360/240ms. Ganancia local modesta,18capturas pixelidénticas y ARM64 listo. Instrumentación de pageflips reales añadida; no60 FPS certificados. Ensayos A/B terminados: raster76,146→65,402ms e intervalo kernel83,395→66,716ms; recorridos manuales distintos. Usuario confirma fluidez y retorno visible a ES, detecta recorte del título alternativo. Siguiente tarea mínima: reproducir/corregir ese recorte.60 FPS y consumo de reposo B siguen pendientes de perfil específico. Sin commit/push ni nuevas funciones. [Informe](PHASE15_REPORT.md).

---

Estado más reciente:2f0ed169 validado un minuto; usuario confirma más suavidad y pide60 FPS. Mantener ritmo360/240 y apariencia. Siguiente tarea única: evaluar caché de composiciones estáticas para reducir coste de raster; objetivo60 pendiente, sin nuevos efectos ni ensayos automáticos.

Siguiente revisión acotada: medir en RG353V el candidato XRGB optimizado (mismo ritmo360/240 ms), tras nueva autorización. El propietario confirma mejor ritmo del ec124ea0; duplicar FPS sigue sin demostrar.

Siguiente revisión mínima: valorar en LCD el ritmo local360/240 ms (ELFec124ea0), solicitado tras el ensayo de cinco minutos del1e635ac6. Ensayo previo cerrado con retorno visible confirmado; sin nueva prueba automática.

# Tarea activa · Fase 14 · revisión física pendiente

Movimiento finito e interrumpible, profundidad ligera y composición editorial diferenciada implementados. Primer ensayo físico: fluidez confirmada, defecto de entrada interior detectado. Corrección local a640 px, secuencias y ARM64 preparados. No iniciar nuevas funciones. Siguiente puerta: ensayo temporal autorizado del ELF corregido `1e635ac6…` en RG353V; comprobar continuidad/profundidad, inversión, menús, cuatro secciones y fluidez conservada, seguido de retorno visible a ES. Usuario decide aceptación artística. El perfil local no sustituye esa validación; ver [PHASE14_REPORT.md](PHASE14_REPORT.md).

---

# Hito activo · Fase 13 — cerrar UX y lag · 21-10-2026

La petición actual sustituye las decisiones de navegación históricas: pestañas superiores Continuar · Tu momento · Biblioteca · Ajustes, L/R circular, un único modal y foco persistente.

1. Navegación: implementada; 160 pasos de replay y siete tests nuevos pasan. Falta confirmación física de esta revisión.
2. Rendimiento: penalización de composición en mmap demostrada con buffer propio en RG353V; backend RAM+copia implementado y compilado. Dos ensayos físicos completados: propietario confirma B claramente más rápido. Recorridos manuales distintos; parallax/transiciones y foco de cápsula siguen abiertos.
3. Composición: tarjetas compactas, más paisaje, capturas auténticas y corrección de máscara blanca/título largo. Aceptación visual humana pendiente.
4. Movimiento: transición 240 ms finita/interrumpible y planos UI/habitación con desplazamiento relativo; sin nuevas ilustraciones. Suavidad física pendiente.
5. Integración: Rust/Python/replays y ARM64 pasan; paquete privado preparado. Siguiente tarea única: ensayo gráfico supervisado del hash final con recuperación y retorno visible a ES.

No publicar ni hacer commits automáticos. No alterar dArkOS, arranque, ROMs o guardados. No ampliar Atlas/audio/clima/instaladores. [Evidencias y puerta de aceptación](PHASE13_REPORT.md).

---

## Historial conservado — no es el hito activo

# Hoja de ruta por hitos

Fuente principal: [EMOTIONALUI_MASTER](docs/EMOTIONALUI_MASTER.md). Propuesta de 19-10-2026 pendiente de aceptación del propietario. **Único hito activo: 0.** No hay nuevas tareas de implementación autorizadas por esta hoja.

| Hito | Estado | Alcance y aceptación |
|---|---|---|
| 0 · Consolidación | Activo; documentación preparada, revisión pendiente | Estado/evidencias, especificación, auditoría Git/licencias y primer checkpoint local seguro. Sin autor/allowlist aceptados no se crea commit. |
| 1 · Interfaz | En espera | Sistema visual común, dock de cuatro destinos, cápsula, foco y legibilidad. Aprobación visual humana y ensayo supervisado del mismo build en RG353V. |
| 2 · Hardware | En espera | Mandos, pantalla, audio, biblioteca, juego real, retorno y recuperación independiente; sin cambiar el arranque. |
| 3 · Calidad editorial | En espera | Revisión SNES, evaluación humana comparativa, diversidad/repetición, preferencias y rendimiento medido. |
| 4 · Distribución | En espera | Entrada de Modo prueba desde ES comprobada, instalación/restauración reversibles, auditoría pública y prueba completa de diez minutos; publicación solo autorizada después. |

No confundir código implementado, simulación y revisión física anterior. El ELF actual no tiene validación física. No se ha aceptado automáticamente el diseño ni demostrado el audio del frontend o una partida real.

**Siguiente tarea propuesta, aún no iniciada:** revisión humana de las capturas actuales Continuar/Descubre sin cambiar código; contrato y punto de parada en la [especificación](docs/EMOTIONALUI_MASTER.md#8-una-sola-recomendación-de-siguiente-tarea).

Backlog diferido, no activo: siete momentos ambientales graduales; pausa inmersiva con RetroArch; controles completos de dArkOS; blur estático, microanimaciones y bordes sobre gameplay sujetos a viabilidad. Su alcance se conserva en la [especificación](docs/EMOTIONALUI_MASTER.md#5-backlog-diferido--no-iniciar-ahora). No se implementa en esta consolidación.

## Planificación histórica conservada — no es la cola vigente

Las prioridades y frases «siguiente paso» del bloque inferior pertenecen a sus fases originales. No sustituyen los hitos anteriores ni autorizan ejecutar tareas.

<details>
<summary>Mostrar el contenido anterior de ROADMAP, conservado íntegramente</summary>

## Próxima comprobación · revisión visual de 19-10-2026

Validar de forma supervisada las cuatro rutas del dock, cápsula/foco y legibilidad del nuevo layout en RG353V. Conectar indicadores únicamente a interfaces de sistema verificadas; por ahora se muestran desconocidos y no cambian configuraciones. No sustituir EmulationStation ni tocar el arranque para esta prueba.

# Continuidad actual · navegación y arte editorial · 19-10-2026

Implementados cuatro estados de presentación (hero, carátula compuesta, captura y tipografía), selección local, composición en el worker existente, caché persistente acotada y reutilización en Continuar/Descubre. Encabezados y dock aclaran la navegación; las dos composiciones y el CTA compartido se validan con renders reales. Véanse ARTWORK_IMPLEMENTATION_REPORT.md y EDITORIAL_ARTWORK.md.

Siguiente hito: revisión humana de las capturas y prueba supervisada de esta versión sobre una biblioteca representativa en RG353V, midiendo caché en microSD, memoria, foco y tiempos de presentación. Mantener la recuperación independiente y el checkpoint de escucha de Fase 12. Sin cambiar arranque ni sustituir EmulationStation. Logo compuesto, plataforma normalizada, watcher de fuentes y recolección de temporales huérfanos quedan como mejoras posteriores; no se necesitan nuevos scrapers ni IA local.

---

# Hoja de ruta vigente

## Actualización vigente — referencia original y dos vistas

La dirección elegida es Foto 1.jpg: habitación lofi, lámpara cálida y cristal oscuro. La propuesta beige se conserva como exploración histórica y no se amplía. Continuar y EmotionalUI comparten escena y navegación; capturas reales en docs/visual-correction/final. SceneEngine, motor editorial y optimizaciones de biblioteca se conservan.

Prioridad inmediata cumplida en desarrollo: dos pantallas renderizadas a 640 × 480, navegación/foco y regresiones de exportación. Siguiente paso: revisar esta adaptación y producir máscaras de luz coherentes, sin cambiar cámara ni abrir nuevas direcciones. Después conectar un emulador y registrar sesiones fiables con recuperación probada. Continuar todavía no abre un juego real. Sin cambios al arranque de dArkOS.

## Planificación histórica anterior

## Entregado en desarrollo · fase 5

Interacción Z/X/S/A/Q/E configurable, controles abstractos y reproducción determinista, foco Glass, confirmación y transiciones interrumpibles, seis escenarios emocionales y contexto enlazado al recomendador. Importación XML → SQLite → GUI navegable, seis filas visibles y preferencias separadas. Docker Debian 13 ARM64 reutilizado, compilación/Clippy/tests dentro del contenedor, capturas software de la aplicación. Véanse PHASE5_REPORT.md y TESTING.md.

Las fases 1–3 y la auditoría anterior se conservan en sus informes y en docs/simulator/history/. No existe informe de fase 4. El código actual no se ha instalado ni validado físicamente durante esta entrega.

## Siguiente · validación supervisada RG353V

1. Reabrir la sesión cuando el usuario esté ante la consola y verificar recuperación independiente.
2. Compilar/empaquetar el release con ABI validada y comprobar manifiesto y dependencias antes de iniciar.
3. Validar pantalla, orientación y retorno a EmulationStation con cierre automático.
4. Completar R2 y stick izquierdo, comprobar cruceta/A/B/Start y recorrido de foco.
5. Medir presentación DRM, latencia observable, CPU/RSS y estabilidad; evaluar costes de transición.
6. Diagnosticar/calibrar táctil opcional. No cambiar arranque ni servicios persistentes.

Criterio: navegación física y retorno visible reproducibles. Docker no satisface ese criterio. 60 FPS, <100 ms y memoria contenida continúan como objetivos.

## Después · biblioteca grande y un emulador

Perfilado con miles de entradas, ranking y escaneo en worker si las mediciones lo requieren, persistencia por cambios, imágenes externas autorizadas y caché acotada. Revalidación antes de lanzar, adaptador de un emulador, argumentos seguros y comprobación de partidas. No confundir playcount con un savestate.

## Más adelante

Confianza NTP del sistema, puesta en hora completa por botones, clima opcional con atribución/caché, amanecer/atardecer, hemisferios y calendario regional ampliado, accesibilidad en LCD. Valorar arranque opcional como frontend principal únicamente después de validar recuperación y rendimiento.

## Fase 8 · estado histórico

Implementados motor ambiental independiente, tres modos y volúmenes persistentes, maqueta musical original MIDI/WAV/Ogg/M4A, dos ambientes sintetizados, salida nativa macOS/ALSA y panorama opcional de un plano entre vistas. Las pruebas de cesión utilizan un proceso sintético y salida temporizada muda. El lanzador real permanece pendiente: Continuar todavía no inicia un emulador.

Siguiente hito: escuchar y revisar la maqueta, validar en sesión supervisada la salida ALSA de dArkOS y conectar el coordinador de lanzamiento respetando ReleaseTicket antes de crear el proceso. Comprobar retorno y recuperación independiente; después medir audio/presentación/latencia en RK3566. No modificar arranque como parte de este hito. Antes de parallax multicapa se necesitan capas artísticas y máscaras coherentes. La síntesis actual no se declara felt piano definitivo.

## Fase 9 · continuidad

Dos candidatas originales con piano vertical muestreado CC0 listas para escucha humana, fuentes y render reproducible; no aprobadas como identidad definitiva. Mantener banco anterior hasta elegir. Continuar/biblioteca/recomendación ya usan adaptador cuando se configura un perfil autorizado; supervisor por salida de proceso, cesión de audio, regreso y registro conservador de resultados se verifican en simulación.

Siguiente paso exacto: revisar A/B, verificar en la consola el ejecutable/core y su contrato foreground, generar un perfil con rutas reales y diseñar la prueba supervisada consciente del PID del juego. No reutilizar un watchdog de tiempo fijo ni cambiar arranque. Después validar DRM/evdev/audio/retorno con un único emulador y medir hardware. Savestates, overrides por juego, hooks completos de dArkOS y scrapers siguen fuera del ciclo demostrado.

Cierre de validación fase 9: 124 tests macOS; 126/127 Docker por defecto/simulator; 25 Python; suite de ejecución con 48 comprobaciones externas y 10 casos nuevos de proceso. ELF KMS release completo y CLI en Docker; juego y retorno físico pendientes. Capturas RGB reales y preescuchas en Escritorio/EmotionalUI-Fase9.

## Fase 10 · estado y siguiente paso

Implementados Continuar con hasta tres partidas, intención explícita reversible, Atlas SQLite offline de solo lectura, pipeline editorial externo, reserva persistente y pruebas de biblioteca sintética hasta 10.000 entradas. Piloto: 52 identidades SNES contrastadas; 21 atributos y una relación aún propuestos, sin mejora de calidad editorial demostrada. No se declara completo SNES ni se amplían plataformas todavía.

Siguiente paso exacto: revisar con fuentes de mecánicas y responsable un subconjunto variado de fichas, resolver variantes de identidad y activar solo atributos aprobados; repetir la hoja comparativa y evaluación humana. Después comprobar un único core/ejecutable físico y guardados aislados antes de una sesión supervisada de lanzamiento/retorno. Sin cambios de arranque. La selección preparada sigue opcional; medir sus costes con bibliotecas reales antes de ampliarla.

## Fase 11 · continuidad vigente

Implementados canal experimental SNES de 21/52 obras, revisión trazable, comparación automática y Modo prueba de biblioteca real con importación de fondo/miniaturas. Instalador de archivos propios, restore y uninstall probados en árboles sintéticos; instalación principal bloqueada. La escena, audio y dos vistas se conservan.

Siguiente paso: revisión humana de fichas y tabla de selecciones; después verificar entrada/retorno del modo temporal en una RG353V con recuperación independiente y supervisión actual. Validar un único emulador con contenido legal y guardados aislados, conservando EmulationStation. Resolver derechos/notices antes de la primera distribución y medir el objetivo de diez minutos con una persona; no ampliar a GBA aún.

## Fase 12 · integración física en curso

Demostrados: GUI en el LCD, botones y retorno temporal a ES, según confirmación humana; DRM/ALSA y biblioteca real inspeccionados. Pendientes: escucha de piano confirmada, percepción del panorama, importación completa sin contención, mediciones de presentación y primer juego real. No cambiar arranque ni activar instalación principal. Se mantienen servicios, ROMs y guardados originales.

Siguiente paso inmediato: cerrar la escucha sincronizada con GUI visible; resolver el audio antes de avanzar al ciclo RetroArch. Después repetir la importación con las optimizaciones y cerrar las regresiones Docker, preparar un único homebrew autorizado con configuración/guardados aislados y recuperación ligada al proceso, sin watchdog fijo de juego.

</details>

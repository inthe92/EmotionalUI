# EmotionalUI — especificación maestra

Versión 1 · 19-10-2026 · **Propuesta consolidada, pendiente de aceptación del propietario.**

Referencia principal de producto y proceso. Los informes numerados conservan valor histórico, pero no definen por sí solos el estado actual ni autorizan tareas. Ante una contradicción: código/configuración → pruebas y registros reproducibles → evidencia física de la revisión identificada → informes → hipótesis. Una ejecución anterior en la consola no valida el build actual.

**Hito activo: 0 — Consolidación.** Esta intervención solo modifica documentación. No se han ejecutado compilaciones, suites, benchmarks ni capturas nuevas, no hay conexión al dispositivo y no se ha creado un commit. Se inspeccionan las evidencias existentes.

## 1. Identidad y límites

Frontend nativo de videojuegos retro. Primera plataforma: Anbernic RG353V con dArkOS; Rust, Slint 1.13.1, SQLite y emuladores existentes. No es una distribución Linux y no sustituye RetroArch ni otros emuladores.

- Menos buscar. Más jugar.
- Una ROM es un archivo. Un videojuego es una obra.
- Respetar el tiempo del jugador, evitar catálogos impersonales y presentar cada obra con cuidado editorial.
- Funcionamiento local, sin cuenta, publicidad, telemetría obligatoria ni IA en la consola. Preferencias e historial permanecen locales por defecto.
- Conservar EmulationStation y los archivos originales. No tocar ROMs, BIOS, guardados, arranque, particiones o configuración global para completar un hito por atajo.

## 2. Realidad comprobada del repositorio

**Identidad actual:** workspace 0.2.0, branch `main`, sin HEAD/commits, sin archivos seguidos, sin remoto y sin autor Git configurado. Todo el proyecto sigue sin seguimiento. El número inicial de archivos no ignorados era 2.530, de los cuales 2.226 estaban en `docs/`; no hacer `git add .` ni asumir que `.gitignore` constituye una auditoría.

El manifiesto [de fuentes](visual-unification-2026-10-19/source-manifest.json) coincide en sus **32 archivos inventariados** al inspeccionarlo. Esa lista no certifica por sí sola cada archivo del árbol. Los registros dan 178 tests Rust macOS, 180 Linux, 181 con simulator y 71 Python. No se han repetido en esta consolidación.

ELF KMS actual: `target/aarch64-unknown-linux-gnu/release/emotional-ui`, 15.606.936 bytes, SHA-256 `5c7fb0073823e4edbbf8ed8e185b7d8ed3825a038099c006d235908397f5da85`. Su hash coincide con [la auditoría de ELF/SDK](visual-unification-2026-10-19/final-arm64-elf.json). **No consta desplegado ni validado físicamente ese binario.** Las pruebas físicas históricas se refieren a otro ELF, identificado en Fase 12 como `a046a019…`; también hay builds intermedios no ejecutados físicamente.

### Estados y evidencias

Las categorías no son equivalentes: Implementado significa código existente; Validado en simulación añade una prueba en el entorno indicado; Validado físicamente siempre identifica una revisión y una observación. Pendiente no equivale a fallo. Bloqueado indica una condición concreta que impide avanzar.

| Capacidad | Categoría | Evidencia identificable y límite |
|---|---|---|
| Continuar, Descubre, Biblioteca y Ajustes; encabezado y dock únicos | Implementado | [home.slint](../app/ui/home.slint), [integrated.slint](../app/ui/integrated.slint), [navigation.rs](../app/src/navigation.rs). Biblioteca/Ajustes reutilizan listas y acciones existentes. |
| Cuatro destinos, foco distinto de sección activa, recuperación de foco, entradas durante transición, movimiento reducido y vacío | Validado en simulación | [21 casos finales](visual-unification-2026-10-19/focus-final-navigation/report.json), ejecutados en Docker ARM64. No prueba botones físicos de esta revisión. |
| Dos composiciones, CTA compartido y presentación a 640×480 | Validado en simulación | [Comparación real](visual-unification-2026-10-19/final-verified/pareja.jpg), [revisión de estados](visual-unification-2026-10-19/final-verified/visual-review.json). Datos/arte de juegos y sesiones sintéticos propios; no aprobación artística humana. |
| Cápsula de cinco iconos, expansión, cierre y enlace a Ajustes | Implementado | [system-controls.slint](../app/ui/system-controls.slint), [navigation.rs](../app/src/navigation.rs); cubierta por los replays anteriores. Todos los indicadores dicen «Sin verificar». |
| Lecturas/controles reales de Wi-Fi, batería, Bluetooth, volumen global y brillo | Pendiente de validación | El componente actual no conecta esas interfaces. Los campos son informativos/deshabilitados; no son controles de hardware implementados. |
| Hasta tres partidas, preferencias explícitas y sesiones fiables | Implementado | [continuity.rs](../core/src/continuity.rs), [persistencia](../app/src/store/continuity.rs), [opciones](../app/src/game_options.rs). Sin inferir guardado/completado por tiempo. La home con tres partidas se prueba con actividad sintética. |
| XML/SQLite propios, biblioteca progresiva, recursos locales | Implementado | [trial.rs](../app/src/trial.rs), [store.rs](../app/src/store.rs), [importador XML](../services/src/emulationstation.rs). El refresco/ranking aún tiene trabajo síncrono en memoria; no se promete un arranque completamente asíncrono. |
| Compositor y caché: hero, carátula, screenshot y tipografía | Validado en simulación | [artwork.rs](../app/src/artwork.rs), [12 casos](visual-unification-2026-10-19/verified-runtime/artwork/report.json), [documentación técnica](../EDITORIAL_ARTWORK.md). Worker, límites, fuente intacta y caché; no benchmark actual de microSD. |
| SceneEngine de seis momentos y contexto offline | Implementado | [scene.rs](../core/src/scene.rs), `RoomScene` y [manifiesto](../assets/scenes/lofi-room-v2/manifest.toml). Franjas fijas, master compartido/tintes/ornamentos provisionales; sin siete momentos ni amanecer astronómico conectado. |
| AmbientAudioEngine, modos, volúmenes, recursos originales y cesión de audio | Implementado | [motor](../services/src/ambient_audio/mod.rs), [procedencia](../AUDIO_ASSET_PROVENANCE.md), pruebas de servicios en los registros de compilación. Cesión/retorno con procesos de prueba, no partida física. |
| Atlas SNES y pipeline externo, reserva local persistente | Implementado | [Atlas](../app/src/atlas.rs), [pipeline](../tools/atlas.py), [reserva](../core/src/reserve.rs). 52 identidades y canal experimental de 21 fichas; `human_review: null` en la fuente. Sin aprobación editorial humana ni mejora de calidad demostrada. |
| Lanzador con perfiles explícitos, supervisor y registro de sesión | Validado en simulación | [launcher/configured.rs](../services/src/launcher/configured.rs), [supervisor.rs](../app/src/supervisor.rs), [contrato](../GAME_LAUNCHER.md), tests en registros. Procesos controlados, no emulador real en RG353V. |
| Modo prueba `--trial`, instalador de archivos propios, restore/uninstall | Validado en simulación | [trial.rs](../app/src/trial.rs), [manage.py](../platform/distribution/manage.py), [tests](../tests/test_phase11_tools.py). `--trial` rechaza lanzadores; no es una prueba completa de juego. |
| GUI anterior, cruceta/A/B/L/R/Start y retorno temporal a ES | Validado físicamente | [gráficos](../PHASE12_GRAPHICS.md), [entrada](../PHASE12_INPUT.md), [recuperación](../PHASE12_RECOVERY.md), registros privados `docs/phase12/` y confirmaciones del usuario recogidas en [Fase 12](../PHASE12_REPORT.md). Otra revisión; alcance/percepción de transición incompletos. |
| Piano dentro del frontend y ciclo real de juego/retorno | Pendiente de validación | [audio](../PHASE12_AUDIO.md): muestra independiente muy tenue, sin escucha confirmada del motor dentro de la GUI. [juego](../PHASE12_GAME_LAUNCH.md): contenido de prueba preparado, no ejecutado. |
| Compilación completa Linux ARM64 y dependencias SDK | Validado en simulación | ELF actual verificado arriba; [build Docker](visual-unification-2026-10-19/focus-verified-docker-build.log), [tests macOS](visual-unification-2026-10-19/final-tests.log). SDK/build no certifican LCD, audio o mandos. |
| Diseño aprobado y build actual probado en RG353V | Pendiente de validación | Existen capturas; falta aceptación expresa del usuario y ensayo supervisado del mismo hash actual. |
| Instalación principal y distribución pública | Bloqueado | [readiness](../RELEASE_READINESS.md), [auditoría](../PUBLICATION_AUDIT.md): faltan recuperación de juego, pruebas del build, derechos del fondo e inventario transitivo. Instalar desde ES y diez minutos no demostrados. |
| Primer checkpoint Git | Bloqueado | Sin autor configurado y sin allowlist aceptada. Propuesta documental segura en §7; el checkpoint completo del código requiere resolver recursos/privacidad. |

Las evidencias físicas privadas no se incorporarán automáticamente a un commit público. Los 60 FPS, respuesta <100 ms, RAM inicial <150 MB y puesta en marcha en diez minutos son objetivos, no resultados actuales de hardware. Véase [TESTING](../TESTING.md) para comandos y resultados históricos; no sumar suites anidadas como pruebas independientes.

## 3. Especificación de producto vigente

### Navegación y contenido

- Dock global permanente: **Continuar · Descubre · Biblioteca · Ajustes**. Compartido, centrado y compacto; selección activa y foco independientes. No duplicar accesos globales dentro de tarjetas.
- **Continuar:** hasta tres partidas, una protagonista y dos secundarias. Ranking prudente de historial, duración, regresos, preferencias e indicios fiables; fijar/retirar/completar por intención explícita. Composición compacta, familiar y acción inmediata. Nunca inventar capítulos, porcentajes o guardados.
- **Descubre:** una recomendación protagonista y alternativas discretas disponibles; hoy la composición muestra una alternativa. Arte horizontal a la izquierda cuando sea adecuado, información editorial integrada a la derecha con degradado oscuro cálido. Composición diferente de Continuar; sin catálogo infinito ni candidatos inventados.
- Biblioteca y Ajustes conservan las funciones existentes. Los menús contextuales se reservan para acciones del juego. Navegación por botones, sin gestos táctiles obligatorios; mantener atajos L/R mientras sean compatibles.

### Sistema visual

Una sola identidad: habitación lofi cálida, ventana al paisaje, iluminación ámbar, materiales oscuros translúcidos; no volver al diseño beige ni abrir identidades alternativas. Ambas secciones comparten familia/escala tipográfica, materiales, botones, alineaciones, bordes, foco, animaciones, dock y cápsula. La referencia comercial guía composición, no autoriza redistribuir su arte.

Resolución **640×480**, rejilla de **4 px**, preferencia por separaciones de 8 px. Radios cerrados: **16 / 12 / 8 / 6 px**. Cápsulas: radio = altura/2. Imágenes interiores: máximo(0, radio exterior − separación). Curvas circulares coherentes, foco consistente y movimiento reducido. Sin blur dinámico.

Geometría vigente del prototipo: dock 344×52 R26; cápsula 176×28 R14; CTA compartido 188×36 R18. Los 36 px del CTA conservan texto de 15 px y distintivo de 20 px. No son nuevas variantes estéticas; cualquier ajuste posterior debe justificar legibilidad/geometría y actualizar el componente único. [DESIGN_SYSTEM](../DESIGN_SYSTEM.md) mantiene el detalle técnico.

### Cápsula y arte

Cinco indicadores: Wi-Fi, batería, Bluetooth, volumen y brillo. Anclaje superior derecho compartido; panel desplegable hacia abajo y B para cerrar/restaurar foco. Si no hay integración verificada, estado desconocido y control deshabilitado. Nunca simular cambios del sistema. Hoy solo funciona la navegación del panel hacia Ajustes completos.

Conservar EditorialArtwork y su worker/caché: hero horizontal, carátula vertical compuesta, captura nítida y fallback tipográfico. Sin descargas durante navegación, estiramientos ni filtros borrosos sobre pixel art. Reutilizar recursos autorizados, validar rutas/tamaños y no modificar originales.

### Sonido, escena y edición

Conservar AmbientAudioEngine y la identidad original de piano: discreción, pocas notas y silencio. No producir música nueva mientras existan problemas funcionales prioritarios. No considerar audible o cedido al emulador un audio solo por abrir PCM o terminar con exit=0.

La escena actual tiene seis momentos y un lugar compartido por las dos vistas. Los siete momentos, capas más precisas y clima real son backlog, no capacidades actuales.

Atlas comienza por SNES: IA únicamente fuera de consola para preparar propuestas trazables; JSON fuente y SQLite local de solo lectura. Datos objetivos, interpretaciones, revisión y aprobación separados. Ranking ligero, explicable y offline, con preferencias locales y candidatos preparados. No declarar mejores recomendaciones por cambiar una tabla ni aprobar automáticamente fichas generadas.

### Distribución y recuperación

Modo prueba antes de instalación principal. Reutilizar gamelist.xml, recursos locales, ROMs y emuladores existentes sin copiarlos/modificarlos innecesariamente. Conservar EmulationStation y restauración independiente. Instalación reversible; diez minutos es un objetivo pendiente de prueba completa con un dispositivo compatible ya configurado. No exigir al usuario final compilar/SSH como resultado definitivo de ese objetivo.

## 4. Hitos y puertas de aceptación

La [hoja de ruta](../ROADMAP.md) es el índice operativo de estos hitos. Solo uno estará activo. Los números de fase anteriores son historia, no una cola de tareas.

| Hito | Puerta de aceptación |
|---|---|
| **0 · Consolidación — activo, pendiente de aceptación** | Estado/evidencias y especificación aceptados; auditoría Git/licencias identificada; allowlist/autor decididos y primer checkpoint local seguro. No marcar cerrado solo por escribir este documento. |
| **1 · Interfaz — en espera** | Composiciones distintas y sistema visual común; dock/cápsula/foco/legibilidad revisados. Aprobación visual expresa del usuario y validación supervisada en RG353V del build identificado. |
| **2 · Hardware — en espera** | Pantalla/mandos/audio/biblioteca y un lanzamiento real con retorno al frontend/ES, guardados aislados y recuperación independiente demostrados; sin cambiar el arranque predeterminado. |
| **3 · Calidad editorial — en espera** | Fichas SNES revisadas, evaluación humana comparativa, diversidad/repetición/preferencias y medidas reproducibles; sin proclamar superioridad editorial automática. |
| **4 · Distribución — en espera** | Modo prueba accesible desde ES verificado, instalación/restauración seguras, auditoría pública y prueba completa de diez minutos. GitHub/Reddit solo mediante autorización posterior. |

## 5. Backlog diferido — no iniciar ahora

**A · Ambiente en siete momentos.** Madrugada, mañana temprana, media mañana, primera tarde, media tarde, anochecer y noche. Evolución gradual del mismo lugar, no siete fondos con saltos. Ubicación regional, fecha/estación/amanecer/atardecer y clima opcional fiable; funcionamiento offline y contexto compartido entre Continuar/Descubre. Priorizar capas y variantes precalculadas de bajo coste. Requiere sustituir explícitamente las seis franjas actuales, no afirmar que ya existe.

**B · Pausa inmersiva.** Solo con juego realmente pausado cuando sea posible: imagen reducida/desplazada a la derecha y menú estrecho izquierdo sin taparla. Integración futura RetroArch, condicionada a arquitectura gráfica y cesión de entrada/audio comprobadas.

**C · Continuidad completa del sistema.** Brillo, sonido y conexiones propios, acceso a herramientas completas de dArkOS y preservación de sus funciones. Depende de interfaces y permisos verificados; la cápsula actual no satisface esta integración.

**D · Pulido avanzado.** Desenfoque estático de menús, microanimaciones/transiciones espaciales y curvas sobre gameplay, condicionados a viabilidad y medidas. No se usan para compensar un layout sin aceptar.

Ninguno se activa antes de las puertas anteriores ni por aparecer en un prompt antiguo.

## 6. Protocolo breve de trabajo

Toda petición nueva debe fijar **Objetivo** (una mejora), **Alcance**, **Exclusiones**, **Evidencia**, **Criterio de aceptación** y **Punto de parada**. Si mezcla subsistemas independientes, dividirla y dejar solo una tarea activa; registrar el resto, no ejecutarlo implícitamente.

- **Visual:** captura anterior → modificación → captura posterior/comparación → pruebas de foco/geometría afectadas. Máximo dos pasadas de pulido, después revisión humana. Una captura no es aprobación.
- **Técnico:** reproducir → cambio mínimo → pruebas relevantes → evidencia y riesgos. Suite completa al cerrar hito o por riesgo de regresión global; no repetirla por dos píxeles.
- **Hardware:** identificar build/hash, dispositivo y recuperación. Pedir confirmación de sesión antes de ceder pantalla, ensayar de forma disruptiva o modificar configuración. Autorizaciones de ensayos anteriores no sustituyen supervisión actual. Separar simulación, proceso vivo e imagen/sonido observados.
- **Informe:** qué cambió, qué se probó, resultado, pendiente y decisión del usuario. Breve; no repetir informes históricos ni regenerar evidencias sin cambio relevante.
- **Eficiencia:** usar evidencias vigentes; no repetir benchmarks, instalar dependencias por comodidad, reescribir componentes correctos o abrir mejoras fuera del objetivo.
- **Parada obligatoria:** criterio alcanzado o dos pasadas visuales sin aceptación; bloqueo de derechos/identidad; evidencia contradictoria; necesidad de ampliar alcance o prueba física no supervisada. Conservar estado y consultar, sin inventar resultados.

## 7. Primer punto de control Git — propuesta, no ejecutada

Primera propuesta: **commit documental local y explícitamente parcial** de `docs/EMOTIONALUI_MASTER.md` y `ROADMAP.md`, tras revisión del propietario y decisión de autor. No incluiría binarios, arte, música, bases, capturas, logs ni datos personales. No es un checkpoint compilable del código; este seguiría sin seguimiento y eso debe constar, sin llamarlo release.

Antes de ejecutarlo: confirmar autor/identidad local, revisar los dos documentos completos y confirmar su allowlist exacta. Después, solo esos paths con `git add -- ...`, revisar `git diff --cached --check`, nombres y diff completo; crear el commit local. No se ha hecho staging, no se configura identidad ficticia y no hay remoto/push. Si la decisión se pospone, no simular un checkpoint existente.

**Checkpoint completo del código:** preparar una segunda allowlist revisada cuando se resuelvan derechos/procedencia del fondo adaptado y notices transitivos. El fondo está embebido desde Slint; excluirlo sin resolver su sustitución produciría un árbol que no reconstruye la aplicación. Esta tarea no cambia recursos para eludir ese bloqueo.

Licencia declarada: GPL-3.0-only en Cargo y LICENSE existentes; no se elige otra aquí. DejaVu y muestras de piano mantienen avisos/licencias separados. Pendientes de distribución: derechos del fondo de referencia adaptado, inventario transitivo y material histórico privado. El escaneo acotado actual no detectó patrones de credenciales en 202 archivos de texto candidatos; sí rutas personales en siete líneas de informes. No demuestra ausencia global de secretos. Los 2.226 archivos iniciales bajo docs y las capturas/logs nuevos no quedan todos protegidos por `.gitignore`; se requiere allowlist, nunca add indiscriminado. [Auditoría](../PUBLICATION_AUDIT.md).

## 8. Una sola recomendación de siguiente tarea

**Revisión humana de la comparación actual Continuar/Descubre, sin editar código.**

- Objetivo: decidir si la composición y el sistema visual actuales se aceptan como base del Hito 1.
- Alcance: [comparación](visual-unification-2026-10-19/final-verified/pareja.jpg), capturas de foco/estado y [guías](visual-unification-2026-10-19/final-verified/guias.png) existentes.
- Exclusiones: nuevas variantes, cápsula de hardware, música, Atlas, instalación, pruebas físicas y rediseño automático.
- Evidencia: anotaciones del usuario sobre esas imágenes; conservar aprobación o lista breve de defectos concretos.
- Aceptación: decisión explícita del usuario. No equivale a validar hardware ni cerrar Hito 1 entero.
- Parada: al recibir la decisión; cualquier corrección se convierte en otra petición acotada. **No ejecutar esta siguiente tarea ni iniciar el siguiente hito durante la consolidación.**

La decisión inmediata sigue siendo revisar esta especificación y la propuesta de checkpoint. Nada de lo anterior autoriza comenzar automáticamente otra intervención.

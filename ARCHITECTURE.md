# Arquitectura · 0.2.0

Se mantienen tres crates. El núcleo no depende de Slint, SQLite, red ni dispositivos de entrada.

```text
app/src/main.rs             CLI, configuración, composición y comprobaciones nativas
app/src/state.rs            estado de pantalla, acciones y traducción a propiedades Slint
app/src/store.rs            SQLite, migración aditiva y worker de persistencia
app/ui/design.slint         tokens de paleta, tipografía, radios y tiempos; Glass reutilizable
app/ui/home.slint           pantalla, foco, confirmación y capas de paneles
core/{mood,calendar,...}    reglas puras, navegación espacial y recomendaciones
services/clock             hora local/simulada/desconocida y confianza
services/weather           condición y vigencia de observaciones, sin HTTP
services/emulationstation  importador XML acotado, de solo lectura
platform/darkos             inventario y smoke manual; no instalador
```

## Flujo y rendimiento

La navegación cambia el estado y las propiedades visuales en el mismo callback. Las animaciones pertenecen a Slint y no bloquean acciones posteriores. El foco indica inmediatamente el destino; solo su acabado se anima. Los paneles permanecen instanciados para poder animar también su cierre; un panel abierto impide activar el contenido situado detrás.

Las escrituras se envían mediante `try_send` a una cola acotada a ocho instantáneas, atendida por un único worker SQLite. Preferencias, hábitos y orden se guardan en la misma transacción. Cola llena, fallo de disco o cierre del canal generan error, no éxito simulado. Los resultados se consultan durante la sesión; al salir se vacía la cola y se comprueba el resultado. No se bloquea el hilo gráfico esperando al disco durante navegación. Las seis entradas demo hacen pequeñas las copias; para miles de juegos conviene sustituir instantáneas completas por cambios por ID antes de conectar el catálogo real.

El reloj se consulta una vez por segundo, pero no repinta el contenido mientras no cambie el minuto/contexto. Las actualizaciones se aplazan durante paneles o pulsaciones. El orden recomendado permanece estable dentro de la fecha/franja; N solicita renovación. Un cambio de fecha/franja recalcula el orden cuando termina la interacción. Los presets de desarrollo actualizan ambos motores explícitamente. La hora simulada es civil local y permanece congelada: no modela DST ni otras zonas horarias.

## Recomendaciones

Disponibilidad y ID válido se filtran antes de puntuar; IDs duplicados no producen dos entradas. Se conservan prioridad de continuación (+65), favorito (+18), duración (+18/−15), preferencia global acotada (+30), contexto débil y variedad determinista (0–28). Se añaden elecciones por franja con peso máximo +15. No se infiere estado emocional ni se excluyen géneros por hora o clima.

El protagonista conserva la primera puntuación; para las alternativas puede elegirse otro género si queda a ≤12 puntos del candidato. La renovación reduce recencia antigua y penaliza los tres títulos mostrados, evitando una saturación permanente. Un guardado con IDs obsoletos, repetidos o ausentes se repara, añadiendo juegos nuevos disponibles. Los catálogos de cero, uno y dos títulos tienen estados seguros en vista y navegación.

El catálogo demo es ficticio y explícito. `available` es una precondición del adaptador, no una prueba de existencia física del núcleo puro. El adaptador real solo importa archivos existentes dentro de la raíz; antes de un futuro lanzamiento hay que revalidarlos. No se implementa lanzamiento en esta versión.

## Persistencia y migración

Esquema v2, aditivo desde v1 y transaccional. Se conserva `preferences`; se añaden `habits`, `session_items` y `imported_games`. Las sesiones v1 siguen siendo legibles; las nuevas se almacenan por filas, admitiendo comas y saltos de línea en identificadores. Una versión futura desconocida se rechaza sin degradarla. Consultas parametrizadas. No hay cambios en bases o configuraciones de EmulationStation.

`imported_games` conserva ruta canónica, XML de origen y un mapa de campos serializado en TOML. Está separado de preferencias y futuros datos editoriales/emocionales; reimportar no sobreescribe elecciones del usuario. No se deduce una emoción a partir del género ni un savestate a partir de `playcount`.

## Importador no destructivo

`--import-gamelist XML --rom-root DIR --db DB` ejecuta una importación explícita y termina sin abrir la GUI. Usa roxmltree 0.20.0, ya presente en la resolución de Slint: no se introduce otra familia de dependencias. Rechaza DTD/entidades externas, XML mal formado, >8 MiB, >100.000 nodos, profundidad >32 y campos >16 KiB. Normaliza raíz y archivo; excluye ausentes, directorios, duplicados y enlaces que escapen de la raíz autorizada. El XML puede estar fuera de la carpeta ROM, como permite EmulationStation.

Conserva nombre, descripción, género, favoritos, historial, imágenes, miniaturas y otros campos como datos originales. No expande `~/` implícitamente ni decodifica carátulas externas. No importa comandos ejecutables ni inicia emuladores. Las referencias a imágenes pueden salir de la raíz en el XML y se conservan como texto; antes de cargarlas habrá que autorizar raíces de recursos, validar dimensiones y gestionar caché. `--library-root` conecta esa tabla al modelo Game tras revalidar rutas y disponibilidad. La biblioteca tiene una ventana de seis filas; no es aún un escaneo incremental ni carga imágenes externas.

## Límites deliberados

La importación es una operación CLI previa a la ventana, no un escaneo en segundo plano del catálogo gráfico. No hay red en la aplicación, NTP propio, táctil probado ni supervisión de emuladores. Hay adaptador evdev y audio opcional originales, sin validación física del build actual. Una hora desconocida evita contexto de calendario/clima y hábitos temporales; la puesta en hora completa con botones sigue pendiente. Se mantiene un punto claro para sustituir Slint si LinuxKMS resulta inviable físicamente.

## Port experimental · fase 3

`tools/arm64/` contiene exclusivamente herramientas de host para SDK, cross-build, auditoría ELF y empaquetado. No afecta al núcleo ni al renderer de escritorio. El paquete Linux activa LinuxKMS software mediante features, sin añadir dependencias Rust ni cambiar Cargo.lock. `platform/darkos/` separa diagnóstico, preflight, supervisor manual y muestreo de recursos de la aplicación.

`app/src/smoke.rs` proporciona un diario TOML optativo, exclusivo y acotado por los eventos del smoke. Las escrituras síncronas solo se activan al pasar `--smoke-log`; no se usan en navegación normal. El cierre por archivo se consulta cada 100 ms en el event loop y se complementa con un supervisor externo para bloqueos. Ningún evento del diario prueba que un buffer se haya presentado en el LCD. El supervisor nunca cambia servicios gráficos ni la VT. Véase `PHASE3_REPORT.md`.

## Entrada y plataforma de simulación

`app/src/controls.rs` traduce teclado configurable; `core/src/controls.rs` produce acciones a partir de entradas crudas. `State::action` es el punto compartido. El teclado conserva fuentes simultáneas antes del RepeatGate; el controlador crudo mantiene fuentes, histéresis y desconexión por separado. `services/gamepad` limita lectura/colas y no captura exclusivamente el dispositivo. El worker de feedback opcional sintetiza PCM y usa aplay solo si está disponible y activado.

La feature `simulator` instala MinimalSoftwareWindow y un event loop software acotado; no cambia Home ni las reglas del producto. Los replays llegan a State::action o al FocusScope real, con expectativas comprobables. `moments.rs` solo se compila en desarrollo/simulator. El núcleo sigue sin depender del renderizador.

Las dos capas de HeroStory comparten componentes y materiales. El controlador conserva el estado sin esperar animaciones; los callbacks diferidos verifican generaciones antes de modificar la tarjeta actual. El reloj solo recalcula recomendaciones cuando cambia la clave fecha/franja, conservando el orden entre cambios.

La carga inicial SQLite/XML y el ranking siguen en el hilo previo/de UI según la operación. Las instantáneas de persistencia aún copian el catálogo; el paso a bibliotecas grandes requiere perfilado y una cola de cambios. Véase PERFORMANCE.md.

## Frontera de lanzamiento · fase 9

`services/launcher/configured.rs` resuelve identificadores importados mediante perfiles autorizados y argv estructurado; nunca convierte XML en órdenes. `app/launch_ui.rs` coordina la cesión asíncrona del audio y conserva foco/vista. `app/supervisor.rs` no inicializa Slint, audio ni evdev: espera la salida del frontend, ejecuta un proceso foreground, registra su resultado y crea un nuevo frontend. SQLite conserva intentos separados de sesiones fechadas y del progreso explícito.

La salida completa del proceso es la frontera elegida para liberar sus dispositivos, en vez de suponer que ocultar una ventana cede DRM. El perfil debe esperar a todos sus procesos de juego; un wrapper que daemoniza no está soportado. La recuperación física y la reapertura DRM/evdev/ALSA siguen pendientes en RG353V. El coste deliberado es reiniciar el frontend, sin volver a importar ni escanear el catálogo. Protocolo, fallos y recuperación en LAUNCH_LIFECYCLE.md.

## Fase 10 · continuidad, Atlas y reserva

core/continuity.rs separa continuidad personal de descubrimiento editorial. app/game_options.rs modifica intención explícita mediante el worker existente. app/store/continuity.rs adapta launch_log/activity sin inferir saves. core/reserve.rs prepara/selecciona IDs locales y mantiene estabilidad. app/atlas.rs abre solo lectura y aplica perfiles por enlaces explícitos, memoizando cada perfil por ID durante esa carga; no se leen miles de JSON.

La SQLite personal conserva su esquema funcional con tablas aditivas para preferencias/enlaces y settings para cache; Atlas vive en un archivo independiente. tools/atlas.py valida/builda el catálogo offline, fuera del ejecutable. Las propuestas IA no se convierten en perfiles activos sin revisión. El supervisor/audio/SceneEngine permanecen desacoplados y no se añade una dependencia de red o IA.

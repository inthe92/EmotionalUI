# AmbientAudioEngine

## Separación

- `core/src/audio.rs`: preferencias, modos, contexto y reglas temporales; sin dispositivos ni dependencias gráficas.
- `services/src/ambient_audio/assets.rs`: carga acotada de WAV PCM mono 16 bits, 22.050 Hz. Cinco clips compartidos mediante Arc; máximo 2 MiB por archivo. Recursos opcionales ausentes se omiten.
- `mixer.rs`: tres voces independientes (música, ambiente, interacción), ganancias y cesión. Bloques de 512 muestras (~23,22 ms), salida limitada, sin red.
- `output.rs`: interfaz de salida sustituible, ALSA en Linux y AudioQueue en macOS. Salida Probe temporizada y muda para pruebas. El simulador gráfico utiliza Unavailable y no accede al audio del host.
- `mod.rs`: único trabajador propietario del dispositivo, buzón con preferencias/contexto coalescidos y cola de cuatro cues. La interfaz solo publica cambios. No hay procesos aplay en la nueva ruta de reproducción.
- `app/src/audio_settings.rs` y `store.rs`: controles abstractos y SQLite. Los ajustes se guardan como TOML en settings/ambient_audio; no requieren cambiar la versión del esquema existente. Escritura asíncrona con coalescencia y flush al cerrar.

La antigua API Feedback se conserva por compatibilidad; sus PCM originales se reutilizan, pero la aplicación ya no instancia su reproductor mediante procesos. Los sonidos de foco se limitan a uno por 80 ms y no generan una cola ilimitada.

## Dispositivos y consumo

Linux carga `libasound.so.2` con símbolos verificados mediante libc ya existente. Solicita el PCM default sin modificar mezcladores del sistema; S16LE, mono, 22.050 Hz, resampling permitido y búfer objetivo 60 ms. Escritura no bloqueante con límite de 200 ms. No se añaden dependencias Cargo ni se requiere un paquete ALSA de desarrollo para enlazar. La presencia/configuración de la biblioteca y la salida real de dArkOS todavía deben comprobarse.

macOS utiliza el framework del sistema AudioToolbox: tres buffers de 512 muestras, callback limitado a devolver un buffer libre. Las firmas y estructuras se contrastaron con los headers del SDK instalado. Se probó abrir/escribir PCM cero/cerrar, sin afirmar escucha musical. Véanse [ALSA PCM](https://www.alsa-project.org/alsa-doc/alsa-lib/group___p_c_m.html) y [Audio Queue Services](https://developer.apple.com/documentation/audiotoolbox/audio-queue-services).

Durante los silencios el trabajador espera en una variable de condición y cierra la salida. Fallar al abrir o escribir produce silencio y estado consultable, sin reintentos continuos. Un cambio explícito de contexto/preferencias permite reintentar. No detecta automáticamente una reconexión de audio sin cambios. La carga inicial usa unos 3.087.000 bytes de muestras decodificadas, además de los bytes WAV incluidos en el binario y estructuras; no equivale a RSS total.

## Ciclo de vida obligatorio

1. El futuro coordinador llama `suspend_for_game()` y recibe un ReleaseTicket.
2. El mezclador realiza un fundido maestro nominal de 120 ms, detiene voces, drena con límite y destruye la salida.
3. Solo cuando `released(ticket)` sea verdadero se puede lanzar el proceso del juego. El método no espera en el hilo de Slint. El coordinador deberá tener timeout y no lanzar mientras no se confirme la liberación.
4. Durante la cesión se ignoran cues. Un ticket antiguo no autoriza un lanzamiento después de volver al frontend.
5. Al terminar el juego se llama `return_from_game(preferencias_actuales)`. Se resuelve la política vigente, sin abrir salida si está en Silencio, y se reinicia con entrada suave y el intervalo inicial correspondiente.

**Implementado y probado mediante salida muda y un proceso sintético `/usr/bin/true`; no conectado todavía al botón Continuar ni a un emulador real.** El botón conserva su comportamiento de prototipo, evitando simular un lanzamiento inexistente. No hay validación física de exclusividad ALSA, mezcla del sistema, altavoces ni regreso desde RetroArch.

Prueba reproducible: `cargo run -p emotional-services --example audio_probe --locked`. `--native-silent` intenta la API nativa exclusivamente con muestras cero; no prueba audibilidad. Las mediciones separan muestras mezcladas, bloques escritos y aperturas/cierres.

## Conexión de fase 9

Continuar ya solicita suspend_for_game a través de launch_ui y espera released(ticket) de forma asíncrona. Tras el cierre correcto del frontend, el supervisor abre el proceso autorizado. Mientras corre no hay frontend ni trabajador sonoro. El regreso crea un motor nuevo que lee los ajustes persistidos y entra con su política suave; no se ejecuta una segunda salida de audio encima del juego. La API return_from_game sigue disponible para cancelación previa y pruebas en proceso.

El ciclo integrado se prueba con procesos sintéticos controlados, incluido modo Silencio y persistencia; no se presenta como juego real de Anbernic. La nueva biblioteca muestreada cabe en el cargador existente (3.616.200 bytes PCM). El ejemplo audio_probe --assets RUTA probó mezcla, entrega a salida muda y liberación de esa biblioteca, sin afirmar audibilidad nativa ni rendimiento físico. Véanse GAME_LAUNCHER.md y LAUNCH_LIFECYCLE.md.

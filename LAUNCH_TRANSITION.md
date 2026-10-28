# Entrada a una partida · 28-10-2026

La interfaz conserva su distribución. Al abrir un juego, sus tarjetas, encabezado, navegación, cápsula y controles se desvanecen durante los primeros ~160 ms. El ambiente permanece visible. Después aparece la tarjeta de lanzamiento sobre el fondo despejado, con carátula, título y «Abriendo tu juego…». Secuencia normal total: 440 ms; no expresa porcentaje de carga del emulador. Movimiento reducido: presentación inmediata y cesión breve de 100 ms. B y el control táctil cancelan mientras la solicitud está pendiente; la interfaz recupera su opacidad en 140 ms (inmediato en movimiento reducido).

## Integración

- `launch_ui.rs` conserva la cola, liberación de audio y contrato de salida completa del proceso antes de iniciar el emulador. No se introduce un segundo propietario DRM/mandos/audio.
- `Home` separa la opacidad de sus controles de `Integrated.content-opacity`; `RoomScene` permanece independiente. Las posiciones, fondos y materiales existentes se conservan.
- El mismo worker de arte prepara una séptima ranura únicamente para el juego pendiente, identificada por ID exacto. La carátula ya visible se reutiliza inmediatamente cuando corresponde; la Biblioteca obtiene su recurso mediante el mismo worker. Sin descargas en la animación.
- El timeout de audio sigue siendo 2 s. Un fallo de preparación cancela; un fallo del ejecutable retorna a la interfaz. Los avisos del supervisor se preservan frente al refresco inicial del catálogo y también se muestran al volver a Biblioteca.
- La animación se detiene al cancelar, al ceder la pantalla o al fallar. No hay un porcentaje ficticio ni un spinner permanente.

## Puente de imagen experimental, desactivado por defecto

La animación anterior cubre el tiempo que EmotionalUI posee la pantalla. **No demuestra por sí misma continuidad hasta el primer fotograma del emulador.**

Se ha preparado `EMOTIONAL_LAUNCH_FRAMEBUFFER=1` únicamente para Linux + GPU, fuera del simulador. Solicita una lectura de 640×480 RGBA al terminar la transición (espera máxima adicional de 200 ms desde la solicitud). El supervisor, una vez terminado el proceso gráfico, puede mantener ese fotograma en el framebuffer fbdev de respaldo mientras inicia el emulador. Es una imagen de carga; no sustituye materiales ni simula transparencias dentro de la interfaz.

El helper usa solo consultas `FBIOGET_FSCREENINFO`/`FBIOGET_VSCREENINFO` y escritura de píxeles en `/dev/fb0`, comprobando dispositivo, dimensiones, stride y canales. No cambia modo, pan, brillo, controlador, servicios ni arranque. Acepta exclusivamente 640×480, 32 bpp truecolor, páginas virtuales acotadas de 480/960 líneas y canales RGB verificados. Conserva padding y copia de restauración en memoria; rechaza formatos desconocidos antes de escribir. Después de confirmar la salida del emulador restaura únicamente si la geometría sigue siendo la misma. Ante un proceso de vida incierta no restaura sobre él. Un puente no disponible deja funcionar el lanzador existente.

Fuente del ABI: [Linux UAPI fb.h](https://github.com/torvalds/linux/blob/master/include/uapi/linux/fb.h). La presencia de `rockchipdrmfb`, formato declarado de 32 bits, virtual 640×960 y acceso al dispositivo se consultó por SSH **sin escribir píxeles**. Eso no valida el puente: un emulador puede borrar su pantalla durante la inicialización o usar otra ruta de vídeo.

**Pendiente de prueba física supervisada:** continuidad visible, orientación/canales, primer fotograma real, retorno, restauración y funcionamiento por sistema/emulador. No activar este puente en el lanzador habitual antes de esa comprobación. La beta.6 instalada no se ha sustituido durante esta preparación.

## Reproducción local

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 cargo test --locked -p emotional-ui --no-default-features --features simulator
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo build --locked -p emotional-ui --no-default-features --features simulator
python3 tools/verify_launch_transition.py --binary target/debug/emotional-ui --output dist/launch-review
python3 tools/verify_player_flow.py --binary target/debug/emotional-ui --output dist/launch-player-flow
cargo test --locked -p emotional-services launch_frame
```

El verificador crea textos e imágenes propios en un directorio temporal. Comprueba cancelación sin solicitud residual, cesión tras la transición y retorno de un ejecutable deliberadamente ausente. Nunca abre DRM ni inicia un emulador físico. Las capturas privadas de las poses GPU de `focus_preview launch` muestran inicio, salida, fondo limpio y tarjeta; el GIF reúne poses estáticas y no acredita FPS ni duración real de presentación.

Los artefactos de revisión, la biblioteca privada y las carátulas descargadas permanecen en `dist/`, fuera de Git. La clave de SteamGridDB está fuera del repositorio y del paquete; se configuró por separado en la consola para una prueba supervisada.

## Prueba Docker solicitada

Se arrancó la VM ARM64 existente de Colima y se compiló una instantánea aislada de la transición. Pasaron cancelación, commit y fallo del ejecutable, además del ciclo normal/reanudar con procesos sintéticos en primer plano y guardados propios. El GIF privado `dist/docker-launch-results/apertura-docker.gif` usa los buffers RGB realmente rasterizados por Slint en Docker; incluye un rótulo explícito durante la fase del proceso sintético y pausas para facilitar su lectura. No muestra un emulador real ni acredita 60 FPS, DRM, LCD o desaparición física del negro entre procesos. Logs, hash del binario de Docker y procedencia están en ese directorio. La función SteamGridDB se añadió después de congelar esa instantánea y tiene sus propias pruebas; no atribuir al binario grabado una función posterior.

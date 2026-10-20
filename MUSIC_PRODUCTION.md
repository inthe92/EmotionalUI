# Producción musical · fase 9

## Auditoría

La fase 8 ofrece frases discretas, pero cuatro parciales con idéntica estructura de ataque por nota producen un timbre demasiado uniforme. Más reverberación no corrige la ausencia de martillo, cuerpo y decaimiento propios de un piano. Se conservan sus WAV/MIDI y se cambia únicamente el instrumento y la interpretación de las nuevas candidatas.

## Instrumento seleccionado

VSCO 2 Community Edition, piano vertical Upright No. 1, muestreado por **Simon Dalzell / Ivy Audio**, distribuido por **Versilian Studios / Sam Gossner**. Se utilizaron siete notas de la capa suave dyn1, registro raíz MIDI 49–73. Las muestras son CC0-1.0 según el [proyecto original](https://github.com/sgossner/VSCO-2-CE/blob/440300901dfe9275fd84e0b7763af1f8443ae62e/LICENSE) y la [página del editor](https://versilian-studios.com/vsco-community/). Se incluyen licencia, notas de procedencia y créditos; la petición de crédito del autor se respeta. No es un felt piano grabado ni una nueva interpretación humana al teclado.

Se evaluó también [Salamander Grand Piano](https://github.com/sfzinstruments/SalamanderGrandPiano): piano de cola con múltiples capas y licencia CC BY 3.0; requiere conservar atribución y su mapeo SFZ utiliza extensiones ARIA. No se incorpora. El vertical CC0 permite una cadena más pequeña para este registro íntimo. La síntesis anterior conserva trazabilidad completa, pero no se seleccionó como timbre de estas candidatas.

## Dos interpretaciones originales, no definitivas

**La luz que queda — A / Refugio**, 112 s: bajo y quinta espaciados ligeramente, gesto ascendente que desciende a una nota interior, respuesta en otro registro y final suspendido. Frases a 3,8 / 38,65 / 79,2 s. Más cercana y cálida.

**B / Silencio**, 128 s: misma familia de intervalos, menos notas y mayor separación; velocidades menores y brillo reducido. Frases a 5,3 / 48,9 / 98,15 s. No confundir esta composición con el modo Silencio del motor, que desactiva música y ambiente.

Son composiciones originales programadas, sin transcribir canciones ni progresiones distintivas de videojuegos. Las duraciones, velocidades y pequeñas desviaciones de afinación se especifican por nota; no hay aleatoriedad libre ni cuantización rígida. MIDI conserva alturas, velocidades y tiempos; el JSON añade microafinación y mezcla para reproducir exactamente el render. No se afirma que MIDI por sí solo reproduzca la mezcla.

## Cadena reproducible

1. `python3 tools/fetch_piano.py --cache CARPETA`: descarga exclusivamente WAV fijados por commit, tamaño y SHA-256 en sources.lock.json. No ejecuta código descargado ni instala instrumentos. Los originales permanecen fuera del repositorio.
2. Con Python que disponga de NumPy y FFmpeg instalado: `python3 tools/produce_music.py --sample-cache CARPETA --output CARPETA_NUEVA --ffmpeg /ruta/ffmpeg`. En este Mac se usó el Python del runtime de Codex y ffmpeg-full ya instalado; no se añadió una dependencia al frontend.
3. El productor verifica hashes, convierte muestras 24-bit estéreo a mono 22.050 Hz únicamente en producción, selecciona la raíz cercana, aplica pequeñas variaciones de tiempo/velocidad/duración, filtrado y reflexiones finitas discretas. No hay un sampler ni conversiones al navegar.
4. Genera MIDI, WAV PCM de clips y preescuchas, Ogg Vorbis y M4A para escucha móvil. Cada ejecución requiere destino nuevo; no sobrescribe trabajo.

Se conserva la dinámica entre notas. La ganancia de bus es fija por variante; no se normaliza cada nota ni se comprime. Un primer render demasiado bajo se archivó en docs/phase9/first-render-low-gain; se aumentó el bus por un factor común de cinco. Picos finales: A 0,1294 FS y B 0,0890 FS; aproximadamente 61,66 % / 66,34 % de muestras cero en sus preescuchas. No se rellena el silencio con ruido.

Fuentes: assets/audio/phase9/source/score.json; configuración de instrumento y licencia en instrument/; candidatos en candidates/{refugio,silencio}. Metadatos de versiones, mezcla y hashes en render-manifest.json. Se volvió a renderizar en el mismo host: WAV y MIDI idénticos; las codificaciones con pérdida pueden variar entre builds de encoder. La reproducción cruzada de la producción en otro toolchain aún no se certifica.

Para escuchar en la aplicación: `--audio-assets assets/audio/phase9/candidates/refugio/runtime`, o la carpeta equivalente de silencio. Se conserva el banco de fase 8 como predeterminado hasta la revisión humana; no se presenta la candidata A como aprobación artística. Ambiente meteorológico y ajustes existentes siguen siendo opcionales. El banco nuevo ocupa 3.616.200 bytes de PCM y usa el mismo decodificador/trabajador.

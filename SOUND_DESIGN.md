# Identidad sonora · fase 8

**La habitación permanece** es una maqueta original, no una banda sonora final. La intención es sugerir un instrumento distante en una habitación tranquila, sin exigir atención. La referencia a un lugar de descanso se utiliza únicamente como intención atmosférica; no se han usado partituras, grabaciones, samples ni transcripciones de videojuegos.

## Material compuesto

Tres motivos: Umbral (3 notas), Brasa (4) y Rastro (3). Registro MIDI 55–69, velocidades 22–38, ataques suaves y frases con espacio. La preescucha de 100 segundos coloca los motivos a los 4, 32 y 69 segundos; el resto incluye colas y silencio real. No hay percusión ni cuerdas. El piano se aproxima mediante parciales amortiguados e inarmonicidad leve, ataque de 25 ms y cuatro reflexiones discretas. **Es síntesis de estudio, no una grabación de felt piano**. La valoración artística y la escucha en los altavoces reales siguen pendientes.

## Política de reproducción

Minimalista es el valor inicial propuesto: primera frase tras 18 segundos, intervalos entre comienzos aproximadamente 42–113 segundos según momento y motivo. Ambiental comienza tras 5 segundos, con intervalos menores. Madrugada reduce presencia; las seis fases comparten los mismos tres motivos. El cambio de contexto afecta a la siguiente frase; no reinicia la actual ni altera su timbre a mitad de nota.

Lluvia sintética (18 s) y aire filtrado (16 s) son estudios discretos, no bucles ambientales continuos. La lluvia solo se programa con clima de lluvia válido; el aire con nubosidad y modo Ambiental. Sin recurso o clima fiable, silencio. Los ambientes comienzan, como pronto, a los 10 segundos; se espacian 55 s en Ambiental y 90 s en Minimalista.

Silencio apaga música y ambiente. Los sonidos de interfaz se controlan aparte y están desactivados por defecto; el volumen general a cero silencia todas las capas. No hay relación entre música, tiempo de permanencia, ranking o recompensas.

## Reproducción del material

`python3 tools/compose_audio.py` genera MIDI y WAV usando solo la biblioteca estándar. `--output-root CARPETA` permite regenerar sin tocar el repositorio. `tools/export_audio.sh CARPETA_NUEVA` exporta Ogg Vorbis y M4A con un FFmpeg instalado que disponga de libvorbis; no instala nada y rechaza sobrescritura. Se puede indicar `EMOTIONAL_FFMPEG=/ruta/ffmpeg`. La versión básica Homebrew presente no incluía libvorbis; se utilizó la instalación ffmpeg-full ya disponible.

Fuentes: `assets/audio/source/score.json`, MIDI en esa carpeta y generador en tools. Preescuchas: `assets/audio/previews/`. Recursos de ejecución: `assets/audio/runtime/`. El frontend reproduce PCM WAV local, no los Ogg/M4A de preescucha. Esta elección evita añadir decodificadores para solo 3,09 MB de PCM.

## Fase 9 · candidatas con piano vertical

La auditoría confirmó el límite del timbre aditivo anterior. Se produjeron A — Refugio y B — Silencio con muestras reales y trazables de piano vertical VSCO 2 CE, capa suave, interpretación programada y sin instrumentos adicionales. Las piezas son originales y comparten una familia de gestos con diferentes tiempos, velocidades y resonancias. Sus fuentes y parámetros están en assets/audio/phase9; MUSIC_PRODUCTION.md explica cadena y decisiones.

No se declaran definitivas sin revisión humana ni se reemplaza silenciosamente el banco anterior. Pueden probarse mediante --audio-assets apuntando a la carpeta runtime de cada candidata; no hay sampler ni generación en tiempo real. El motor, los tres modos y volúmenes permanecen. Las maquetas anteriores y el primer render de ganancia baja se conservan. La escucha/calidad artística y los altavoces de RG353V no se certifican mediante métricas de señal.

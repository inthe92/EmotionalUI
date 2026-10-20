# Procedencia de audio

Todos los recursos de fase 8 se han compuesto o sintetizado para EmotionalUI a partir de código y datos propios. No hay grabaciones de terceros, SoundFonts, modelos musicales, servicios de generación en nube, samples, melodías ni transcripciones de videojuegos.

| Recurso | Fuente | Estado |
|---|---|---|
| score.json y la-habitacion-permanece.mid | eventos originales de 3 motivos; generador SMF propio | composición editable, maqueta |
| umbral.wav, brasa.wav, rastro.wav | síntesis aditiva amortiguada de tools/compose_audio.py | timbre provisional tipo piano |
| rain.wav, air.wav | ruido determinista filtrado, semilla del score | ambientes de estudio |
| preescucha WAV 100 s | colocación de los tres motivos; silencio explícito | audio exportado real |
| preescucha Ogg Vorbis y M4A AAC | codificación local de ese WAV mediante FFmpeg instalado | formatos de escucha, no ejecución |
| cues de foco/confirmar/cancelar/panel | services/src/feedback.rs, PCM propio ya existente | reutilizados, opcionales |

Código, partitura y recursos originales se distribuyen bajo la licencia GPL-3.0-only del proyecto. No se atribuye a terceros la autoría de estos sonidos ni se presupone que una idea musical breve pueda tener exclusividad absoluta. Es necesaria revisión artística humana antes de llamar definitivo al timbre y a la composición.

`assets/audio/source/render-manifest.json` registra duración, muestras, pico y SHA-256 de WAV. Las pruebas regeneran WAV/MIDI en una carpeta temporal y comparan bytes. El Ogg se valida como Vorbis real; la codificación puede variar entre versiones de FFmpeg. Se documentó la exportación en `docs/phase8/ogg-export.log` y `aac-export.log`.

FFmpeg es una herramienta externa de producción, no se empaqueta ni ejecuta en el frontend; su build/licencia corresponde a la instalación utilizada. No se redistribuyen plugins ni instrumentos. ALSA y AudioToolbox pertenecen al sistema de destino; la aplicación no copia sus bibliotecas. La biblioteca libc ya estaba fijada en Cargo.lock. No se introdujo un framework multimedia adicional.

## Fase 9 · materiales con licencia separada

- Muestras originales de piano vertical: VSCO 2 CE, Simon Dalzell / Ivy Audio, Versilian Studios / Sam Gossner; CC0-1.0. Fuente fijada a commit 440300901dfe9275fd84e0b7763af1f8443ae62e; URLs, tamaños y hashes en assets/audio/phase9/instrument/sources.lock.json. Texto legal, Info/Readme y mapping incluidos. Los siete WAV originales están en caché externa; no se redistribuye la biblioteca completa.
- Composición La luz que queda, MIDI, interpretación programada, código y exportaciones nuevas de EmotionalUI: GPL-3.0-only, coherente con Cargo.toml. Las muestras CC0 mantienen esa licencia; no se atribuye al proyecto la grabación del instrumento. Se corrigió la mención previa or-later que no coincidía con la licencia declarada por el workspace.
- Recursos renderizados A/B: candidates/{refugio,silencio}; manifiesto con pico, duración, ceros, hashes y versiones del productor. WAV/MIDI reproducidos en el mismo host. Ogg/AAC son formatos de escucha, no dependencias de ejecución.

Fuentes verificadas: [editor](https://versilian-studios.com/vsco-community/) y [licencia upstream fijada](https://github.com/sgossner/VSCO-2-CE/blob/440300901dfe9275fd84e0b7763af1f8443ae62e/LICENSE). La licencia del sample, la composición, el programa y las herramientas de producción se documentan por separado. No se usa un instrumento comercial gratuito de permisos desconocidos ni material musical de videojuegos.

## Fase 12 · calibración diagnóstica

`platform/darkos/calibrate_audio.py` deriva un banco separado de los cinco WAV propios de fase 8. Una única ganancia de familia modifica los tres motivos de piano; lluvia/aire conservan exactamente sus bytes. No se añaden muestras ni se modifican notas, afinación, tiempos o instrumentos. Fuentes y parámetros quedan registrados por hash; la licencia de esos recursos originales sigue siendo GPL-3.0-only. Las candidatas muestreadas de fase 9 y el banco normal se conservan. Esta calibración sirve para diagnosticar nivel y salida física, no es una composición nueva ni una mezcla definitiva aprobada.

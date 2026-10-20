# Dependencias y licencias

El código y las ilustraciones originales de EmotionalUI se ofrecen bajo GPL-3.0-only. La elección permite utilizar la opción GPL de Slint en un frontend que puede evolucionar hacia un dispositivo embebido. No se presume que la licencia gratuita alternativa de Slint cubra cualquier distribución embebida. No se ha publicado ni distribuido este proyecto a terceros.

| Componente directo | Versión fijada | Licencia declarada |
|---|---|---|
| Slint / slint-build | 1.13.1 | GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0 |
| roxmltree (importador, ya presente transitivamente) | 0.20.0 | MIT OR Apache-2.0 |
| chrono | 0.4.42 | MIT OR Apache-2.0 |
| serde | 1.0.228 | MIT OR Apache-2.0 |
| rusqlite | 0.32.1 | MIT |
| SQLite bundled | mediante libsqlite3-sys del lockfile | Dominio público según SQLite |
| toml | 0.8.23 | MIT OR Apache-2.0 |
| image | 0.25.8 | MIT OR Apache-2.0 |

`Cargo.lock` fija las dependencias transitivas. `docs/dependency-licenses.tsv` recoge el inventario declarado por Cargo, incluidos paquetes de otras plataformas y dependencias opcionales presentes en la resolución. No equivale a una revisión legal ni indica que todos formen parte del binario de escritorio. Antes de distribuir: seleccionar las opciones de licencia pertinentes, entregar avisos y textos requeridos, facilitar el código correspondiente y revisar las licencias con identificadores LicenseRef y las dependencias incluidas en el binario final.

No se redistribuyen fuentes del sistema, ROMs, BIOS, carátulas comerciales, código de dArkOS ni scripts externos. Los enlaces a dArkOS se usan como documentación técnica. Las ilustraciones SVG se han creado para este proyecto. El feedback opcional genera PCM original en ejecución; no hay muestras comerciales ni archivos de audio externos.

Open-Meteo no es una dependencia ni hay datos meteorológicos descargados. Una futura integración necesita atribución visible en ajustes/créditos, revisión de condiciones y política de localidad/cache. Véase `docs/DARKOS_RESEARCH.md`.

## Herramientas y paquete ARM64 de fase 3

Zig 0.14.1 se descarga de la distribución oficial, con checksums fijados en `tools/arm64/zig.lock.json`; se conserva fuera del repositorio y del paquete de ejecución. El SDK Debian de referencia queda fuera del repositorio; `sysroot.lock.json` inventaría paquetes, procedencia y SHA-256. No se ejecutan sus maintainer scripts. No se redistribuye el SDK completo ni una imagen del sistema.

El paquete local experimental sí incluye `libseat.so.1` de Debian bookworm, libseat1 0.7.0-6 (MIT/Expat), porque falta en la consola observada. Su aviso completo Debian/upstream está en `licenses/libseat-copyright` del paquete, junto a la licencia GPL de EmotionalUI y el lock de procedencia. No se copian glibc, fuentes, ROMs ni BIOS. Las demás bibliotecas se resuelven desde el sistema existente. Para publicación externa sigue siendo necesaria la entrega de avisos y código correspondiente de la aplicación y revisar todos los componentes realmente distribuidos.

## Desarrollo Docker y controles de fase 5

`libc` 0.2.191 (MIT OR Apache-2.0), ya fijado en Cargo.lock, es dependencia explícita solo en Linux para evdev. No se ha añadido otra versión. Los replays, perfiles y PCM sintetizado son código/datos originales del proyecto.

Las imágenes locales de desarrollo usan Debian 13 y la imagen oficial Rust 1.99.1 con sus avisos de distribución; no forman parte del frontend ni se entrega una imagen de dArkOS. El inventario exacto apt de esta ejecución está en dist/simulator/packages.txt. DejaVu se utiliza desde el contenedor para capturas, sin copiar la fuente al repositorio. Las ilustraciones de la biblioteca importada son genéricas y propias; no se redistribuyen ROMs, BIOS ni carátulas externas.

## Actualización fases 6–7: fuente y escena embebidas

Corrección de las notas históricas anteriores: ahora sí se incluye `assets/fonts/DejaVuSerif.ttf`, copiada de fonts-dejavu-core del contenedor existente. `assets/fonts/DejaVu-COPYRIGHT.txt` conserva los avisos completos upstream/Debian. La fuente tiene condiciones Bitstream Vera/DejaVu; la licencia de empaquetado Debian mencionada en ese documento no convierte la fuente en GPL. Se debe entregar ese aviso con los binarios que la embeben. No se ha modificado ni renombrado la fuente.

`assets/scenes/room-v1/master.png` es una ilustración creada para EmotionalUI con la herramienta integrada imagegen, sin referencias comerciales. Su procedencia, prompt y límites artísticos están en PROVENANCE.md. No es una captura del software; las capturas de Fase 7 sí son renders reales de Slint usando ese recurso. No se afirma exclusividad sobre una imagen generada.

La auditoría de otros frontends no incorpora código de ellos. Véase REUSE_AUDIT.md para licencias verificadas y aquellas no resueltas. La copia local de fuentes, ilustraciones y código correspondiente debe acompañar cualquier futura distribución; no hay nueva dependencia gráfica o HTTP.

## Corrección de dirección visual

`assets/scenes/lofi-room-v2/master.png` adapta la referencia aportada por el usuario mediante imagegen para retirar interfaz y carátulas. Procedencia y prompt íntegro junto al recurso. La referencia original con carátulas no se incluye en el repositorio. El fondo adaptado es candidato; no se afirma exclusividad ni aprobación artística definitiva. Las capturas son renders de Slint, no el resultado del editor de imágenes presentado como software.

## Audio original de fase 8

Partitura, MIDI, síntesis de piano de estudio, lluvia y aire son recursos originales de EmotionalUI bajo GPL-3.0-only; no incluyen samples, SoundFonts ni música comercial. Inventario y cadena de producción en AUDIO_ASSET_PROVENANCE.md. FFmpeg/libvorbis/AAC se usan únicamente como herramientas locales de exportación, no se incorporan al ejecutable. El nuevo backend utiliza AudioToolbox del sistema macOS y carga ALSA del sistema Linux con libc ya existente. La aplicación no redistribuye esas bibliotecas ni introduce nuevas dependencias Cargo.

## Piano muestreado de fase 9

Las nuevas candidatas sí utilizan muestras externas: VSCO 2 Community Edition, Upright No. 1, CC0-1.0. Piano grabado por Simon Dalzell / Ivy Audio y distribuido por Versilian Studios / Sam Gossner; el README upstream acredita edición de muestras a Elan Hickler. Se respetan sus créditos. Licencia y documentos íntegros, commit, URLs y SHA-256 en `assets/audio/phase9/instrument/`. Los WAV fuente permanecen en la caché de producción externa; los exports incorporan su sonido.

La partitura original, MIDI, scripts y exports del proyecto se ofrecen bajo GPL-3.0-only; la licencia CC0 de las muestras se mantiene separada. No se presenta como música de terceros ni como interpretación humana grabada para EmotionalUI. NumPy y FFmpeg son herramientas de producción externas, no nuevas dependencias del ejecutable. Véanse MUSIC_PRODUCTION.md y AUDIO_ASSET_PROVENANCE.md.

## Emotional Atlas · fase 10

El piloto contiene 52 identificadores/títulos contrastados con una lista oficial Nintendo, sin ROMs, BIOS, imágenes, reseñas ni descripciones copiadas. La página no se trata como un dataset abierto: sus condiciones y alcance factual están en sources del JSON. Las marcas/nombres siguen perteneciendo a sus titulares. Esquema, herramienta y propuestas originales del proyecto bajo GPL-3.0-only. No se incorpora ninguna biblioteca ni modelo IA y no se añade un proveedor HTTP.

## Tipografía compartida · revisión visual de 19-10-2026

Se añaden DejaVuSans.ttf y DejaVuSans-Bold.ttf sin modificar, procedentes del mismo paquete fonts-dejavu-core del simulador local. Se embeben para mantener la misma familia/pesos en las dos vistas y plataformas. Aplican los avisos Bitstream Vera/DejaVu ya incluidos en assets/fonts/DejaVu-COPYRIGHT.txt; no se añaden librerías.

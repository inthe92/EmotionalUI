# Consolidación · inspección local de 19-10-2026

Referencia vigente: [especificación maestra](docs/EMOTIONALUI_MASTER.md#7-primer-punto-de-control-git--propuesta-no-ejecutada). No se publica, crea commit, hace staging ni modifica .gitignore/identidad Git.

- Git: main sin commits, cero archivos seguidos, sin remoto ni autor configurado. 2.530 archivos no ignorados al iniciar (2.226 bajo docs). El código sigue sin checkpoint; no confundir la nueva documentación con una copia versionada del ejecutable.
- Coinciden los 32 hashes del manifiesto de fuentes de la última revisión y el ELF local con su auditoría SDK. No es un análisis de todos los archivos ni prueba física.
- Escaneo acotado de 202 textos de app/core/services/tools/platform y Markdown raíz: sin coincidencias de los patrones de credenciales usados ni IPv4 privadas. Rutas personales en `ARTWORK_IMPLEMENTATION_REPORT.md:60`, `PHASE3_REPORT.md:7`, `DARKOS_INTEGRATION.md:52` y `ARM64_BUILD.md:42,47,48,49`. No se copian sus valores. No se escanearon todos los logs/artefactos privados ni se declara el árbol libre de secretos.
- GPL-3.0-only coincide en Cargo y LICENSE. Avisos de DejaVu, procedencia de audio y del fondo adaptado existen. El derecho de redistribución del fondo y notices transitivos siguen sin resolver; esta inspección no es una aprobación legal/publicitaria ni una licencia nueva.
- .gitignore preservado: no cubre todo el material reciente de validación. No añadir docs/recursos/logs en bloque. La primera propuesta se limita a los dos documentos expresamente listados en la especificación, tras decisión de autor/revisión. El checkpoint completo del código está condicionado a recursos y privacidad.

## Auditoría anterior conservada — evidencia histórica

# Auditoría para publicación

Git heredado: main sin commits, proyecto sin seguimiento. Se conserva todo; no hay reset/clean, push ni publicación. No se ha hecho add indiscriminado. La identidad de autor no está configurada y no se configura ficticiamente para producir un commit.

Escaneo local de código/recursos/textos con detección de rutas personales, IPv4 privadas, patrones de tokens/claves y extensiones de juegos/firmware. El informe privado docs/phase11/publication-audit.json guarda categoría/ubicación, sin copiar valores sensibles. Los registros históricos no forman parte de la lista pública candidata. Un escaneo por patrones no prueba ausencia absoluta de secretos.

Se encontró una ruta personal en la procedencia del recurso histórico room-v1. Se conserva el archivo original; no se incorpora a una publicación sin sanear una copia de distribución y mantener atribución. También se detectaron archivos desmaterializados por macOS: se solicitó su descarga local, sin cambiar su contenido. Los artefactos/reportes anteriores no se borraron.

.gitignore amplía exclusiones para resultados locales de fase 11, fixtures, .env, claves y formatos comunes de ROM. No basta como auditoría: el primer commit debe usar una lista explícita revisada. No se crea un commit público reproducible mientras queden derechos del fondo adaptado, revisión transitiva y autoría sin resolver. Preparar una lista candidata no equivale a aprobar su publicación.

Licencia del código ya elegida en fases previas: GPL-3.0-only, consistente con Cargo y el texto existente. Fuente DejaVu con copyright/licencia propios; muestras VSCO2 CE CC0 con créditos y lock; composiciones originales y generaciones conservadas. Los nombres de videojuegos y las fuentes factuales no incluyen ROMs ni portadas. La licencia de una web no se presenta como dataset abierto.

Materiales de Reddit y storyboard en docs/phase11/presentation son borradores. No se ha publicado texto, vídeo o repositorio. Un vídeo conceptual no se usará como evidencia de funcionamiento; el regreso a EmulationStation no se representará como probado.

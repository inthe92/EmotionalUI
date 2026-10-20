# Emotional Atlas · piloto SNES

Atlas es un catálogo editorial local versionado, separado de ROMs y del historial. No ejecuta IA en consola ni hace HTTP. Se conserva la base funcional actual para biblioteca/usuario y se añade un solo archivo SQLite editorial de solo lectura; no se crean library.db/user.db duplicadas.

Piloto 0.1.0-pilot: **52 identidades de obras** contrastadas contra la [lista oficial Nintendo](https://www.nintendo.com/en-gb/Games/Nintendo-Switch-download-software/Super-Nintendo-Entertainment-System-Nintendo-Classics-1632515.html). Selección con plataformas, aventuras/RPG, puzles, acción, deportes y estrategia. La selección por familias es una decisión de cobertura, no un género objetivo añadido a las fichas sin fuente específica. No es todo SNES ni una base de hashes de ROM.

La fuente acredita título/plataforma de las obras ofrecidas como reproducciones SNES; no acredita un dump, traducción, revisión, año original o desarrolladora. Esos campos permanecen null. Los 21 atributos interpretativos iniciales y una relación son **propuestas del asistente**, identificadas como tales, no respuestas de una API ni revisión humana. No se activan en el SQLite de ejecución. El piloto sirve para identificación, revisión y prueba de arquitectura; su cobertura emocional revisada sigue pendiente. No se afirma una mejora editorial demostrada sobre el ranking anterior.

Fuentes editables: assets/atlas/source/snes-pilot.json; esquema en assets/atlas/schema.json. Artefacto actual: assets/atlas/emotional-atlas.db, idéntico al archivo versionado emotional-atlas-0.1.0-pilot.db. El primer build se conserva como evidencia en docs/phase10/atlas-first-build.db. Código y propuestas originales GPL-3.0-only; los nombres son identificadores factuales y las marcas pertenecen a sus titulares. No se copian reseñas, descripciones ni imágenes. No se presenta el sitio de Nintendo como una base de datos bajo licencia abierta.

Uso: `--atlas RUTA_DB`. Para vincular un juego importado: `--db BASE --atlas RUTA_DB --atlas-link ID_LOCAL=atlas:snes:000001`. El enlace queda en atlas_links con procedencia explicit-user-link. Hay que conocer la identidad: no se realiza emparejamiento difuso ni se agrupan ROMs por parecido de nombre. El pipeline ofrece candidatos por título exacto/alias, que deben resolverse explícitamente.

La aplicación valida application_id, versión y perfiles; abre SQLITE_OPEN_READ_ONLY y query_only. Lee solamente los perfiles enlazados, no miles de JSON. Metadatos editoriales propios ya existentes prevalecen sobre los del Atlas; unknown nunca los borra. Una actualización del archivo no escribe preferencias ni historial. No se descarga una actualización automática ni se reconstruye el Atlas al iniciar Slint.

## Fase 11 · canal experimental separado

El piloto estable anterior se conserva íntegro. Nuevo snes-experimental.json / emotional-atlas-0.2.0-experimental.1.db: 21 fichas investigadas, mecánicas trazables e interpretaciones propuestas. `profiles` sigue vacío de atributos; `profiles_experimental` solo se lee con --atlas-experimental. No hay aprobaciones humanas automáticas. La clave de caché incluye el canal. Detalles en ATLAS_EDITORIAL_REVIEW.md.

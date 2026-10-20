# Esquema Atlas 1

Formato fuente JSON, schema=1, plataforma snes y versión semántica. ID estable `atlas:snes:000001` asignado una vez; un cambio de título no recalcula el ID. identity separa work_id, región, revisión y traducción. La región del piloto es unspecified: no se asume una ROM estadounidense por el título inglés. Alias con región/procedencia permiten variantes; una coincidencia ambigua devuelve candidatos, no un merge.

Cada ficha contiene objective, editorial y aliases. Objective declara explícitamente title/platform/year/developer/publisher/genre/mechanics/duration. Ausencia de conocimiento = null. Un atributo conocido contiene value, confidence (0–100), source, status y origin; admite note y reviewer. Estados: unknown, proposed, source_checked, approved, rejected. Son estados de revisión, no verdad universal. Una aprobación exige un revisor identificable. Las importaciones de IA no pueden autoaprobarse.

Editorial admite calma, intensidad, nostalgia, exploración, misterio, exigencia mental, tensión, ritmo, facilidad para retomar, sesiones breves y minutos de sesión; afinidades season.*, context.* y celebration.*. Valores 0–100, duración 1–240. Se compilan únicamente atributos approved con confianza >=70. No se deduce emoción del género. El valor desconocido no equivale a cero observado.

Relaciones dirigidas: exploración, ritmo, atmósfera, estructura de aventura, sesiones breves, descubrimiento e historia documentada. Exigen extremos válidos y procedencia; las propuestas siguen siendo propuestas. La primera relación del piloto no se usa en ranking.

SQLite: application_id=1161909324, user_version=1. meta contiene schema/version/source_sha256; games conserva fichas auditables, profiles expone TOML compatible con el Profile Rust existente, candidates precalcula pesos por contexto y relations conserva relaciones. El adaptador utiliza perfiles enlazados; la tabla candidates es un artefacto para inspección y futuras optimizaciones, no un segundo recomendador oculto. No hay imágenes ni paths de ROM en el Atlas.

La validación semántica tools/atlas.py complementa el JSON Schema: unicidad de IDs/ediciones/relaciones, tipos, rangos, fuentes existentes, contradicción de plataforma, revisión y objetivos verificados antes de publicar. El esquema distribuido es versión 1; no existe una migración Atlas de formato futuro aún. Se rechazan formatos desconocidos. La base personal mantiene su esquema 4 con tablas aditivas idempotentes, conservando datos de las fases anteriores.

Los IDs locales del importador siguen vinculados a su ruta, como antes: mover/renombrar contenido puede requerir remapear un enlace y su historial. Los IDs Atlas son independientes. No se fusionan automáticamente historiales/guardados de revisiones o regiones; el resolver devuelve ambigüedades para decisión explícita. La deduplicación global por hash o identidad de obra todavía no se implementa.

## Extensión compatible · fase 11

Schema fuente/SQLite sigue en 1; extensión opcional experiment con channel=experimental, selected_ids únicos, human_review=null, author y explanations por ID. Requiere mecánicas source_checked y procedencia HTTPS/notas de cada interpretación; no altera status=proposed. La tabla adicional profiles_experimental contiene perfil TOML y explicación; el adaptador normal sigue usando profiles. Así se mantienen lectores anteriores y el catálogo anterior, y el opt-in distingue canal en caché. Desconocidos no se convierten en valores activos.

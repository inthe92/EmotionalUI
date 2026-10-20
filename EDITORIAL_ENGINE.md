# Motor editorial — estado conservado

La corrección visual no sustituye el recomendador ni el importador. `core/src/editorial.rs` conserva un perfil versionado con intensidad, calma, nostalgia, misterio, exploración, exigencia mental, tensión, ritmo, duración, facilidad para retomar, afinidades estacionales/festivas y atributos adicionales. Son valores opcionales; desconocido sigue siendo `None`, sin inferencia automática por género.

`assets/editorial/catalog.toml` contiene anotaciones de seis juegos ficticios. `calendar.toml` define eventos offline por fecha, estación, fin de semana y región. La SQLite propia mantiene editorial separado de XML importado, elecciones/hábitos, historial y disponibilidad.

El ranking filtra IDs vacíos, duplicados y juegos ausentes. Continuidad, elecciones y hábitos pueden superar el contexto; la contribución editorial contextual está acotada a 24 puntos. Se han ampliado los pesos aprendidos y las regresiones para que acción de noche pueda superar una sugerencia tranquila. La personalización se puede desactivar y se ignoran elecciones/hábitos al puntuar. La renovación manual penaliza repetición sin excluir títulos por género.

Las recomendaciones se mantienen mediante restauración por ID; los títulos nuevos se añaden usando el ranking contextual ya calculado. Esta restauración usa un índice y evita el barrido cuadrático de la versión heredada. La selección y el ranking siguen ejecutándose en memoria; las escrituras se realizan en un worker con snapshots compactos de preferencias, no con copias de descripciones/arte/perfiles completos.

## Medición heredada de esta fase

`docs/phase7/performance-before-lookup.json` y `dist/simulator/performance-phase7.json` permiten repetir 1.000/5.000/10.000 juegos. La búsqueda de última sesión pasó de aproximadamente 296 ms a 5 ms con 10.000 entradas al indexar IDs; la lectura de preferencias SQLite, de aproximadamente 116 ms a 28 ms usando tres lecturas agrupadas. Son muestras debug de Linux ARM64 virtualizado, no resultados RK3566. El benchmark conserva su alcance y separa encolado de escritura y flush en worker.

## Límites

No todos los atributos almacenados tienen aún una contribución específica al ranking. Amanecer/atardecer y preferencias explícitas de energía no están conectados a la GUI. Clima disponible proviene de configuración/simulación y caché existente, sin cliente meteorológico HTTP. El catálogo importado utiliza valores prudentes cuando no tiene anotaciones. No hay IA obligatoria, cuenta ni telemetría.

La estrategia de scraping sigue diseñada en REUSE_AUDIT.md; no se ha creado un scraper ni descargado arte comercial. La siguiente integración funcional debe ser un emulador y un historial de sesiones fiable antes de ampliar señales o servicios de red.

## Fase 10 · continuidad y Atlas

La continuidad se separa en core/continuity.rs y usa duración/recencia/regresos sin suponer saves. El recomendador editorial anterior permanece; rank_affinity añade una señal explícita local de -100/0/+100 puntos y conserva el límite contextual. Preferencias reales pueden superar la ambientación. Contextos horarios/lluvia con atributos aprobados se expresan mediante Profile.extra; unknown no contribuye.

La GUI carga perfiles Atlas enlazados de forma explícita, respetando overrides propios. Las ejecuciones confirmadas alimentan un mínimo de elecciones y hábitos por franja cuando existe un timestamp plausible; errores, foco y fijación sin historial no cuentan como sesiones. Con personalización desactivada esos conteos no contribuyen al ranking.

Con --atlas se prepara/resume una reserva máxima de 96 IDs y selecciones persistentes por contexto. No se vuelve a ordenar toda la biblioteca al mover foco. Si se actualiza solo uso/disponibilidad, se intentan conservar las propuestas anteriores que siguen presentes en la reserva; cambios explícitos de afinidad o versión Atlas permiten una nueva selección. La cache no usa GitHub, meteorología futura ni IA en runtime.

El piloto SNES tiene 52 identidades y propuestas pendientes de revisión: no proporciona todavía una mejora emocional demostrada. La comparación automática, sus límites y la revisión manual pendiente están en RECOMMENDATION_EVALUATION.md.

## Fase 11

Subconjunto experimental trazable y opt-in, sin cambiar el ranking general para forzar ganadores. Se conecta extra.short_sessions (antes no consumido) con un aporte positivo de hasta diez puntos solo en sesiones <=15 minutos. No inventa duración, abandono ni guardados. Los pesos personales y continuidad conservan mayor importancia. Comparación, límites y revisión humana en ATLAS_EDITORIAL_REVIEW.md.

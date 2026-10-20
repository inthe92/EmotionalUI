# Revisión editorial SNES · experimento 0.2.0-experimental.1

Se conservan las 52 identidades del piloto y el artefacto publicado 0.1.0-pilot, sin alterar. Se investigaron **21** de esas obras utilizando páginas primarias de Nintendo/las editoras. URLs, fecha de consulta, paráfrasis factuales y razonamiento están en `assets/atlas/research/snes-21.json`; no se redistribuyen páginas, carátulas, manuales ni reseñas.

Muestra: Super Mario World, Yoshi's Island, Super Mario Kart, Zelda: A Link to the Past, Super Metroid, Star Fox, F-ZERO, Pilotwings, Kirby's Dream Course, Super Ghouls'n Ghosts, Demon's Crest, Breath of Fire, Kirby Super Star, Breath of Fire II, Panel de Pon, Wild Guns, Donkey Kong Country, Mario's Super Picross, EarthBound, Harvest Moon y Sutte Hakkun. Abarca clásicos, puzles menos presentes, RPG, acción exigente y propuestas pausadas. No existe una medida de popularidad real en estas fichas ni se afirma que todo juego menos conocido sea mejor.

Las fuentes describen mecánicas. Los valores de calma, ritmo, tensión, exploración y facilidad para retomar son **interpretaciones del asistente**, confianza provisional 65/100, con explicación individual. No son emociones observadas en usuarios. Nostalgia personal, duración total, fecha original, desarrolladora y editorial histórica permanecen desconocidas cuando no se investigaron específicamente. Una función original de guardado descrita para Picross no demuestra que exista un guardado del usuario.

Flujo explícito:

1. Propuesta: anotación del asistente y fuente; `status=proposed`, sin reviewer.
2. Validación automática: tipos, límites, fuente HTTPS, notas, evidencia de mecánicas, IDs y relaciones. Los tests son validación de contrato, no juicio humano.
3. Evaluación experimental: manifiesto `experiment.selected_ids`; nunca convierte atributos en approved. Se compila en `profiles_experimental`, separado de profiles.
4. Revisión editorial humana: **pendiente**, campos de revisión vacíos. Debe valorar hechos, interpretación y selecciones, pudiendo rechazar cada atributo.
5. Aprobación/publicación estable: no realizadas. Importar nuevos resultados IA elimina la activación experimental, para que no hereden confianza anterior.

Activación en GUI: `--atlas assets/atlas/emotional-atlas-0.2.0-experimental.1.db --atlas-experimental`, con enlaces explícitos de identidad local. Omitir la última opción conserva perfiles aprobados vacíos. La versión de caché distingue ambos canales. No se emparejan automáticamente ROMs por nombres parecidos. Las fichas se refieren a obras, no certifican revisiones/regiones ni traducciones.

“¿Por qué este juego?” muestra una nota original identificada como Atlas experimental. Una preferencia personal solo se menciona si existe una decisión registrada y la personalización está activa; fijación/historial tienen explicaciones propias. Las relaciones nuevas entre puzles o aventuras son propuestas de estructura, nunca influencias históricas afirmadas.

Resultado inicial: dos de diez escenarios cambian; siete títulos distintos en las veinte posiciones frente a cinco antes. La repetición de los mismos candidatos sigue siendo importante. No hay mejora humana demostrada. Ver RECOMMENDATION_EVALUATION.md y el CSV sin calificaciones.

La confianza 65 es una etiqueta provisional de la rúbrica del asistente, no una probabilidad calibrada ni una puntuación aportada por jugadores. Debe revisarse atributo por atributo antes del canal estable.

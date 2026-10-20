# Evaluación de recomendaciones · fase 10

Se ejecuta `cargo run -p emotional-ui --example phase10_evaluate --locked`. Produce un worksheet reproducible sobre las 52 identidades del piloto: diez escenarios, selección pseudoaleatoria de semilla fija, proxy fijo de fama, ranking anterior y ranking con perfiles Atlas compilados. El proxy usa dos IDs explícitos; **no es una medición de popularidad real**. Las preferencias/actividad de estos escenarios son fixtures, no información de una persona.

Escenarios: domingo lluvioso de otoño, tarde de verano, noche nublada de octubre, mañana invernal, sesión breve, preferencia por una aventura seleccionada, preferencia por un arcade seleccionado, poco historial, retorno a un título antiguo y pruebas sin continuidad. Preferencia por familias es el nombre del caso; los perfiles solo incorporan los IDs explícitamente elegidos, sin inventar un género objetivo en las fichas.

Resultado observado: todos los perfiles editoriales de producción siguen sin atributos activos. Por ello los diez resultados Atlas son iguales a los del ranking anterior; no hay evidencia de mejor coherencia contextual ni calidad humana gracias al piloto. La repetición de algunos IDs se ve en evaluation-summary.json: hay que investigar la calidad de selección con perfiles revisados, no interpretar un empate de datos desconocidos como una joya descubierta. La disponibilidad y las preferencias registradas sí se respetan; no existe una variable de popularidad en el recomendador.

La reserva usa escenarios y los tests verifican estabilidad, disponibilidad, preferencias, diversidad por penalización y caducidad. Las relaciones y etiquetas propuestas no se activan para aparentar un cambio de ranking. El benchmark con perfiles sintéticos aprobados mide coste de cálculo, no interés artístico de esas recomendaciones.

Artefactos: docs/phase10/recommendation-evaluation.json, evaluation-summary.json y manual-review.csv. El CSV conserva vacías las puntuaciones humanas de afinidad contextual, diversidad, relevancia, explicación y notas. No se han inventado reseñas ni se ha realizado una sesión de revisión humana.

Siguiente evaluación: revisar un subconjunto diverso de fichas con fuentes de mecánicas y responsable de aprobación, recompilar, repetir estos escenarios y valorar a ciegas las cuatro selecciones con una persona. Evaluar variedad entre sesiones y el efecto de preferencias de aventura/arcade, incluyendo no favoritos. No imponer cuotas de juegos desconocidos ni corregir pesos solo para que pase una selección predeterminada.

## Evaluación vigente · fase 11

`cargo run -p emotional-ui --example phase11_evaluate --locked` conserva los diez escenarios, 52 identidades, fixtures personales, semilla y proxy anterior. Compara perfiles aprobados vacíos con el subconjunto experimental de 21 fichas. Se conectó la señal short_sessions antes no utilizada: aporta hasta diez puntos si la sesión solicitada es <=15 min, sin inventar una duración. Los coeficientes de continuidad, decisiones y penalización anteriores se conservan. No se afina el ranking para obtener títulos predeterminados.

Resultado ARM64 Docker: cambian **2/10** pares (domingo lluvioso y sesión breve). Veinte posiciones muestran **7** identidades distintas frente a **5** en el ranking anterior; selección pseudoaleatoria 20 y proxy fijo 2. La selección aleatoria es más diversa por esta métrica y no se oculta. Los otros ocho casos, incluidas preferencias explícitas y retorno a una partida antigua, se mantienen. Los desconocidos no son expulsados ni se impone cuota de “joyas”.

No demuestra superioridad artística: la repetición de Nobunaga/Sutte Hakkun sigue siendo alta, la cobertura experimental es desigual (21/52), el hash de desempate favorece ciertos IDs y estos casos tienen escaso historial. No se corrigen pesos para maquillar esa limitación. Clima nublado no suma afinidad porque no se inventaron atributos compatibles; las estaciones/celebraciones siguen disponibles en esquema, pero no se rellenan sin justificación editorial. Las propuestas no incorporan una popularidad real.

Comparación legible y 40 filas para revisión en docs/phase11/evaluation-review/{comparison.md,human-review.csv}. Campos humanos vacíos. Evaluar coherencia, interés, explicación respaldada y descubrimiento con personas, sin revelar antes qué método produjo cada pareja. Una valoración humana a ciegas todavía no se ha ejecutado. El resumen automático no representa votos de usuarios.

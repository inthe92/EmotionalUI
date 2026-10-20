# Producción editorial fuera de la consola

Python estándar, sin cuenta ni proveedor IA requeridos. No ejecuta peticiones de red ni código recibido. Cada salida exige una ruta nueva; nunca sobrescribe una ficha/base anterior.

```
python3 tools/atlas.py validate assets/atlas/source/snes-pilot.json
python3 tools/atlas.py requests assets/atlas/source/snes-pilot.json --output lote-nuevo.json
python3 tools/atlas.py import-results assets/atlas/source/snes-pilot.json --results respuestas.json --output propuestas-nuevas.json
python3 tools/atlas.py build propuestas-revisadas.json --output atlas-nuevo.db
python3 tools/atlas.py resolve assets/atlas/source/snes-pilot.json --title "Super Mario World"
```

import-list admite CSV title,source,region mediante --results y asigna IDs nuevos consecutivos sin cambiar los anteriores. Produce títulos pendientes, campos desconocidos explícitos y ediciones; esos títulos no son publicables hasta contrastarlos. resolve solo propone coincidencias exactas con título/alias. Regiones, revisiones y traducciones requieren investigación; no hay identificación automática por dump.

requests prepara entradas de trabajo, no resultados simulados de un modelo. Cada resultado externo debe traer id y editorial con atributos estructurados y fuentes ya registradas. import-results fuerza proposed (o unknown para null), retira reviewer y rechaza sobrescribir atributos aprobados. No basta con que un modelo diga approved. La revisión se hace editando un diff de JSON con fuente, confianza, estado y responsable; no existe un servicio de revisión humana implementado. Contribuciones: mantener ID, cambiar solo hechos/propuestas pertinentes, validar y adjuntar fuentes; comparar JSON antes de regenerar.

build valida contradicciones/duplicados y conserva solo atributos aprobados con confianza suficiente en profiles. Incluye el JSON completo para trazabilidad. SQLite reproducible en el mismo toolchain, sin timestamp variable, VACUUM y permisos 0444. Fuente JSON + hash canónico y versión permiten regenerar. SQLite de solo lectura no sustituye una firma de actualización; no hay descarga automática. Las pruebas comparan dos construcciones byte a byte y rechazan escritura/overwrite.

La prueba integrada usa un Atlas sintético claramente identificado con aprobaciones de fixture, no personas reales. El piloto SNES actual conserva todas sus interpretaciones como propuestas; ningún test convierte esas propuestas en validación humana.

Fuentes: Nintendo aporta solo identificación factual de título/plataforma. Su página sigue protegida; no descargar imágenes ni descripciones masivas. No se consultaron ScreenScraper/TheGamesDB ni APIs IA en esta fase. El pipeline admite incorporar fuentes autorizadas con condiciones registradas, sin sustituirlas por información inventada.

## Experimento reproducible · fase 11

`python3 tools/build_atlas_experiment.py SALIDA_NUEVA.json` reconstruye las anotaciones investigadas de assets/atlas/research/snes-21.json sobre el piloto intacto. Luego `tools/atlas.py build SALIDA_NUEVA.json --output BASE_NUEVA.db`. No consulta una API ni Internet al compilar. Importar resultados externos retira el manifiesto experimental; la propuesta importada necesita volver a evaluarse explícitamente.

Separar validación automática, selección experimental, revisión humana y aprobación. Ninguna de las dos primeras produce la segunda clase de revisión. El CSV de evaluación conserva campos humanos vacíos. No se añaden fuentes sin consulta ni aprobación implícita por tener JSON válido.

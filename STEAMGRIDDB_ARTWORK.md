# Carátulas verticales comunes · SteamGridDB

Preparación externa para todos los sistemas, usando el worker/caché de imágenes actual. No cambia geometría de la home, fondos, materiales, datos de Atlas ni lanzadores. No instala nada automáticamente.

- Formato maestro: **600 × 900, 2:3**, PNG/JPEG estático. SteamGridDB lo denomina Steam Vertical.
- Recurso para RG353V: **240 × 360 PNG**, proporciones intactas, sin recortes ni estiramiento. El original se conserva en la caché del Mac.
- Descargas secuenciales fuera del frontend. Clave solo para API; nunca para CDN, consola o manifiestos. No se siguen redirecciones.
- Identidad: ID exacto de EmotionalUI + sistema + título del inventario → ID de juego SteamGridDB revisado. Una búsqueda nunca autoriza la primera coincidencia. Revisar edición, plataforma y remakes; la API de búsqueda no certifica la plataforma.
- Una vez revisado el juego se elige el grid estático con mayor puntuación (empates por ID) o un `grid_id` explícito. La puntuación no equivale a revisión artística. Excluidos humor, contenido adulto y epilepsia en la consulta.
- Falta de coincidencia/carátula válida: conservar arte existente. No se garantiza cobertura completa de SteamGridDB. No se transforman cajas horizontales en portadas verticales falsas.
- Fuente, autor, ID de juego/grid, hashes y URL se conservan para trazabilidad. Descargar no concede derechos de redistribución; arte, inventarios privados y clave quedan fuera de Git.

## Uso en Mac

Python 3.11+ y Pillow (ya disponibles en el runtime de este entorno). Ejecutar desde la raíz del repositorio. Sustituir `python3` por el Python del runtime si el del sistema es anterior a 3.11.

1. Iniciar sesión en [SteamGridDB y obtener una clave personal](https://www.steamgriddb.com/profile/preferences/api). En una terminal privada:

```sh
python3 tools/steamgriddb_artwork.py configure
```

La entrada está oculta y se guarda con permisos 0600 en `~/.config/emotional-ui/steamgriddb.key`. No pegar la clave en el chat, argumentos, archivos del repositorio o registros. Alternativa: variable de entorno `STEAMGRIDDB_API_KEY` ya configurada de forma privada. No se accede a contraseñas de Steam ni se automatiza el inicio de sesión.

2. Preparar una copia coherente de **nuestra** SQLite mediante la API de backup de SQLite, sin copiar a ciegas una base activa con WAL. El inventario solo lee esa copia y conserva IDs originales; no abre ROMs. La presencia física de los juegos no se verifica al trabajar desde una copia remota.

```sh
python3 tools/steamgriddb_artwork.py inventory --db /ruta/copia-propia.sqlite --output dist/sgdb/inventory.json
python3 tools/steamgriddb_artwork.py search --inventory dist/sgdb/inventory.json --system snes --limit 20 --offset 0 --output dist/sgdb/snes-000.json
```

Lotes de 1–100 juegos, paginables con `--offset`. Omitir `--system` recorre todos los sistemas del inventario. El nombre del sistema se conserva tal como lo exporta la base, sin adivinar equivalencias. El buscador conserva hasta 20 candidatos por título, sin borrar etiquetas de hacks/variantes ni aprobar nada.

3. Revisar `matches`: mantener `game_id`, `system` y `title`; elegir `sgdb_id` correcto y establecer `approved: true`. Opcionalmente fijar `grid_id`. Dejar dudas sin aprobar. El hash del inventario impide aplicar propuestas a otro catálogo. Se pueden reunir lotes del **mismo** inventario en una lista `matches`, sin IDs duplicados.

```sh
python3 tools/steamgriddb_artwork.py fetch --inventory dist/sgdb/inventory.json --plan dist/sgdb/snes-000.json --cache dist/sgdb/cache --output dist/sgdb/manifest.json
python3 tools/steamgriddb_artwork.py export --manifest dist/sgdb/manifest.json --cache dist/sgdb/cache --device-root /home/ark/.local/share/emotional-ui-player/steamgriddb --output dist/sgdb/overrides.toml
```

La caché se reutiliza tras interrupciones y sus hashes se comprueban. Un error de autenticación, red, límite, formato o checksum detiene el lote; no publica silenciosamente un resultado completo. Un juego revisado sin grid compatible se registra en `missing_portrait`. Las selecciones ya cacheadas quedan fijadas; usar otra caché o fijar otro `grid_id` para preparar una revisión. No hay actualizaciones automáticas ni garantía de ranking/cobertura.

## Activación reversible pendiente

El exportador **prepara**, no copia a la consola ni modifica lanzadores. Antes de activar: comprobar IDs contra la biblioteca actual, revisar una muestra visual de distintos sistemas, copiar exclusivamente `cache/portraits/` al subdirectorio `portraits/` de la raíz elegida y verificar hashes. Los originales maestros pueden quedarse en el Mac. Configurar `EMOTIONAL_ARTWORK_OVERRIDES` para una ejecución independiente con el TOML exportado.

El worker existente da prioridad a estos recursos y conserva alternativas locales si un archivo falta/falla. Desactivar la variable y reiniciar restaura la selección anterior. Si ya existe una selección manual, conservarla y resolver explícitamente sus coincidencias antes de reemplazar ese archivo; el exportador no mezcla ni sobrescribe esa configuración.

La beta.6 instalada acepta overrides de hasta **256 KiB**. El código de desarrollo amplía el límite a **4 MiB**, en el mismo worker, para un catálogo de varios sistemas; probado con 10.000 IDs. Un manifiesto grande requiere compilar e instalar esa revisión antes de activarlo. No se ha sustituido la beta instalada.

## Verificación y límites actuales

### Selector nativo por juego

Ruta implementada: **Opciones del juego → SteamGridDB · Carátula**. Consulta el nombre al abrir; el usuario elige explícitamente la obra/edición y después una carátula. El panel muestra una vista vertical previa antes de aplicar. X permite cambiar la búsqueda con cruceta/A y teclado táctil; Y restaura la selección original del juego; B vuelve sin aplicarla. El contexto del juego y sus opciones se conservan.

API, descarga, validación, reducción a 240×360 y escritura se realizan fuera del hilo de entrada. Las peticiones tienen un límite de 30 s; cerrar el panel cancela el proceso de red. No se hace ninguna búsqueda durante la navegación normal. No se envían rutas de ROMs: solo el texto de búsqueda, el ID de SteamGridDB y la URL de imagen validada.

La elección nativa se guarda por ID exacto en `~/.local/share/emotional-ui-player/artwork-choices.json`, separado de los XML/SQLite originales. El worker de arte recarga esa selección cuando cambia el archivo y se invalida la presentación; el resto de juegos se conserva. Restablecer quita únicamente esa preferencia, sin borrar arte ni descargas. La procedencia del recurso descargado queda junto a la caché. Una preferencia corrupta se conserva sin sobrescribir; un recurso inválido mantiene el fallback.

El payload incorpora `steamgriddb_rpc.py` y el mismo cliente HTTP externo como `steamgriddb_client.py`. El helper de consola solo usa Python estándar, compatible con Python 3.10; la normalización usa el decodificador Rust existente. No requiere Pillow ni instalar paquetes en dArkOS.

La API necesita una clave privada en **cada equipo que vaya a consultar**, en `~/.config/emotional-ui/steamgriddb.key` con permisos 0600. La clave se configuró por separado en la consola para la prueba supervisada; no está incluida en el paquete. El selector muestra un fallo recuperable si no hay clave/red; el resto del frontend funciona sin esa integración. `EMOTIONAL_ARTWORK_CHOICES` permite aislar las preferencias durante pruebas; el helper alternativo `EMOTIONAL_SGDB_HELPER` se admite únicamente en builds de desarrollo/simulador.

Verificador: `python3 tools/verify_artwork_picker.py --binary target/debug/emotional-ui --output dist/picker-review`. Usa un RPC e imágenes propios: elige deliberadamente el segundo resultado, aplica, reinicia y restaura, comprobando que las fuentes no cambian. Una prueba adicional con el RPC real consultó Super Metroid y cargó la vista previa autenticada sin aplicar esa elección. No certifica el controlador o el Wi-Fi de RG353V.

Verificación local del 28-10-2026: **14 tests Python superados**, con imágenes sintéticas propias y API simulada (filtros/autenticación, IDs/sistemas, archivos inalterados, normalización, animaciones, caché/hashes, exportación y fallback). **25 tests Rust de artwork superados**, incluyendo prioridad en ambas presentaciones, regreso al arte original y carga acotada de 10.000 IDs. Compilación del target de tests `simulator` correcta; no equivale a compilación ARM64 ni prueba física. Los JSON locales admiten hasta 32 MiB de procedencia y cada respuesta API hasta 4 MiB.

**Comprobación en vivo del 28-10-2026:** clave guardada con permisos privados fuera del proyecto; búsqueda autenticada y filtros de grids correctos. Se descargaron y revisaron visualmente 11 carátulas de juegos asociados explícitamente a 9 sistemas, todas con maestro 600×900 y derivado 240×360. API y CDN funcionaron con cabeceras separadas. Manifiesto, procedencia, caché y overrides preparados en `dist/sgdb/`, sin publicar arte ni datos de la biblioteca.

**Estado físico:** el usuario confirmó la selección y aplicación de una carátula en RG353V con el binario `474cdc2bf5a7`. Pendiente: ampliar la cobertura con coincidencias revisadas. El lote inicial no acredita cobertura total ni rendimiento físico. La consola conserva su beta y carátulas anteriores.

Paquete de revisión privado preparado en `dist/emotional-transition-sgdb-review-20261028/`, con binario AArch64 `474cdc2bf5a7724ae7c863b291900d02bd76201343d4eb481d351b20f4440dd4` (GLIBC máximo 2.35; sin RPATH). Manifiestos e importación de helpers verificados; no incluye clave ni arte descargado. 134 tests de la aplicación, 17 tests Python del proveedor/RPC y la prueba de empaquetado superados. La prueba nativa completa verifica selección explícita del segundo resultado, aplicación/persistencia, restauración y cancelación del proceso de consulta. La comprobación física se dejó pendiente por indicación expresa del usuario.

## Fuentes técnicas

- [API v2 oficial](https://www.steamgriddb.com/api/v2).
- [Wrapper oficial: endpoints, Bearer, dimensiones y filtros](https://github.com/SteamGridDB/node-steamgriddb/blob/master/src/index.ts). Implementación propia, sin copiar el cliente.
- [Catálogo oficial: 2:3 / 600×900 y formatos](https://www.steamgriddb.com/collection/9492/grids).

El cliente limita las imágenes a HTTPS en los hosts CDN de SteamGridDB previstos y al directorio `/grid/`, PNG/JPEG. Si el servicio cambia de host/ruta o devuelve un formato distinto se detiene, en vez de ampliar automáticamente el acceso a otros servidores. Verificar y adaptar esa compatibilidad con la primera respuesta real.

# Caché gráfica

El worker existente es único. Entrada y salida tienen capacidad 1; el frontend permite una petición pendiente y descarta una respuesta si su ID/vista/generación ya no corresponde. Se preparan solo protagonista y entradas visibles. Un cambio del modelo importado invalida las peticiones resueltas, incluso si el ID del juego sigue igual. Esto corrige la carrera de carga del segundo arranque.

## Identidad y escritura

Clave: versión `editorial-v1`, ID exacto del juego, ruta del recurso (canónica en importados), contenido del archivo, dimensiones, presentación y modo. Un cambio de bytes invalida aunque se conserve el tamaño del archivo. No se usa nombre de ROM aproximado. La misma baldosa del mismo juego se reutiliza en las dos secciones.

La firma usa `DefaultHasher` de la toolchain fijada. Es una identidad de caché derivada, no un checksum criptográfico de autenticidad ni un formato estable de distribución. Cambiar el algoritmo/toolchain permite regenerar la caché.

Destino por defecto: `$XDG_CACHE_HOME/emotional-ui/artwork-v1`, o `~/.cache/emotional-ui/artwork-v1`. `EMOTIONAL_ARTWORK_CACHE` permite un directorio propio absoluto. Se rechaza un destino dentro de una raíz de recursos, resolviendo también padres que aún no existen y alias como `/var → /private/var`.

Temporal exclusivo `.part` → PNG completo → `sync_all` → renombrado atómico. Los parciales no cuentan como hits. Un fallo de escritura no bloquea el uso de la composición en memoria. La publicación es atómica, pero no se promete durabilidad del directorio ante pérdida de alimentación. Los parciales abandonados se ignoran; todavía no existe un recolector específico para esos huérfanos.

## Presupuestos

- Disco: 64 MiB por defecto; `EMOTIONAL_ARTWORK_CACHE_MIB=1..256`.
- Limpieza por antigüedad de generación, al publicar; solo archivos regulares con nombre exacto `editorial-v1-<16 hex>.png`. No sigue symlinks ni elimina archivos personales con otros nombres.
- Memoria: LRU de píxeles preparados, máximo 4 MiB, además de una composición/decodificación en curso y las imágenes visibles de Slint. No equivale al RSS total.
- Un caché hit vuelve a validar el recurso fuente y su contenido; evita decodificar y componer, pero conserva un coste de lectura. No se presenta como acceso gratuito en microSD.
- Destinos GUI actuales: 282×224, 122×174 y 64×42. No se retienen imágenes originales decodificadas.

Un trabajo ya iniciado no interrumpe el decodificador; su resultado obsoleto se descarta. La cola está acotada y se drena a la petición más reciente cuando hay varias. Las medidas y el generador reproducible están en `docs/editorial-artwork-2026-10-19`; todavía falta medir microSD y consumo físico en RG353V.

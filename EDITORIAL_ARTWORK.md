# Arte editorial local · 19-10-2026

La imagen horizontal es un contenedor de EmotionalUI, no un requisito de la biblioteca. No se descarga arte, no se ejecutan comandos con metadatos y no se cambian originales. Se amplía el worker de Fase 11 con la dependencia `image` ya fijada; no se añade un framework.

## Recorrido implementado

`gamelist.xml → imported_games.metadata (SQLite) → Store::art_paths → Resource[] → worker único → selección/composición/caché → EditorialArtwork (Slint)`.

El importador existente ya conserva los campos XML en TOML dentro de SQLite. No se migra ni duplica el catálogo. Las rutas siguen asociadas al ID exacto importado, sin búsquedas por parecido de nombre. Solo se publican recursos de juegos disponibles dentro de la raíz autorizada.

| Entrada opcional | Interpretación |
|---|---|
| `fanart`, `emotionalui_hero` | Candidato a hero, solo si tiene dimensiones utilizables |
| `boxart` | Carátula explícita |
| `screenshot`, `titleshot` | Captura, escalado nearest |
| `image`, `thumbnail` | Semántica desconocida: vertical compuesta; otras proporciones tratadas conservadoramente como captura |
| `marquee` | Logo; por ahora se conserva la referencia y se muestra la alternativa tipográfica |

Solo `<image>` era consumido anteriormente. Los demás campos son opcionales; su presencia no se presupone en ninguna instalación de dArkOS. No se ha vuelto a inspeccionar la consola durante esta intervención.

## Cuatro presentaciones

1. **Hero landscape:** imagen completa a escala uniforme, con fondo derivado sobrio para las bandas. Se evita recortar personajes/logos; no hay recorte automático destructivo.
2. **Boxart composed:** carátula completa centrada, margen de 10/8 px, fondo derivado reducido y desenfocado una sola vez en el worker. No se distorsiona.
3. **Screenshot feature:** imagen contenida, bandas oscuras, nearest; al ampliar una captura pequeña se prefiere un múltiplo entero. No se difumina la captura.
4. **Typographic editorial:** título y categoría disponible sobre fondo salvia oscuro. Slint lo presenta mientras carga, cuando falta arte o cuando falla la validación. No se inventan ilustraciones ni etiquetas.

Prioridad hero: arte horizontal explícito válido → carátula vertical → captura → texto. En baldosas, captura precede a carátula. Dentro de una prioridad se prefiere mayor resolución útil. Un hero demasiado pequeño/estrecho no gana por llamarse `fanart`. Una imagen genérica horizontal no se convierte automáticamente en una ilustración; se desconoce su naturaleza.

## Selección manual autorizada

Archivo TOML propio, opcional; no editar los XML originales:

```toml
[games."ID-EXACTO-DE-LA-BIBLIOTECA"]
path = "/ruta/absoluta/arte-autorizado.png"
root = "/ruta/absoluta/arte-autorizado"
```

Configurar `EMOTIONAL_ARTWORK_OVERRIDES=/ruta/overrides.toml`. Máximo 4 MiB, cargado en el worker al iniciar (ampliado desde 256 KiB para bibliotecas de varios sistemas). El recurso elegido tiene prioridad, conserva proporciones y debe superar las mismas validaciones. El ID no se deduce del título. La configuración expresa la autorización para leer esa raíz; no certifica derechos de redistribución.

Preparación opcional de carátulas verticales de SteamGridDB: [STEAMGRIDDB_ARTWORK.md](STEAMGRIDDB_ARTWORK.md). Usa este mismo mecanismo; no añade red al frontend ni cambia la home, los XML o las carátulas originales.

## Límites y seguridad

PNG/JPEG por extensión y firma real; archivo regular, raíz canónica, comprobación de pertenencia antes y después de abrir, e identidad de descriptor/ruta en Unix. Máximo 8 MiB de entrada, 2048×2048, presupuesto del decodificador de 24 MiB, destino máximo 640×480 y 16 candidatos por petición. Los fallos se convierten en fallback o en el siguiente candidato. No hay subprocess de conversión.

No hay watcher del sistema de archivos: los cambios se comprueban al solicitar de nuevo el recurso, cambiar de tarjeta/vista o reiniciar. No se promete una transacción de filesystem frente a modificaciones hostiles concurrentes del árbol de fuentes. La caché no puede escribirse dentro de las raíces fuente de una petición.

## Estado

Implementados selección, preparación, fallback e integración en las dos vistas y baldosas. El logo aislado todavía no se compone visualmente: se usa texto. Las ilustraciones de pruebas son gráficos procedurales originales provisionales, no arte editorial definitivo. Las rutas y formatos verificados en escritorio/Docker no certifican una biblioteca física diferente.

## Revisión de estabilidad · 28-10-2026

Durante la prueba RG353V del binario `474cdc2bf5a7`, el propietario confirmó apertura/regreso del juego y aplicación de una carátula SteamGridDB. También observó desaparición/reaparición rápida de carátulas en Biblioteca, incluso con movimiento reducido.

Se corrigió un vaciado prematuro en la presentación: cambiar la generación del catálogo conserva la carátula del mismo juego mientras el worker prepara su reemplazo. Cambiar de juego sí descarta la imagen anterior; un reemplazo inválido retorna al fallback. No cambia geometría, materiales, fuentes, lanzadores o datos originales.

`tools/verify_artwork_stability.py` usa un archivo de texto propio y una imagen sintética, retrasa 650 ms la carga y comprueba dos invalidaciones con ocho capturas intermedias. La variante de control con el vaciado anterior falla en la primera actualización; la revisión mantiene iguales los píxeles de la carátula. Pasan los 27 tests de artwork y el flujo nativo de selección/aplicación/restauración/cancelación de SteamGridDB con proveedor simulado. Esto verifica el fallo de refresco reproducido en software; no certifica aún que explique todo el parpadeo observado en el LCD.

Candidato ARM64 privado `da2f29084dbc3a0ca07e3ac8406576de6468360af9bce588204538aceac4dadf`, GLIBC máxima 2.35, sin RPATH. Preparado como entrada independiente «EmotionalUI - Caratulas estables prueba», conservando la anterior. Comprobación física posterior: el propietario abrió esta entrada y confirmó «Las carátulas ya permanecen estables» tras la indicación de dejar la pantalla quieta y navegar entre juegos. Queda validada la corrección observada del parpadeo en esta revisión; no acredita FPS medidos, estabilidad prolongada ni una nueva prueba de apertura/SteamGridDB del candidato corregido.

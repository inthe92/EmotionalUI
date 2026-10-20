# EmotionalUI · Modo prueba

Implementado en el frontend: `--trial --db BASE_PROPIA --library-root RAÍZ_EXISTENTE`. Conserva Continuar/EmotionalUI, navegación, privacidad y escena. No modifica EmulationStation, servicios ni arranque. No inicia emuladores en esta primera vista previa: combinar --trial con un perfil de lanzamiento se rechaza antes de abrirlo. La acción Jugar explica ese límite, sin registrar una sesión falsa.

Sin raíz explícita, se comprueba `/roms`, documentada en el upstream de dArkOS, **solo si existe localmente**. No se asegura que todas las revisiones utilicen esa ubicación. Si falta, la interfaz abre vacía y explica la situación. No requiere configurar sistemas que ya tienen gamelist.xml, pero detección de un segundo almacenamiento u otras ubicaciones no está implementada. No se inventan entradas ficticias.

Después de construir la ventana se inicia un worker independiente: busca gamelist.xml en la raíz y un nivel de sistemas (máximo 512 entradas), usa el importador existente, revalida archivos/canonicalización y actualiza SQLite propia. Publica un catálogo tras cada sistema. Los fallos de un XML permiten continuar con los demás. El estado/foco de biblioteca se conserva por ID; opciones de un juego desaparecido se cierran. El ranking de actualización se aplica en memoria; no se consulta SQLite durante el foco.

Primer inicio: se puede usar la biblioteca ya cacheada; no se espera a parsear todos los XML antes del event loop. Reapertura: firma de contenido versionada evita parsear/escribir XML idénticos. Sigue leyéndose su contenido para la firma (no criptográfica ni autenticación), y se comprueba disponibilidad de archivos. Un cambio reimporta el sistema afectado. No se eliminan registros históricos porque una entrada desaparezca del XML; solo se muestran archivos existentes dentro de la raíz.

Carátulas PNG/JPEG locales: se reutiliza la referencia `image` dentro de la raíz autorizada, sin copiarlas a otra colección. Worker separado, límite 8 MiB/2048×2048, salida máxima 104×142 RGBA, una petición en vuelo, sin decodificación durante el callback. Rechazo/falta de imagen deja el diseño de texto utilizable. No se aceptan imágenes externas a la raíz, red ni SVG externos. No se conserva una caché ilimitada de carátulas.

La importación XML ocurre fuera de Slint, pero la sustitución del modelo/ranking todavía es síncrona en memoria y ocurre una vez por actualización, no por movimiento de foco. Las medidas en Docker no prueban el RK3566. Para bibliotecas enormes repartidas en muchos XML, las instantáneas completas repetidas pueden requerir optimización posterior.

En consola: **entrada segura desde el menú existente y recuperación gráfica física pendientes**. No se añade un script Ports ni se detiene EmulationStation a partir de una suposición. `platform/distribution/trial.sh` solo inicia el proceso después de reconocimiento explícito de pantalla liberada y recuperación independiente; esas opciones no realizan ni demuestran la cesión. No ejecutar sobre un propietario DRM activo. Salir por Start → Salir; el proceso devuelve el código al invocador. Un retorno al invocador en Docker no demuestra un retorno visible a EmulationStation.

Prueba física futura: seguir GAME_LAUNCHER.md, con diagnóstico actual, ROM legal aislada y destinos de guardado comprobados. El Modo prueba de vista previa no es esa prueba de lanzamiento. No utilizar el watchdog histórico fijo con juegos en curso.

## Corrección de rendimiento comprobable

La refrescada original preservaba decisiones con una búsqueda completa por cada juego (coste cuadrático). Se sustituye por un índice HashMap por ID, conservando elecciones y metadatos nuevos; regresión con orden cambiado y juego nuevo. En Modo prueba, las rutas de carátulas se cargan en el worker: se elimina la segunda lectura completa de metadatos de imagen antes del event loop. La primera lectura del catálogo cacheado sigue precediendo a la ventana; no se declara un arranque totalmente asíncrono.

Continuar también conserva el foco por ID cuando una importación inserta títulos antes del juego activo. La regresión gráfica reproduce el fallo anterior y verifica la corrección; los índices de los controles se recalculan sin cambiar la elección del usuario.

# Ciclo diario · Hitos 2.1–2.2

La ruta activa es `assets/scenes/daycycle/assets.slint`, generada por `tools/daycycle_assets.py` desde **una tabla**: `assets/scenes/daycycle/palettes.json`. Conserva los siete IDs y un slot neutro. Cada entrada enlaza una escena local RGB 640 × 480 y colores/materiales propios. `AtmosphereDesign` sigue siendo el único sistema geométrico y tipográfico; sus colores y superficies ahora enlazan esos tokens compartidos. Continuar, Descubre, dock, cápsula y paneles reciben la misma paleta.

El usuario eligió la reconstrucción de la referencia de ventana, lago y lámpara ámbar. `selected-room-v1/anochecer.png` es la maestra; otras seis ediciones modifican luz/cielo y encendido de lámpara. Son **estudios provisionales**, no capas con geometría matemáticamente idéntica. Mantienen composición y objetos reconocibles, con pequeñas diferencias pictóricas pendientes de revisión. Imágenes derivadas de referencia privada: excluidas de Git por derechos de redistribución pendientes. No constituyen un paquete público compilable sin esos archivos. Originales, propuestas japonesas descartadas y master independiente anterior se conservan. Procedencia, prompts y hashes junto a las variantes.

Paleta: madrugada azul grisáceo; mañana temprana salvia; media mañana oliva claro; primera tarde arena; media tarde dorado; anochecer rosa humo; noche lavanda gris. Cambian texto principal/secundario, superficies, bordes, foco, selección del dock, cápsula y paneles; nunca cambia la rejilla ni la tipografía. Los PNG pequeños de materiales se generan **offline con el mismo generador de bordes de 2 px** y antialiasing; no hay blur ni relighting en ejecución. Ocho juegos pequeños de texturas evitan shaders o tintar contornos incorrectamente. Los atributos de superficie incluyen opacidad; no son siete temas independientes.

El cambio de pintura por franja es directo; no se afirma una transición multicapa o interpolación del cielo. Se conservan el desplazamiento entre secciones y las transiciones ambientales simples existentes. Movimiento reducido conserva su contrato. Lámpara y sombras se hornean en cada recurso: no se superpone el antiguo rectángulo luminoso en una posición equivocada. La máscara climática es un rectángulo inscrito en el cristal principal (352,96,160,216), todavía provisional. No hay arte de clima/estaciones final ni nuevas consultas de red.

`core/src/environment.rs` contiene TimeContext, DayMoment, SceneContext y SceneResolver; reutiliza Season y Conditions. Franjas ajustables únicamente en `DayMoment::START_MINUTES`: 00:00, 06:00, 09:00, 12:00, 16:00, 19:00 y 21:00. Identificadores: madrugada, manana_temprana, media_manana, primera_tarde, media_tarde, anochecer y noche. Las seis Phase anteriores siguen siendo un contrato de compatibilidad del audio y las pruebas históricas; ya no deciden el arte ni la paleta visual integrada.

El reloj usa fecha/hora civil local del sistema, con plausibilidad 2020–2100 **sin afirmar sincronización NTP**. Orden: sistema plausible → muestra ya sincronizada suministrada por un adaptador → último dato guardado → desconocido. No se consulta red. La muestra sincronizada tiene API y tests, pero no un proveedor de red conectado. El dato guardado en SQLite propia es histórico, no avanza como si fuera actual: se identifica LastKnown, no fecha sesiones y usa escena neutra. `--unknown-time` fuerza desconocido; `--at` es simulación y no contamina el checkpoint horario real. El writer existente guarda muestras reales al iniciar y en actualizaciones de minuto; no añade trabajo de disco al foco.

Estaciones meteorológicas del hemisferio norte: primavera marzo–mayo, verano junio–agosto, otoño septiembre–noviembre e invierno diciembre–febrero. Clima por defecto desconocido; `--weather` es una entrada manual/simulada offline (clear/cloud/rain/fog, además de las opciones anteriores). No hay API meteorológica ni inferencias de sol/lluvia. Se mantienen las comprobaciones existentes de observaciones caducadas. Sin hora fiable no se infieren estación, clima ni festividad.

Pendiente: revisión humana de las siete variantes, derechos para distribución, producción de capas finales, suavizado entre pinturas, selección estacional/climática, proveedor opcional verificado, amanecer astronómico y validación física de esta revisión. La composición de las pantallas no cambia.

---

## Diseño anterior conservado (histórico, seis franjas)

> Dirección actualizada por el usuario: la vista integrada utiliza `assets/scenes/lofi-room-v2/manifest.toml`, una adaptación de Foto 1.jpg. La escena ocupa 640 × 480 en ambas vistas; la máscara climática está en x334,y104,178×208. El master y la cámara son únicos. `room-v1` queda conservado como recurso anterior, sin sustituirlo ni borrarlo. Las reglas horarias y la separación núcleo/renderizador descritas abajo se mantienen. Las referencias anteriores a viewport 287 corresponden a la propuesta descartada y no al diseño vigente.

# SceneEngine

`core/src/scene.rs` selecciona un estado puro; no conoce Slint, rutas, red ni dispositivos. `app/ui/integrated.slint::RoomScene` lo representa. El manifiesto `assets/scenes/room-v1/manifest.toml` inventaría recursos y límites. La escena maestra es una ilustración raster generada para este proyecto y reducida a 640 × 480; procedencia y prompt íntegro junto a ella.

| Momento | Hora local | Atmósfera |
|---|---|---|
| Mañana | 06–10:59 | luz crema |
| Mediodía | 11–13:59 | claridad neutra |
| Media tarde | 14–16:59 | calidez contenida |
| Anochecer | 17–19:59 | exterior violeta humo, lámpara tenue |
| Noche | 20–23:59 | azul tinta, luz interior |
| Madrugada | 00–05:59 | exterior más oscuro, menos luz interior |

Ventana, escritorio, silla, lámpara y cámara se conservan. Un único PNG compartido evita seis decodificaciones de fondos completos. Estimación de almacenamiento de píxeles RGBA del master: 1.17 MiB; no equivale a RSS de la aplicación. No hay motor 3D, blur ni fotogramas ambientales permanentes.

Las capas de tinte exterior/interior, nubosidad y luz interpolan 220 ms; lluvia/niebla/nieve aparecen mediante opacidad en una máscara fija. La lluvia y la nieve actuales son estáticas, no partículas animadas. Estación cambia un tinte discreto sin animación prolongada. Halloween, Navidad y Año Nuevo reutilizan IDs del calendario configurable y añaden detalles pequeños provisionales.

Al arrancar se selecciona directamente el momento actual. El temporizador contextual de la aplicación espera a no tener panel ni controles mantenidos antes de aplicar cambios. Sin fecha conocida o con ambiente neutro selecciona mediodía seguro, clima desconocido y sin fiesta/estación inferida. Se ignora clima caducado mediante el servicio existente. No se consulta Internet.

## Caché y selección

El master está embebido y lo comparte un componente persistente. Solo existen las capas de composición; no se precargan cientos de variantes. Slint conserva la imagen decodificada; no se introduce otra caché de bitmaps. Los renders se solicitan por cambios y animaciones finitas; el simulador informa fotogramas rasterizados, nunca FPS presentados. Movimiento reducido o transición ambiental desactivada aplica las nuevas capas sin interpolación.

## Estado artístico y limitaciones

La habitación es un candidato real integrado, no la vieja habitación vectorial ni arte definitivo aprobado. La noche utiliza tintes/lámpara; no reconstruye físicamente sombras y luz exterior independiente. La variación estacional es limitada y la nieve no cubre el paisaje. Amanecer/atardecer astronómicos no están conectados: se usan franjas fijas y no se inventan coordenadas geográficas. Próxima producción: máscaras artísticas ajustadas, luces nocturnas y vegetación/ornamentos compartidos, conservando el master y su cámara.

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

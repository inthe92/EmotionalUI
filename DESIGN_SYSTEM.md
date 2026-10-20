# Revisión definitiva · 19-10-2026

Sistema vigente en `AtmosphereDesign`, `AmbientPanel`, `AmbientAction`, `SectionHeader`, `SectionDock` y `SystemControls`. Se conservan las notas históricas inferiores.

- Curvas circulares: L16, M12, S8, XS6. Las cápsulas usan altura/2: dock 344×52 R26 en x148/y416; controles 176×28 R14 en x440/y16; CTA único 188×36 R18. La altura del CTA se aumenta de la propuesta de 32 a 36 para texto de 15 px y distintivo A de 20 px con margen vertical.
- Rejilla de 4 px; margen exterior 24, padding 16 y separación principal 16. Encabezado x24/y16, sección 44 px negrita, texto de juego 22/19, cuerpo 13 y auxiliares 11. Ambos layouts utilizan los mismos componentes, no copias.
- Continuar: protagonista x24/y104, 544×208; hasta dos partidas en y344, 264×56. Descubre: x24/y104, 592×240, arte 320×232 y columna editorial; alternativa en y352, 288×48. Contenido termina antes de y400; dock comienza en y416.
- Imágenes interiores derivadas: protagonista de Descubre R12 = L16 − separación4; portada de Continuar R0 = L16 − padding16; miniatura de baldosa R4 = M12 − separación8. Material interior de panel R14 = L16 − borde2. Foco interior de 2 px; selección activa del dock es relleno/subrayado, foco es contorno.
- Material cálido prerenderizado, sin blur dinámico. `tools/room_materials.py` produce todas las variantes desde los mismos colores. PNG con esquinas transparentes y nine-slice en paneles evita defectos observados del gradiente redondeado/SVG del renderizador software de Slint 1.13. Botón, dock y cápsula tienen recursos interiores de su tamaño exacto para conservar la curva. Nunca se usa una textura nine-slice con centro de tamaño cero.
- Panel superior: indicadores explícitamente sin verificar; no hay interruptores falsos. B recupera el foco, los controles no atraviesan el panel. Biblioteca y Ajustes reutilizan su lógica y listas existentes. Se retiran los accesos duplicados de biblioteca/cambio de vista de las tarjetas.
- Movimiento finito 220 ms, reversible desde la posición actual. Baldosas de Descubre empiezan tras ~81 ms de la curva prevista; el dato es un objetivo del algoritmo, no una medición del LCD. Movimiento reducido elimina desplazamientos y retardo.

Capturas y comprobaciones: `docs/visual-unification-2026-10-19`. Los gráficos de juego y sesiones de la revisión son fixtures propios, no arte final ni evidencia física.

---

# Revisión vigente de navegación y arte · 19-10-2026

La habitación lofi se conserva. Esta revisión sustituye las coordenadas de home de las fases históricas documentadas más abajo.

| Elemento | Continuar | Descubre |
|---|---|---|
| Encabezado | Continuar, 40 px, x24/y20 | Descubre, misma escala |
| Protagonista | Tarjeta estable 480×202, x24/y110 | Arte 290×232 a la izquierda; lectura 288×232 a la derecha |
| Secundarios | Hasta dos partidas 234×56 | Una alternativa discreta y biblioteca, 44 px de alto |
| Acción | CTA 188×40 | Misma familia, dimensiones y foco |
| Navegación base | Dock 204×58, x234/y412 | Mismo dock, otro estado activo |

La diferencia se entiende sin fondo ni movimiento: Continuar agrupa partidas; Descubre separa arte y lectura editorial. Título y dock indican la sección. El foco de navegación es distinto de la indicación de sección activa. La cruceta llega al dock y vuelve a la columna apropiada; A confirma, B vuelve y L/R alternan.

EditorialArtwork usa la misma familia de fondos, radios y foco para hero, carátula compuesta, captura y texto. El fondo derivado se desenfoca durante la preparación, nunca en el render. Véanse EDITORIAL_ARTWORK.md, ARTWORK_CACHE.md y ARTWORK_RENDERING.md. No se añaden porcentajes de progreso ficticios ni arte comercial de las referencias.

---

# EmotionalUI — identidad de la referencia original

Decisión definitiva del usuario, 18-10-2026: habitación lofi al atardecer, ventana hacia paisaje, luz cálida interior y paneles oscuros translúcidos. No se continúa la exploración editorial beige. Se conserva su implementación anterior en `docs/visual-correction/before/` y sus pruebas históricas.

## Composición nativa, 640 × 480

La habitación ocupa ambas pantallas completas y conserva la cámara. No se separan ilustración y controles mediante una franja de papel. La zona derecha mantiene ventana, lago y lámpara visibles. Una lectura oscura discreta protege cabecera y textos sin animación continua.

| Elemento | Geometría / regla |
|---|---|
| Margen principal | 24–32 px |
| Cabecera | marca 15 px, fecha 12 px, reloj 25 px |
| Saludo | serif 30 px en dos líneas, inspirado en la referencia |
| Panel protagonista | x24, y207, 392 × 166 px |
| Carátula de prueba | 104 × 142 px, recorte contenido |
| Título del juego | serif 19 px, máximo dos líneas, elipsis |
| Descripción | 12 px, máximo dos líneas |
| Acción principal | 154 × 31 px; distintivo A, texto 15 px |
| Secundarios | biblioteca y descubrimiento/alternativa según vista |
| Radios | panel 12 px, botón 8 px |

Resume conserva únicamente el último juego, Continuar, microestadísticas opcionales y destinos biblioteca/EmotionalUI. Emotional presenta recomendación y una alternativa; no incluye un catálogo infinito en home. Los títulos y portadas de las capturas son ficticios. Sin carátula se recupera espacio para texto.

## Color y material

`AtmosphereDesign` en `app/ui/integrated.slint` centraliza fondo `#181e1c`, texto `#f1e5d2`, secundario `#c9bfaf`, foco/acción `#ddbc95` y cristal `#232724e6`. Cristal sin desenfoque en tiempo real: superficie oscura translúcida, borde de luz fino y borde de foco inmediato. El botón enfocado utiliza crema cálida con texto oscuro.

El test de contraste comprueba texto y foco sobre el cristal compuesto con fondo blanco y negro, y las máscaras de lectura. Mínimo objetivo 4.5:1 para texto y acción; no equivale a certificación WCAG completa ni validación del LCD.

## Interacción y coherencia

Cruceta: abajo desde protagonista llega a biblioteca; derecha pasa al secundario contiguo; arriba vuelve al protagonista disponible. No hay salto circular. Resume añade un destino EmotionalUI con foco propio. L/R alterna vistas conservando contexto. A confirma, B vuelve, Start abre opciones.

El foco se reconoce por borde de 2 px y botón contrastado; no depende de animaciones. La confirmación comprime discretamente el contenido durante 80 ms. Se conservan renovación de contenido, paneles de 180 ms y transiciones ambientales finitas de 220 ms; movimiento reducido elimina sus desplazamientos/interpolaciones. El cambio lógico de vista es inmediato.

## SceneEngine conservado

Los seis momentos, clima, estación y calendario mantienen las reglas puras existentes. Un solo master de `assets/scenes/lofi-room-v2` sustituye el recurso beige en la vista integrada. No se cambian geometría, ventana o muebles entre estados. La iluminación por capas es todavía experimental: una imagen base al atardecer no produce por sí sola sombras diurnas físicamente correctas. Ornamentos y portadas siguen provisionales. No se han generado seis habitaciones independientes.

No se muestran batería, Wi-Fi, temperaturas ni estadísticas inventadas. Se omiten datos sin fuente fiable. La última sesión se muestra con fecha disponible; los datos de demostración se rotulan. El catálogo importado sigue sin usar portadas ficticias como si fueran carátulas reales.

## Pulido sensorial · fase 8

Se preserva la habitación lofi elegida; no se crean variantes de identidad ni se recupera el diseño beige. Audio usa la misma tinta crema, borde ámbar y panel oscuro translúcido de AtmosphereDesign. Ajustes accesibles por Start → Sonido; siete filas, foco evidente, izquierda/derecha para regular, A confirma/cambia y B vuelve conservando la fila del menú general.

La habitación puede recorrer seis píxeles entre vistas, en un solo plano y durante 240 ms. La interfaz permanece anclada para facilitar lectura. Movimiento reducido, ambiente neutro o movimiento ambiental desactivado anulan el desplazamiento. No hay animación en reposo. Véase PARALLAX_DESIGN.md: todavía faltan capas para parallax real.

La música es opcional y deja silencios amplios. Sonidos de interfaz inicialmente desactivados; ninguna espera musical retrasa una acción. Partitura y recursos son provisionales, explícitamente separados del diseño visual definitivo. SOUND_DESIGN.md documenta el lenguaje y sus límites.

## Ampliación funcional · Continuar de fase 10

Se conserva habitación, tipografía y material. Con más de una partida, el protagonista pasa de 166 a 126 px de alto, en la misma posición x24/y207; portada de 102 px y CTA a y85 dentro del panel. Dos entradas compactas de 190×32 px en y344, tipografía 13 px y títulos largos elididos. Estadísticas y accesos inferiores mantienen posición. Con una partida se conserva la composición anterior. No hay nueva identidad ni carrusel.

Opciones del juego reutiliza hoja oscura y foco ámbar del panel de Sonido, sin blur nuevo. La explicación se solicita y puede ocultarse; no expone puntuaciones técnicas. Los tests de contraste y exportación RGB previos se mantienen, además de renders reales de tres partidas/opciones. La validación de legibilidad en LCD sigue pendiente.

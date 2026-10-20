# Movimiento entre vistas · fase 8

## Recurso disponible y límite artístico

La habitación actual es una imagen plana única `assets/scenes/lofi-room-v2/master.png`. No existen máscaras fiables de paisaje lejano, ventana, muebles y primer plano. No se han inventado capas desplazando recortes que dejen huecos: **esta implementación es un panorama de un plano, no parallax multicapa real**. Se preservan la identidad lofi, geometría, perspectiva y luces anteriores.

## Implementación

`core/product.rs` selecciona el destino y `home.slint` anima `pan-position` entre −3 px (Continuar) y +3 px (EmotionalUI) durante 240 ms con ease-out. `integrated.slint` aplica el mismo desplazamiento al fondo y a sus adornos ambientales, usando una imagen sobredimensionada a 648 × 486 y viewport recortado de 640 × 480. El margen cubre los extremos sin bordes vacíos. Paneles, texto, foco y máscara de lectura permanecen estables.

No hay movimiento permanente, respuesta a inclinación, blur ni timers por frame propios. Slint vuelve a interpolar desde la posición actual si se interrumpe el cambio. El estado de la vista y sus acciones cambian inmediatamente: no se espera al fin de la animación. La restauración de foco por vista se conserva.

Al arrancar se coloca directamente en la posición correcta. La habilitación de movimiento después de la sonda de arranque no recorre estados anteriores. Movimiento reducido, movimiento ambiental desactivado o modo visual neutro sitúan el panorama en cero sin transición decorativa; los botones siguen respondiendo. El ajuste de movimiento reducido ya existente se conserva.

## Verificación y métricas

La suite `platform/simulator/phase8.py` despacha pulsaciones reales de teclado a Slint, muestrea la propiedad animada y exige valores intermedios y llegada al destino. Prueba pulsaciones durante transiciones, cambios L/R repetidos, retorno de biblioteca, conservación de foco, catálogo vacío y movimiento reducido. `pan-timing.json` incluye tiempo programado y tiempo observado en cada muestra.

Esto mide interpolación en el bucle de software, **no presentación en LCD ni latencia pulsación-fotón**. Los 240 ms son la duración declarada; el tiempo observado se acota con muestras de 20 ms y el retraso del bucle. Las capturas finales son renders reales RGB 640 × 480.

Para multicapa futuro hacen falta imágenes y máscaras con encuadre idéntico, extensión fuera de pantalla y oclusiones resueltas por el artista. Solo entonces tendría sentido distribuir 2–3/5–7/9–12 px según profundidad; no es parte de esta entrega.

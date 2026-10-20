# Renderizado editorial a 640 × 480

`EditorialArtwork` en `app/ui/integrated.slint` recibe una imagen **ya preparada**, disponibilidad, título, categoría y estado de demostración. No abre ni decodifica rutas. El panel padre mantiene el foco ámbar; el componente ofrece imagen preparada o tipografía sin icono roto.

Se conserva la habitación lofi y las dos composiciones diferenciadas:

- **Continuar:** protagonista horizontal estable, carátula contenida de 122×174, dos partidas compactas, estadísticas fiables existentes y dock. No se inventan capítulos ni porcentajes.
- **Descubre:** hero de 282×224 a la izquierda y lectura editorial a la derecha, un solo CTA, alternativa discreta, biblioteca y dock. El panel derecho utiliza degradado oscuro translúcido fijo. Texto y título no se superponen a una imagen sin protección.
- CTA compartido de 188×40, misma tinta, foco y confirmación.
- Títulos largos: dos líneas con elipsis. Los títulos de baldosas se eliden conservando las dimensiones.

No hay blur dinámico ni shader. La copia al `SharedPixelBuffer` solo ocurre al terminar una composición vigente. El modo tipográfico no exige crear un bitmap ni realizar un trabajo gráfico.

## Transición y foco

Encabezados grandes y dock activo explican el cambio incluso sin animación. Las composiciones tienen jerarquías distintas. Un desplazamiento de UI corto y entrada secundaria escalonada acompañan al cambio. El fondo sigue siendo un panorama plano; no se presenta como parallax multicapa.

La transición finita usa 220 ms como objetivo y una generación para cancelar una transición anterior, conservando la posición actual. Solo programa el siguiente tick mientras hay movimiento. No bloquea eventos, y el control secundario enfocado aparece inmediatamente. Movimiento reducido cambia directamente; desactivar movimiento ambiental detiene inmediatamente el panorama, conservando respuesta funcional de la sección.

Las capturas dentro de un callback pueden retrasar timers. La prueba de progreso sin capturas se separa de la secuencia visual y exige valores intermedios y ambos extremos. No se deducen FPS o duración de presentación del tiempo nominal ni de PNG capturados.

## Reproducción

```sh
python3 tools/artwork_fixtures.py /ruta/nueva/fixtures  # Pillow, solo producción de pruebas
python3 platform/simulator/artwork_visual.py /ruta/emotional-ui-simulator /ruta/fixtures /ruta/nueva/capturas
python3 platform/simulator/navigation_ux.py /ruta/emotional-ui-simulator /ruta/nueva/navegacion
```

El primer script crea gráficos procedurales propios y archivos de texto `.fixture`, nunca ROMs. Los otros dos ejecutan el programa Slint real. Comprueban RGB opaco 640×480, CRC/descompresión, recursos ausentes/corruptos, persistencia de caché, tres juegos importados y navegación durante 1200 ms de retraso artificial del worker. Este último solo existe con feature `simulator`; no reproduce una microSD real.

Los recursos sin licencia de los artes de referencia no se copian. La referencia guía proporciones y jerarquía, no convierte una captura comercial/conceptual en evidencia de software.

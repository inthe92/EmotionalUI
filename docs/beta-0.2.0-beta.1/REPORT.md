# Beta 0.2.0-beta.1 · 27-10-2026

Recuperación de la build Slint/GPU de fondo abstracto azul/dorado elegida por el
propietario, con lanzador funcional y la última exportación Atlas terminada.
No se reconstruyen la home, tarjetas, cápsula o navegación. La rama beta deriva
de la última publicación y conserva los checkouts de trabajo anteriores.

## Integración

- Se reutilizan el supervisor Rust, los perfiles explícitos de emuladores instalados,
  la importación XML/archivos permitidos y el aislamiento de progreso existente.
- El lanzador de Ports carga Atlas experimental y permite jugar desde Biblioteca
  aunque falte un perfil. Continuar utiliza argumentos de reanudación diferenciados.
- El SQLite exacto `0.4.0-ai-experimental.5-snes-research.2` contiene 1.841 identidades
  provisionales y 274 perfiles propuestos. La proyección preparada conserva los
  valores, desconocidos y atribución. Solo coincide por título de metadatos/sistema
  inequívocos; no por nombres ROM ni aproximación. No se promueve al canal estable.
- Un enlace explícito SQLite marca su identidad para que el fallback no mezcle
  atributos de otra versión. No se modifican puntuaciones de investigación.
- El foco claro tiene degradado dorado/melocotón/rosa. Ambos modos giran el ángulo
  del degradado en GPU, una vuelta en ocho segundos, sin regenerar imágenes de borde.
  Movimiento reducido detiene el temporizador; software conserva borde estático.
  El temporizador se detiene al destruir el contexto antes de abrir un juego.

## Pruebas observadas

| Comprobación | Resultado |
|---|---|
| Núcleo/servicios Rust | 196 pruebas aprobadas |
| Aplicación Rust, incluida integración Atlas | 116 pruebas aprobadas |
| Perfiles Python | 18 pruebas aprobadas |
| Entrada de jugador Python | 3 pruebas aprobadas |
| Entrada/recuperación funcional Python | 26 pruebas aprobadas |
| Recorrido real de la app en simulador | Abrir proceso propio, volver, continuar, regresar; 2 lanzamientos y guardado original intacto |
| Apariencia | Persiste entre reinicios y cambios de hora |
| Shader/borde GPU en Mac | Claro/oscuro ejecutados, capturas reales OpenGL 4.1 a 640×480 |
| Giro | Píxeles de contorno distintos en dos fotogramas reales; fondo reconstruido una vez por contexto |
| Movimiento reducido | Sin secuencia continua de fotogramas de giro |
| ARM64 | Compilación release terminada, con avisos heredados de código sin usar/backend/linker |

[Claro](light.png) · [Claro tras girar](light-rotated.png) · [Oscuro](dark.png) ·
[Verificación de fotogramas](gradient-verification.json).
[Auditoría del ELF y del paquete final](artifact-verification.json).
Las capturas son poses de los componentes reales con datos de muestra, no fotos
del LCD ni mediciones de FPS. El camino de juegos se prueba con procesos propios,
no con emuladores comerciales o ROMs.

La suite Python histórica ampliada ejecutada en el árbol recuperado tuvo siete
fallos y 21 errores: fixtures de investigación ausentes y contratos antiguos de
materiales, contraste ambiental y procedencia. No se han ocultado ni convertido
en aprobaciones. Esa suite no valida la composición GPU nueva; sigue pendiente
su reconciliación y la validación completa de los caminos software heredados.
Los tests de investigación que no forman parte del runtime no se incorporan por
arrastre a esta publicación. Esta entrega se publica como **prerelease**.

## Instalación y estado físico

El paquete incluye ELF AArch64, catálogo, lanzador, instalador de archivos propios,
avisos de fuentes, dependencias y Atlas. Verifica hashes, arquitectura, modelo,
bibliotecas y configuración antes de ceder pantalla. No incluye drivers ni toca
arranque, servicios permanentes, EmulationStation, ROMs o BIOS. La versión anterior
se conserva y los datos personales permanecen fuera del paquete y Git.

La instalación y la ejecución de esta beta en RG353V **están pendientes**. La
consola respondió por red pero la autenticación SSH no pudo completarse al no
tener el propietario acceso al Mac. La espera se cerró sin escrituras al dispositivo.
La prueba física anterior de otra build no se atribuye a este ELF. No hay ensayo
gráfico programado, prueba de juego/guardado físico ni afirmación de 60 FPS.

No se recomienda sustituir el frontend principal. La próxima aceptación debe
comprobar apertura/cierre de un juego real, Continuar, audio, controles, retorno
visible y coste de la animación en hardware. [Instrucciones y límites](../../BETA.md).

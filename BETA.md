# EmotionalUI 0.2.0-beta.2

Revisión del paquete: incluye el auxiliar `menu.py` que faltaba en beta.1 y verifica
las importaciones desde el propio payload antes de crear el ZIP. La apariencia,
Atlas y la lógica de juegos se conservan. Usar esta revisión en lugar de beta.1.

Beta funcional para RG353V / dArkOS ARM64. Conserva la pantalla de inicio y el
aspecto abstracto azul y dorado elegido por el propietario. No es el antiguo banco
de materiales ni una pantalla reconstruida. Claro/oscuro son elecciones persistentes;
el foco claro usa un degradado dorado/melocotón/rosa suave y el oscuro conserva
su gradiente anterior. Los colores giran suavemente en el foco mediante GPU;
Movimiento reducido detiene el giro. El camino software mantiene un borde estático.

## Instalar sin sustituir EmulationStation

1. Descargar `EmotionalUI-Beta-0.2.0-beta.2-RG353V.zip` y comprobar `SHA256SUMS`.
2. Extraer la carpeta `EmotionalUI-Beta-0.2.0-beta.2` y `EmotionalUI - Beta.sh`
   en el mismo directorio **Ports** de la tarjeta. Conservar ambos juntos.
3. Abrir **EmotionalUI - Beta** desde EmulationStation. El lanzador comprueba
   archivos, dispositivo y bibliotecas antes de ceder la pantalla.
4. Salir desde **Ajustes → Volver a la consola**.

La aplicación se copia a `~/.local/opt/emotional-ui-beta-<hash>`; nunca sobrescribe
la versión anterior. Los datos y el progreso reutilizan
`~/.local/share/emotional-ui-player`, separado de los guardados originales.
Retirar la entrada de Ports deshabilita su acceso; no borrar datos de progreso para
desinstalar el programa. `manage.py` conserva el instalador/restaurador de archivos
propios con comprobación de modificaciones. La preview antigua sigue disponible.

No cambia el arranque, unidades permanentes, controladores, EmulationStation,
XML originales, ROMs o BIOS. Los emuladores existentes se abren con perfiles
explícitos de RetroArch; no se ejecutan comandos de shell del XML ni se descargan
cores. No todos los emuladores independientes o ajustes por juego están soportados.

## Funciones conectadas

- Biblioteca real desde `gamelist.xml` y archivos de formatos permitidos; abrir un
  juego no exige una ficha emocional. Ports no se trata como un sistema de ROMs.
- Lanzamiento y retorno mediante el supervisor existente. La interfaz libera el
  contexto gráfico antes de abrir el juego y se reconstruye al terminar.
- Continuar utiliza argumentos de reanudación propios. Estados y RAM originales
  reconocidos se copian sin sobrescribirlos; cada ruta tiene progreso independiente.
  La disponibilidad de estados depende del core, no se garantiza por tiempo jugado.
- Atlas `0.4.0-ai-experimental.5-snes-research.2`: 1.841 identidades provisionales,
  274 perfiles con algún atributo. No representa revisión completa ni aprobación
  humana. El índice preparado usa título de metadatos y plataforma inequívocos;
  no adivina por nombres de archivo. Los no enlazados siguen en Biblioteca.
- Ajustes, controles del sistema existentes y herramientas originales de Opciones
  se conservan. Las herramientas conservan su interfaz original.

Atlas funciona sin IA ni red en la consola. El reloj sirve a recomendaciones;
no activa cambios automáticos de pintura o clima. [Licencias y alcance de Atlas](assets/atlas/BETA-NOTICE.md).

## Verificación y límites

Compilación ARM64 con `linuxkms-gpu-material`, ELF64 AArch64, GLIBC máxima requerida
2.35, sin RPATH ni drivers incluidos. `SHA256SUMS` y el manifiesto del paquete
identifican la entrega exacta; el informe de pruebas identifica el ejecutable.
Las bibliotecas Mali/EGL/GLES proceden del sistema ya instalado.

196 pruebas de núcleo/servicios y 116 de aplicación aprobadas. Las pruebas Python
focalizadas de perfiles, entrada de jugador y recuperación funcional también pasan.
El recorrido de la aplicación simulada abre un proceso propio, vuelve, continúa y
regresa, preservando el guardado de prueba y la apariencia entre reinicios. Esos
procesos no son emuladores y no acreditan savestates reales.

La suite histórica Python ampliada aún tiene contratos visuales del antiguo
material/ambiente y fixtures de investigación no incluidos en esta entrega; sus
fallos quedan registrados y no se presentan como aprobados. La beta no es una
versión estable ni una certificación de todos los caminos software heredados.

La animación solicitada modifica el ángulo del degradado, sin regenerar texturas
ni imágenes de borde en cada fotograma. El temporizador pertenece al contexto GPU,
se detiene al destruirlo y no sigue ejecutándose durante una partida. El fondo y
su blur mantienen la caché; la actividad gráfica en reposo cambia respecto de la
build sin animación. Su coste en la consola aún no está medido.

**Beta.2 instalada en RG353V / dArkOS, sin iniciar gráficos.** Se verificaron sus
20 archivos, la carga de bibliotecas y la planificación de 114 perfiles compatibles
(16 omitidos). La entrada propia de Ports quedó registrada; las 11 entradas
anteriores y el proceso de EmulationStation se conservaron. [Informe de instalación](docs/beta-0.2.0-beta.2/REPORT.md).

La interfaz y los juegos de este ELF todavía requieren validación física supervisada.
La build visual anterior sí tuvo una prueba de GPU/mando/retorno; no se traslada
esa aceptación a este ELF. Faltan juego y Continuar reales, sonido, tacto/mandos,
latencia, estabilidad prolongada y comparación equivalente con Slint
software/FemtoVG. No se afirman 60 FPS sostenidos; no hay ensayo gráfico programado.

## Compilar

Rust y dependencias fijadas en `Cargo.lock`; Python 3.11+ para herramientas.

```sh
cargo test --locked -p emotional-core -p emotional-services
cargo test --locked -p emotional-ui --no-default-features --features simulator
cargo build --locked -p emotional-ui --features gpu-material
# SDK aislado y Zig 0.14.1 ya preparados; no instala dependencias en la consola:
EMOTIONAL_KMS_FEATURE=linuxkms-gpu-material tools/arm64/build.sh
python3 tools/build_player_package.py --binary target/aarch64-unknown-linux-gnu/release/emotional-ui --output dist/beta-nueva
```

El wrapper ARM64 exige `EMOTIONAL_ZIG` y `EMOTIONAL_ARM64_SYSROOT` explícitos.
La proyección Atlas se reproduce con `tools/prepare_beta_atlas.py`, el SQLite
incluido y `assets/atlas/classic-legacy-v1.toml`; no inicia investigación ni rellena
desconocidos. La fuente del catálogo mantiene atribución por juego dentro del SQLite.

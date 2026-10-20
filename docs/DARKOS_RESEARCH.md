> Nota histórica de fase 1. Para el estado y procedimiento actuales, consultar [DARKOS_INTEGRATION.md](../DARKOS_INTEGRATION.md).

# Investigación técnica inicial · 2026-10-18

## Decisión provisional: Slint software

Se fija Slint **1.13.1** para un prototipo reproducible. Su manifiesto oficial ofrece Winit, renderizador software y LinuxKMS. El escritorio usa únicamente Winit + software: no depende de que exista OpenGL en la consola. La elección de una versión anterior a la documentación actual evita adoptar capacidades nuevas sin comprobarlas; no significa que sea la versión más reciente.

Fuentes consultadas:

- [Manifiesto Slint 1.13.1](https://github.com/slint-ui/slint/blob/v1.13.1/api/rs/slint/Cargo.toml).
- [LinuxKMS 1.13.1: dependencias](https://github.com/slint-ui/slint/blob/v1.13.1/internal/backends/linuxkms/Cargo.toml).
- [Documentación LinuxKMS actual](https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backend_linuxkms/).

La versión fijada necesita libinput, xkbcommon y sus dependencias Linux; `backend-linuxkms` incluye libseat, además de fontconfig para fuentes. No aplicar directamente las instrucciones de Slint 1.18 para desactivar libinput/libseat a esta versión. DRM y acceso al asiento deben comprobarse en la imagen instalada. El backend actual documenta DRM dumb buffers y una alternativa LinuxFB; esta última no se considera verificada para 1.13.1. El renderizado software evita exigir GLES, pero no garantiza 60 FPS ni un consumo de memoria concreto.

## Qué confirma dArkOS y qué falta

El [build oficial RG353V](https://github.com/christianhaitian/dArkOS/blob/master/build_rg353v.sh) utiliza el camino RK3566. El [build de EmulationStation](https://github.com/christianhaitian/dArkOS/blob/master/build_emulationstation-rk3566.sh) instala su configuración, scripts y servicio. Es evidencia del diseño upstream, no un inventario del dispositivo del usuario.

La [configuración RK3566 de 64 bits](https://github.com/christianhaitian/dArkOS/blob/master/Emulationstation/es_systems.cfg.rk3566-64bit_Only) contiene raíces `/roms`, emuladores y órdenes con sustituciones. Por tanto, no basta con ejecutar una ROM: hay que respetar selección de núcleo, configuración y operaciones asociadas. No se copia ni ejecuta ese XML en este prototipo.

No se ha inspeccionado la consola en esta entrega. Permanecen desconocidos su versión concreta de dArkOS, glibc, DRM/KMS, permisos, controlador, bibliotecas, disponibilidad de aceleración y eventos táctiles. `platform/darkos/inspect.sh` produce un inventario local de solo lectura. Para confirmar táctil harán falta además capacidades de eventos y una prueba de coordenadas; el nombre de un dispositivo no basta. Los gamepads necesitan un adaptador explícito: libinput no garantiza navegación por cruceta como si fuese teclado.

## Camino ARM64 preparado, sin certificar

El núcleo y los servicios pueden comprobarse para ARM64 Linux sin renderizador:

```sh
rustup target add aarch64-unknown-linux-gnu
cargo check --locked -p emotional-core -p emotional-services --target aarch64-unknown-linux-gnu
```

Para la aplicación hacen falta compilador cruzado, sysroot y bibliotecas **de la imagen destino**, incluyendo SQLite bundled (compilación C). Comando previsto, no una compilación Linux verificada:

```sh
cargo build --locked --release -p emotional-ui --target aarch64-unknown-linux-gnu --no-default-features --features linuxkms
# En una sesión controlada del dispositivo, una vez verificados permisos:
SLINT_BACKEND=linuxkms-software ./emotional-ui --db :memory:
```

No ejecutar simultáneamente con otro propietario de DRM. No instalar como servicio, cambiar el arranque ni usar root para ocultar problemas de permisos. Debe existir un procedimiento probado para recuperar EmulationStation antes de la primera prueba física.

## SDL3 como alternativa

No se cambia de tecnología ahora: Slint permite verificar primero un camino software y conserva la lógica independiente. Si el backend no consigue acceso gráfico/entrada fiable o no cumple presupuestos tras medir en la consola, crear una prueba Rust + SDL3 con el mismo catálogo y las mismas reglas. Comparar renderizado, navegación, memoria y retorno desde emulador en condiciones iguales. Ni la presencia de SDL2 en dArkOS ni un antecedente de otro proyecto prueban SDL3 o Slint.

## Clima opcional

Se revisaron [condiciones de Open-Meteo](https://open-meteo.com/en/terms) y su [licencia/atribución](https://open-meteo.com/en/licence). La API gratuita tiene condiciones de uso y límites propios, y los datos requieren atribución según su licencia. Antes de distribuir una integración se debe comprobar el tipo de uso, plan y condiciones vigentes. El prototipo no consulta la API, no configura localidad y no almacena ubicación. Solo implementa simulación y validación de edad de caché (hasta tres horas, descartando fechas futuras).

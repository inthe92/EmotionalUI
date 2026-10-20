# Compilación completa Linux ARM64 · fase 3

Estado del 2026-10-18: **ELF completo enlazado y ejecutado por CLI en la RG353V**. La interfaz gráfica LinuxKMS no se ha iniciado todavía.

## Artefacto y herramientas

- Target: `aarch64-unknown-linux-gnu`, Rust 1.99.1, Slint 1.13.1, SQLite bundled.
- Host: macOS ARM64. Compilador C, archivador y linker: Zig 0.14.1 oficial para aarch64-macos.
- SDK de referencia: Debian 12 bookworm ARM64, 40 paquetes fijados por versión y SHA-256 en `tools/arm64/sysroot.lock.json`. No es un sysroot extraído de la consola.
- Bibliotecas de la consola identificada: Debian 13 trixie, glibc 2.41, libinput 1.28.1, libudev 257.13, xkbcommon 1.7.0. Falta libseat.
- Binario: `target/aarch64-unknown-linux-gnu/release/emotional-ui`, 7.102.336 bytes.
- SHA-256: `edbc66a3da9e6cb8ea12546fdbd331ec15fef7fdab6468d6d5e8740575b8b6a7`.
- Intérprete: `/lib/ld-linux-aarch64.so.1`; sin RPATH/RUNPATH. No enlaza directamente EGL, GLES, GBM, X11, Wayland ni Qt.
- NEEDED: libseat.so.1, libxkbcommon.so.0, libudev.so.1, libinput.so.10, libm.so.6 y libc.so.6. La copia privada de libseat utiliza bibliotecas existentes de la consola; no se copia glibc.

## Reproducir desde un directorio nuevo

```sh
rustup target add aarch64-unknown-linux-gnu
export EMOTIONAL_ZIG=$(python3 tools/arm64/prepare_zig.py "$HOME/.cache/emotional-ui/toolchain-new")
export EMOTIONAL_ARM64_SYSROOT="$HOME/.cache/emotional-ui/sdk-new"
python3 tools/arm64/prepare_sysroot.py "$EMOTIONAL_ARM64_SYSROOT"
./tools/arm64/build.sh
python3 tools/arm64/verify_elf.py target/aarch64-unknown-linux-gnu/release/emotional-ui --sysroot "$EMOTIONAL_ARM64_SYSROOT" --output /tmp/emotional-elf-new.json
python3 tools/arm64/stage_bundle.py dist/rg353v-new --sysroot "$EMOTIONAL_ARM64_SYSROOT"
```

Se necesita Python >=3.9, `ar`, pkg-config y LLVM/GNU readelf. En este Mac se utilizó `/opt/homebrew/opt/llvm/bin/llvm-readelf`. `prepare_zig.py` fija URL y checksum oficiales en `zig.lock.json`; rechaza sobrescribir una instalación ajena. `prepare_sysroot.py` verifica cada .deb y extrae exclusivamente los datos: no ejecuta scripts de paquetes ni instala bibliotecas en el sistema. Los hashes se obtuvieron del índice oficial mediante HTTPS; no se implementó verificación GPG del índice. Un paquete retirado del mirror exige conservar el .deb verificado o resolver explícitamente otro lock, nunca actualizarlo en silencio.

El build aísla pkg-config del host y utiliza un compilador C cruzado para SQLite. `EMOTIONAL_GLIBC_FLOOR=2.36` configura Zig; el binario resultante requiere como máximo GLIBC_2.34 directamente. El inventario de versiones de las bibliotecas del SDK no equivale a sus requisitos mínimos. El preflight en el dispositivo real y la ejecución de `--help` sí resolvieron los símbolos de este artefacto; otra imagen dArkOS exige repetirlos.

## Ajustes y problemas reales

El wrapper elimina el `--target` GNU que cc-rs añade, conservando el target aceptado por Zig. También elimina `--fix-cortex-a53-843419`, rechazado por Zig: este port se limita al RK3566 **Cortex-A55**, confirmado por el árbol del dispositivo. No utilizar ese wrapper para hardware Cortex-A53. La compilación final emite un aviso de Zig por la optimización de linker `-O1` declarada por rustc; el enlace termina correctamente y el aviso se conserva en el log.

Durante la preparación, Cargo quedó bloqueado leyendo un `Cargo.lock` descargado bajo demanda por iCloud. Se recuperó el contenido original: el hash del lockfile sigue idéntico al de fase 2. No se regeneró ni se cambiaron dependencias Rust para sortearlo.

## Despliegue aislado

Paquete local: `dist/emotional-ui-0.2.0-rg353v-phase3/`. Contiene binario, libseat privada, avisos de licencia, scripts y manifiesto SHA-256. Rechaza generar sobre una carpeta existente.

En la consola se creó exclusivamente `/home/ark/emotional-ui-tests/phase3-20261018-ssh/`. `run-preflight.sh` establece `LD_LIBRARY_PATH` solo para ese proceso. No modifica `/etc`, el cargador, los servicios ni las bibliotecas instaladas.

Para otra prueba, elegir una carpeta nueva. Transferencia sin metadatos de macOS:

```sh
ssh ark@DIRECCION 'mkdir /home/ark/emotional-ui-tests/otra-prueba'
COPYFILE_DISABLE=1 tar --no-xattrs --no-acls -C dist/rg353v-new -czf - . | ssh ark@DIRECCION 'tar -xzf - -C /home/ark/emotional-ui-tests/otra-prueba'
ssh ark@DIRECCION '/home/ark/emotional-ui-tests/otra-prueba/run-preflight.sh'
```

No guardar contraseñas en comandos, scripts o repositorio. El preflight llama al loader con `--list`; no abre DRM ni inicia Slint. `--help` retorna antes de crear ventana. Evidencias: `docs/phase3/arm64-build.log`, `arm64-elf.json`, `rg353v-preflight.json`, `rg353v-cli-probe.json`.

## Fase 8: artefacto nuevo, sin despliegue

Se reutilizaron el SDK y Zig anteriores para compilar el frontend actual con `EMOTIONAL_KMS_FEATURE=linuxkms-direct tools/arm64/build.sh`. El resultado se copió a `dist/phase8-cross/emotional-ui`: 12.553.696 bytes, ELF64 AArch64, loader GNU y sin RPATH. Requisitos directos hasta GLIBC_2.34; no hay dependencia enlazada obligatoria de ALSA porque se carga opcionalmente en ejecución. Inventario y hash en docs/phase8/arm64-elf.json y log de build arm64-release-build.log. El aviso conocido de Zig por -O1 sigue registrado. Se mantiene previous-emotional-ui como copia del ejecutable que había antes de compilar.

No se transfirió ni ejecutó este binario en la RG353V. Los resultados físicos y hashes de fase 3 que figuran arriba son históricos y no validan esta revisión. El backend sonoro necesita comprobar libasound.so.2 y PCM default en el dispositivo, además de los requisitos gráficos existentes. Una segunda variante release de simulator se utiliza solo para comparar software en Docker, no como backend de consola.

# Estado actual · fase 5 local

Esta entrega solo compila y prueba en macOS/Docker. No se ha conectado por SSH ni se han cambiado archivos o servicios de la Anbernic. El build actual no tiene navegación física, táctil ni rendimiento validados. Véanse [PHASE5_REPORT.md](PHASE5_REPORT.md), [SIMULATOR.md](SIMULATOR.md) e [INPUT_MAPPING.md](INPUT_MAPPING.md).

Slint 1.13.1 ofrece `backend-linuxkms-noseat`; el proyecto conserva la feature `linuxkms-direct` como alternativa al backend con libseat. Existen registros históricos de una primera imagen y restauración humana de otro hash en docs/phase5/human-confirmation.json, y de captura parcial de botones. No demuestran compatibilidad del artefacto Docker actual.

Los ejecutables `dist/simulator/emotional-ui-kms` y `emotional-ui-simulator` son builds de desarrollo Linux ARM64. El segundo usa una plataforma software sin pantalla y no debe confundirse con una prueba DRM. Antes de transferir el primero hay que generar el paquete release, verificar ABI/bibliotecas, manifiesto y acceso al dispositivo. No hay instrucciones de despliegue automático en el simulador.

Para la siguiente sesión física: usuario delante de la consola, dos conexiones SSH independientes, recuperación de EmulationStation comprobada y watchdog externo armado antes de ceder la pantalla. No hacerlo mientras el usuario esté ausente. R2/stick permanecen sin calibración física completa. No cambiar arranque, kernel, particiones, servicios persistentes, RetroArch, ROMs ni partidas.

## Procedimiento histórico de fase 3

El texto siguiente se conserva para referencia técnica. Sus afirmaciones de estado corresponden a la fase 3, no a la entrega Docker actual. No ejecutarlo automáticamente.

# Integración con dArkOS · fase 3 experimental

**ELF ARM64 enlazado, bibliotecas resueltas y CLI ejecutada en la RG353V. La interfaz LinuxKMS sigue sin probarse en el LCD.** No se modificó el arranque ni se detuvo EmulationStation.

Consultar [compilación](ARM64_BUILD.md), [diagnóstico real](HARDWARE_DIAGNOSTIC.md) e [informe](PHASE3_REPORT.md).

## Camino gráfico conservado

Slint 1.13.1, feature `linuxkms`, renderer software (`SLINT_BACKEND=linuxkms-software`), escala 1. Se conserva el diseño 640 × 480, los cuatro estados, Glass sin blur dinámico y movimiento reducido. Núcleo y servicios siguen independientes del renderizador.

El código exacto 1.13.1 usa buffers DRM dumb y contempla fallback LinuxFB; `SLINT_BACKEND_LINUXFB` fuerza ese segundo camino. Esto es evidencia del backend, no una prueba en esta consola. No activar el fallback a ciegas mientras otro frontend controla el display. El formato de pantalla, gestión de buffers, tearing y liberación/restauración del CRTC deben verificarse físicamente.

Fuentes fijadas: [dependencias LinuxKMS](https://github.com/slint-ui/slint/blob/v1.13.1/internal/backends/linuxkms/Cargo.toml), [software/display](https://github.com/slint-ui/slint/blob/v1.13.1/internal/backends/linuxkms/display/swdisplay.rs), [DRM](https://github.com/slint-ui/slint/blob/v1.13.1/internal/backends/linuxkms/drmoutput.rs), [entrada/event loop](https://github.com/slint-ui/slint/blob/v1.13.1/internal/backends/linuxkms/calloop_backend.rs).

`SLINT_DRM_OUTPUT=DSI-1` y `SLINT_DRM_MODE` son opciones del backend; comprobar su formato y detección antes de fijarlas. `SLINT_KMS_ROTATION` admite 0/90/180/270. No configurar rotación sin observar imagen y coordenadas. No se presupone aceleración EGL/GLES/GBM y el ELF software no las requiere directamente. No hay evidencia que justifique sustituir Slint por SDL3 ahora.

La consola dispone de libinput 1.28.1, suficiente para las APIs 1.21 activadas por la dependencia Rust. Falta libseat: el paquete experimental incluye una copia privada 0.7.0-6 de Debian, con licencia y hash, sin instalarla en el sistema. Las pruebas del loader y CLI pasan con esa copia. El acceso al asiento desde SSH todavía puede ser rechazado por logind; no instalar seatd, usar root ni cambiar permisos para ocultarlo.

## Prueba manual: requisitos

1. Consola sin juego en ejecución y conexión SSH estable. Abrir una segunda sesión SSH por una conexión independiente y comprobar que responde.
2. Tener la consola a la vista para verificar imagen y recuperación. Confirmar que `sudo -n systemctl start emulationstation.service` es el procedimiento de recuperación apropiado para esta instalación. Se comprobó el permiso, pero aún no la recuperación física tras Slint.
3. Inspeccionar el servicio/wrapper antes de usarlo. La instalación observada tiene User=ark, Restart=on-failure y ejecuta `/usr/bin/emulationstation/emulationstation.sh`. Matar solo el proceso puede provocar reinicio. No ejecutar ese wrapper desde EmotionalUI.
4. Mantener una orden de recuperación preparada en la segunda sesión. No modificar servicios persistentes ni dejar una prueba sin recuperación independiente.

## Comandos manuales, no ejecutados por el agente

En la segunda sesión se puede armar un arranque diferido del servicio original antes de ceder la pantalla:

```sh
# Solo en la prueba supervisada; guardar el log en una carpeta propia nueva.
nohup sh -c 'sleep 45; sudo -n systemctl start emulationstation.service' > "$HOME/emotional-ui-tests/recovery-new.log" 2>&1 < /dev/null &
```

En la primera sesión, una vez verificados los requisitos:

```sh
cd /home/ark/emotional-ui-tests/phase3-20261018-ssh
./run-preflight.sh
sudo -n systemctl stop emulationstation.service
./run-smoke.sh "$HOME/emotional-ui-tests/smoke-new" --recovery-confirmed
sudo -n systemctl start emulationstation.service
systemctl is-active emulationstation.service
```

Elegir un nombre de log nuevo; no sobrescribir uno previo. La cesión gráfica requiere supervisión humana: **estos comandos no se han ejecutado**. Si la interfaz original no vuelve visualmente, usar la segunda sesión y el procedimiento de recuperación de la instalación. Un servicio activo no demuestra que el LCD se haya restaurado. No realizar la prueba si no se puede garantizar recuperación.

El harness rechaza EmulationStation/Xorg/Xwayland/Weston activos y nunca los detiene. Verifica SHA-256 y ELF, ejecuta preflight y abre la demo con SQLite en memoria. La aplicación solicita salida tras 2 segundos; el supervisor espera como máximo 12 segundos, envía TERM solo al grupo creado y, si sigue vivo, KILL tras 3 segundos. Ignora SIGHUP para mantener ese límite si cae el SSH iniciador; una interrupción del supervisor limpia su proceso hijo. No restaura automáticamente el CRTC ni inicia servicios.

`started.json` proporciona PID y ruta `quit.request`. Desde la segunda sesión, crear ese archivo pide cierre al event loop cada 100 ms. Si el event loop se bloquea, se mantiene el timeout del supervisor. La salida no depende de botones ni de la imagen. Registros: `events.toml`, stdout/stderr, preflight, `resources.jsonl` y `result.json`. Los eventos de ventana/event loop y resolución detectada no certifican presentación en el LCD.

## Entrada, táctil y audio

El gamepad real es retrogame_joypad (event4 en este arranque); sus sticks tienen rango -1800..1800. Los eventN pueden cambiar. `input-map.example.toml` continúa deshabilitado y sin asignar A/B/X/Y: falta registrar códigos con una herramienta supervisada y conectar el adaptador evdev a las acciones existentes. Libinput no transforma automáticamente un gamepad en teclado.

El kernel expone Hynitron cst3xx Touchscreen (event1), X 0..639/Y 0..479 y multitáctil. Eso no prueba tacto funcional en Slint; validar orientación, esquinas, foco y coexistencia con botones después de demostrar imagen. No hay funciones esenciales exclusivas del táctil. No se abre dispositivo de audio en esta fase.

## Medición y emuladores

`sample_process.py PID --seconds 10` recoge RSS/HWM, threads y CPU de /proc. No mide FPS presentados ni latencia pulsación-imagen. Para esos valores hace falta instrumentar presentación y medir con vídeo/temporización de hardware. Registrar renderer, build/hash, temperatura, alimentación, escena y movimiento reducido. 60 FPS, <100 ms y <150 MB permanecen como objetivos.

El importador gamelist.xml de fase 2 conserva su almacenamiento propio y lectura no destructiva. No se conecta todavía a la GUI ni se inicia RetroArch u otro emulador. El lanzamiento/retorno se abordará después de validar display y entrada, sin modificar ROMs, BIOS, partidas ni configuración crítica.

## Preparación de lanzamiento · fase 9

Nuevo ELF release KMS directo en dist/phase9-cross/emotional-ui, auditado frente al SDK de referencia; CLI ejecutada en Docker ARM64, inicio gráfico rechazado al faltar DRM/framebuffer. No se instaló ni ejecutó en RG353V. El supervisor permite configurar un único emulador existente con argv explícito y proceso foreground; no copia plantillas de shell XML ni modifica governor/RetroArch. Continuar ya usa esa infraestructura, verificada con fixtures controlados.

Antes de usar un perfil físico comprobar ejecutable/core, bibliotecas del dispositivo y contrato de espera. Diseñar recuperación dependiente del PID del juego: **no combinar con el viejo watchdog de tiempo fijo que podría devolver EmulationStation mientras el emulador sigue abierto**. No se ha validado recuperación física de esta fase. Véanse GAME_LAUNCHER.md y LAUNCH_LIFECYCLE.md.

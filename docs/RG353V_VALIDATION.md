# RG353V · validación ambiental privada

20-10-2026. **Prueba física actual: NO PROBADA.** No se alteró ninguna sesión gráfica.

| Estado | Evidencia / alcance |
|---|---|
| VERIFICADO | Origen `80c6dcd4a4243a6ccaaf2dd7486516420de8bf5d` + árbol local ambiental conservado. Un worktree, rama main; sin remotos. Los respaldos y originales no se han modificado. No se encontró AGENTS.md aplicable. |
| VERIFICADO | ELF release ARM64, KMS directo, Slint software, escala 1 y ventana lógica 640×480. SHA-256 `9e2b88a686ac0e56dc54baefd3941af77e57d4110109890e172a98e6d54018bb`, 21.100.744 bytes. Build existente recompilado mediante wrapper, sin cambios funcionales ni dependencias nuevas. |
| VERIFICADO | 216 pruebas Rust del workspace y 40 Python focalizadas superadas. Incluyen las regresiones previas de recuperación, empaquetado, perfiles, geometría/materiales y clima. Formato y diff correctos. No se repitieron benchmarks. |
| VERIFICADO · Docker ARM64 | Mismo ELF, loader y `--help` correctos; 36 configuraciones aceptadas con `--history --trial --input-profile`, red desactivada, sistema de archivos de contenedor de solo lectura y DB temporal propia. No se inicializó una pantalla ni se abrió un mando. |
| VERIFICADO | Paquete privado, 50 archivos de carga útil + manifiesto/checksums. Extracción aislada, hashes, estructura y permiso ejecutable comprobados. Launcher en dry-run por defecto; no instalador ni modificación de arranque. |
| EXPERIMENTAL | Renderer físico linuxkms-software y perfil histórico `retrogame_joypad`. El perfil procede de una captura anterior: cruceta, A/B/X/Y, Start/Select, L/R/L2; R2 y sticks sin inventar. Su funcionamiento actual debe comprobarse en la consola. |
| BLOQUEADO | SSH a la dirección previamente facilitada agotó el tiempo de conexión. Sin supervisión actual confirmada ni segunda conexión verificable: no despliegue ni parada de EmulationStation. No se probaron otras direcciones ni credenciales. |
| NO PROBADO | LCD actual, 28 contextos visuales en hardware, bordes/aliasing/escala, controles físicos, brillo bajo/alto, RAM/CPU reales, carga visible, fluidez, sesión prolongada, liberación de recursos y retorno visible a ES. No hay FPS ni latencias físicas medidos. |

## Paquete y compatibilidad

`~/Desktop/EmotionalUI-RG353V-20261020-9e2b88a6-r2.tar.gz` (~10,95 MB), con checksum exterior. Carpeta extraída del mismo nombre. Es **privado**: el ELF embebe pinturas provisionales de redistribución pendiente, conservadas fuera de Git. Fuentes, materiales y audio embebidos; no requiere copiar otro juego de imágenes. Audio desactivado en el launcher. Sin ROMs, BIOS, carátulas comerciales ni partidas en el paquete.

Se reutiliza el backend **linuxkms-direct**, no `simulator`: este último rechaza el perfil físico. Los 28 casos usan `--at` con fechas de cuatro estaciones meteorológicas NORTE y siete horas clave; no cambian el reloj. Hay seis casos adicionales para clima/eventos, más live y reloj no fiable. Clima explícito = DEMO; live usa clima desconocido y no supone ubicación/hemisferio. No hay proveedor meteorológico real.

Dependencias directas: `libxkbcommon.so.0`, `libudev.so.1`, `libinput.so.10`, `libm.so.6`, `libc.so.6`; loader `/lib/ld-linux-aarch64.so.1`. SDK/floor de build GLIBC 2.36; no se certifica un ArkOS más antiguo. La evidencia física histórica correspondía a dArkOS, no necesariamente a la instalación actual. El preflight usa el loader real y cancela incompatibilidades; no instala bibliotecas ni sustituye libc.

## Procedimiento preparado

El README incluido contiene copia aislada sin sobrescribir versiones, verificación, preflight sin GUI, dos conexiones SSH independientes, prueba `true` de recuperación y ejecución temporal **solo tras presencia y autorización actuales**. Requiere modelo RG353V, conector DRM 640×480, unidad ES y nombre de mando comprobables. Biblioteca real únicamente mediante ruta verificada, protegida de escritura; sin ruta se usa biblioteca vacía. No se fabrican sesiones para llenar Continuar.

La unidad transitoria conserva ExecStopPost para ES y límite 2–180 s exclusivamente en `--trial` sin juegos. Cada ejecución tiene logs, archivo de cierre y resultado independientes: no reutiliza un cierre anterior. Salida normal Start → Salir; cierre alternativo por el archivo indicado y recuperación desde la segunda conexión. Confirmar ES **en el LCD**, además del estado del servicio. No se cambia brillo, sonido, conexiones ni servicios permanentes. Prueba prolongada pendiente; no retirar el límite para aparentarla.

No se implementó un fundido nuevo: la mezcla ambiental heredada entre claves ya existía. Los perfiles de matriz arrancan en horas clave; la decisión sobre fundidos queda pendiente. No cambian UI, pinturas, fuentes, botones, navegación, SQLite, audio ni launcher de juegos.

## Capturas, incidencias y trazabilidad

Comparativas reales macOS de las cuatro combinaciones, ambas vistas: `~/Desktop/EmotionalUI-Hito2.5/`. Son simulación, no fotografías del LCD. No se regeneraron sin cambios visuales. Logs de esta tarea: `~/Library/Application Support/EmotionalUI-Validation/rg353v-validation-20261020-t8xo9ods/`.

Incidencias de herramientas resueltas: la imagen Docker tenía un entrypoint de suite y se repitió con `/bin/sh`; una comprobación de restauración usó una ruta local con espacios que el helper rechaza para systemd, por lo que se verificaron directamente todos los hashes del contenido extraído. No son fallos de la GUI ni evidencia física. Se detectó que el diario exclusivo de humo también necesitaba un nombre distinto por ejecución; corregido en el helper y protegido por una regresión. No se cambió el frontend.

El commit local de esta entrega solo versiona infraestructura de validación, pruebas e informe mediante allowlist de cinco rutas. **No incorpora el conjunto de cambios ambientales anteriores ni constituye un checkout autocontenido de la GUI actual.** El manifiesto identifica base `80c6dcd` y árbol modificado. No se publican recursos, paquete ni logs; no push.

Siguiente puerta: disponibilidad/SSH actual y presencia confirmadas, diagnóstico solo lectura del firmware/loader/mando/DRM y recuperación; después autorización expresa para una primera prueba gráfica breve del hash indicado. Aprobación visual y rendimiento físico pendientes.

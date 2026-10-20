# Adaptador de lanzamiento · fase 9

## Datos y autorización

Un identificador de la GUI se resuelve mediante imported_games de SQLite propia. La ruta se vuelve a canonicalizar y debe ser un archivo dentro de una raíz autorizada. Las etiquetas command, emulator, title o rutas de imágenes del XML no generan instrucciones ejecutables.

Los perfiles TOML son configuración explícita del usuario, no se importan del XML. services/launcher/configured.rs valida campos, raíces, un marcador `{rom}` como argumento completo, working_directory y entorno. Selecciona la raíz autorizada más específica; dos raíces equivalentes son un error, no una elección arbitraria. No se interpretan `%ROM%`, `%EMULATOR%` ni comandos compuestos de EmulationStation.

Plan construye Command/argv sin concatenación de shell. Se rechazan ejecutables relativos, archivos ausentes/no ejecutables y texto sin cabecera reconocible; un script debe tener shebang explícito. Este último sigue siendo código autorizado que hay que inspeccionar: la aplicación no hace seguro un script que internamente utilice eval sobre sus argumentos. La prueba de un archivo de texto ejecutable detectó un fallback de plataforma y motivó esa validación adicional.

LD_LIBRARY_PATH privado del frontend se retira del entorno del juego, evitando mezclar bibliotecas del paquete con las del emulador. Un perfil autorizado puede declarar explícitamente un entorno necesario; no se obtienen variables ni directorios del XML.

## dArkOS: investigación y límites

Se revisó la configuración [RK3566 64 bit upstream](https://github.com/christianhaitian/dArkOS/blob/master/Emulationstation/es_systems.cfg.rk3566-64bit_Only) con commit y hash registrados en docs/phase9/research/darkos-source.json. Declara raíces por sistema, ejecutables seleccionables bajo /usr/local/bin, core libretro bajo la configuración del usuario y hooks de governor. El diagnóstico histórico del proyecto incluye envolturas del sistema, pero **no se inspeccionó el dispositivo durante esta fase**. No se confunde el upstream con la instalación actual ni con ArkOS.

No se ejecutan sudo/perfmax/perfnorm, no se copian sus scripts y no se recrean configuraciones globales. El primer adaptador usa un ejecutable/core existente indicado expresamente. No reproduce todavía overrides de emulador por juego, governor, todos los hooks ni selección automática de core de EmulationStation. No hace funcionar EmulationStation como interfaz oculta.

## Preparar un perfil sin inventar rutas

En el entorno de destino, indicar rutas **ya comprobadas**:

```
python3 tools/launcher_profile.py --root RUTA_ROMS_EXISTENTE \
  --executable EJECUTABLE_RETROARCH_EXISTENTE --core CORE_EXISTENTE \
  --working-directory DIRECTORIO_EXISTENTE --output PERFIL_NUEVO.toml
```

El generador resuelve todas las rutas, comprueba su existencia y se niega a sobrescribir. Solo escribe un perfil propio, no ejecuta RetroArch ni modifica sus configuraciones. Para un script de otro emulador se puede redactar un perfil equivalente tras inspeccionar su contrato; arguments debe contener `{rom}` aislado.

Campos obligatorios: `handoff="process-exit"`, `[[launchers]]`, id/root/executable/arguments/working_directory y `waits_for_exit=true`. El contrato exige un proceso **en primer plano que termina cuando termina el juego**. Scripts que daemonizan o dejan descendientes jugando no están soportados: declarar true no sustituye su verificación. No se garantiza recuperación gráfica con un perfil incorrecto.

Inicio local controlado: `emotional-ui --launcher-config PERFIL --db BASE_ABSOLUTA --library-root RAIZ`. Sin perfil la interfaz sigue disponible y explica que falta configurar un lanzador. Con perfil, Continuar, recomendación principal y selección de biblioteca usan el adaptador.

Para LinuxKMS se requiere además `--launch-physical-supervised`. Es un bloqueo de uso accidental; no constituye una prueba de recuperación. Solo utilizarlo durante una sesión supervisada con recuperación independiente comprobada. No se ha realizado esa prueba en esta fase.

## Preparación física de fase 10 · sin ejecutarla

Se conserva el lanzador de fase 9. Solo se amplía el rango de foco para las dos partidas secundarias; el test controlado abre la segunda y comprueba ID/argv/retorno. No se configura un perfil físico a partir del listado de Nintendo o de los paths de Docker.

Procedimiento siguiente, manual y supervisado:

1. Confirmar identidad/IP, dArkOS y acceso SSH independiente; ejecutar inspect.sh existente y guardar el diagnóstico. Contrastar versión de RetroArch, su `--help`, core y contrato foreground con los archivos realmente instalados. El registro upstream de fase 9 es investigación histórica, no un inventario actual.
2. Elegir contenido de prueba legal y aislado de ROMs/guardados personales. Revisar destinos de SRAM/savestates, auto-save/auto-load y posibles overrides. **Cancelar si no se puede garantizar que la prueba no escribirá guardados originales**. No modificar ajustes globales para conseguirlo.
3. Generar un perfil propio con tools/launcher_profile.py indicando únicamente rutas verificadas. Inspeccionar argv y los scripts del ejecutable; deben esperar al juego completo y no daemonizar. La [guía oficial RetroArch CLI](https://docs.libretro.com/guides/cli-intro/) explica el contrato básico `-L core contenido`; el --help instalado debe confirmar las opciones de esa versión. No se inventan flags para aislar guardados.
4. Con recuperación SSH comprobada y el usuario delante, preparar una cesión temporal de pantalla con estado/PID del juego. No usar el antiguo watchdog de tiempo fijo ni parar servicios persistentes. La autorización histórica de una sesión supervisada no demuestra supervisión presente.
5. Registrar PID de supervisor/frontend/juego, tickets de audio, journal, versión/hash de binario y perfil, resolución y observación física. Confirmar A desde cada entrada de Continuar; observar cierre del frontend antes del emulador. Salir del juego por su mecanismo validado.
6. Comprobar terminación del proceso, regreso de Slint, foco, mandos y audio. Solo después restaurar el menú alternativo mediante el mecanismo temporal previamente verificado. Si la reapertura falla, usar SSH y el PID real; no matar procesos por nombre ni restituir gráficos sobre un emulador aún activo.

Condiciones de cancelación: SSH inestable, ausencia de supervisión, destinos de guardado inciertos, ABI/dependencias no verificadas, proceso que daemoniza, falta de recuperación o error de apertura DRM/ALSA/evdev. Guardar stdout/stderr y no repetir a ciegas. Esta fase no se conecta a la consola y no prueba RetroArch físicamente.

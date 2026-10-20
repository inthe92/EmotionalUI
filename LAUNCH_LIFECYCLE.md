# Ciclo de vida · fase 9

## Por qué existe un supervisor

Ocultar una ventana no demuestra que Slint haya liberado DRM master, libinput o todos los descriptores. Se utiliza la salida completa del proceso como frontera de recursos. El supervisor es la misma aplicación en modo coordinador: no inicializa Slint, SceneEngine, evdev ni AmbientAudioEngine. Existe un único proceso frontend hasta que el usuario decide abrir un juego.

1. Continuar toma el identificador del último juego disponible. Biblioteca y recomendación principal utilizan la misma entrada. Se guarda la preferencia de audio efectiva y se inicia el ticket de cesión. No se incrementa historial por enfocar o seleccionar una carátula.
2. Mientras se prepara, A repetida se ignora y B puede cancelar. Un timer comprueba el ticket sin esperar en el callback; timeout de dos segundos cancela y recupera audio. Los ajustes de audio se drenan a SQLite al cerrar.
3. Después de confirmar audio cerrado, el frontend oculta la ventana y sale. Escribe una petición privada de datos; el supervisor espera la salida correcta del proceso antes de resolver de nuevo el juego y abrirlo. La finalización del proceso es la cesión de gráficos y mandos, no hide().
4. El supervisor valida ID, archivo y perfil, crea el proceso con argv y espera su terminación. Durante ese periodo no existe frontend que capture botones, dibuje escenas ni reproduzca música. Un proceso extra ligero es el coste deliberado de esta frontera de recursos.
5. Registra el resultado y abre una instancia nueva del frontend, que lee la base SQLite existente. No reimporta gamelist.xml ni escanea ROMs. Recupera vista, foco, índice de biblioteca y últimas preferencias de audio. Retira overrides CLI de modo/volumen para que no vuelvan a activar un ajuste que se desactivó en el menú. El banco de audio explícito se conserva.

Las opciones originales sin launcher-config siguen el camino previo. Los replays de desarrollo se aplican solo a la primera instancia; sim-return-replay permite verificar la segunda en simulator. No están disponibles como controlador físico. Las sondas de smoke de la primera instancia se conservan y no se sobrescriben al retornar.

## Registro fiable

launch_log separa intentos, perfil, resultado, código de salida, tiempo monotónico del proceso y registro de sesión. spawn-error, early-exit y exit-error no actualizan la última partida. Un proceso correcto de al menos dos segundos permite recordar el juego por orden de lanzamiento, sin depender del reloj. Esto demuestra un proceso observado, no que una persona haya jugado ni que exista un savestate.

Solo se escribe una sesión fechada si `clock_confirmed=true`, la fecha del sistema es plausible y el proceso terminó correctamente tras ese mínimo. Por defecto no se inventa esa confianza: la fecha puede omitirse y Continuar sigue funcionando mediante el último proceso válido. No se usa la fecha simulada del Mood Engine para un registro real. El reloj confirmado debe proceder de una comprobación fiable posterior, no de afirmar que NTP está funcionando.

No se escribe progress/completed. No se afirma reanudación exacta: el adaptador abre normalmente la ROM; autoload/savestates dependen del emulador y todavía no se integran. Los fallos quedan auditados, pero no se presentan como partidas jugadas. Borrar historial también borra launch_log; mantiene las preferencias de audio.

## Recuperación y riesgos concretos

Una exclusión por base de datos impide dos supervisores. El directorio BASE.launch-lock contiene owner y, mientras corre el juego, active-game. Las peticiones usan un directorio temporal 0700 y archivos create_new 0600; documentos acotados, sin reutilizar una petición anterior.

Un error recuperable de plan/spawn/exit vuelve a la GUI con un mensaje. Una biblioteca vacía o raíz desconectada se muestra vacía sin recomendar juegos ficticios ni rutas inexistentes. Si no se puede confirmar la terminación del proceso, se rechaza reiniciar gráficos y se conserva el lock/PID para recuperación independiente. No se mata automáticamente un juego ni un proceso crítico. Un cierre duro puede dejar un lock: comprobar PID y función antes de retirar ese directorio, nunca borrarlo a ciegas.

Si falla la nueva inicialización DRM/evdev o la base está dañada, no puede garantizarse recuperación completa sin hardware. El supervisor termina con error; requiere la sesión independiente y restauración comprobada de EmulationStation. Un script que daemoniza rompe el contrato de proceso foreground: debe rechazarse antes de autorizar el perfil. No existe detección automática completa de descendientes ni una garantía para scripts arbitrarios.

**No combinar aún el lanzamiento con el viejo smoke/watchdog que restaura EmulationStation tras un número fijo de segundos**: podría competir con un juego que continúa abierto. Una futura prueba debe vigilar el estado/PID del juego y restaurar solo después de comprobar su cierre. No modificar servicios ni arranque para esta prueba. Mantener SSH independiente y al usuario delante, registrar PID/estado, comprobar salida del emulador y después retorno gráfico/controles/audio. Nada de esto se afirma probado físicamente en esta fase.

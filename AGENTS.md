# EmotionalUI — instrucciones del proyecto

- Leer `docs/EMOTIONALUI_MASTER.md` y respetar `ROADMAP.md`. La petición vigente delimita el hito activo; los informes antiguos no acreditan el estado actual.
- GitHub (`https://github.com/inthe92/EmotionalUI`) es la referencia compartida. Trabajar cada objetivo en una rama y un pull request, utilizando la integración de GitHub cuando sea posible. No dejar una tarea terminada únicamente en archivos locales; entregar el enlace al PR y distinguir su estado de revisión y validación.
- Usar entornos locales o temporales para compilar, probar y capturar cuando sea necesario. Conservar el trabajo local previo y trasladar únicamente cambios revisados mediante una lista explícita de archivos; no subir carpetas completas, evidencias privadas, credenciales, ROMs, BIOS ni guardados.
- Una tarea activa cada vez. Reutilizar lo implementado y modificar únicamente lo necesario para el objetivo autorizado. Conservar cambios locales, recursos originales y respaldos.
- Priorizar resultados verificables y pruebas focalizadas. No repetir pruebas superadas, auditorías o capturas sin motivo. Distinguir siempre simulación, comandos y observación física del build identificado por hash.
- Proteger ROMs, BIOS, guardados, EmulationStation y dArkOS. No alterar arranque ni configuración global. Antes de una intervención gráfica, exigir supervisión y autorización expresa actual, SSH independiente y recuperación comprobada.
- Registrar en commits y PR los cambios del objetivo solicitado conforme al flujo de GitHub autorizado por el propietario. No fusionar PR, publicar releases, cambiar visibilidad ni incluir arte con derechos pendientes sin autorización para esa acción. Los datos privados permanecen fuera de GitHub.
- El director utiliza un solo agente para tareas sencillas; delega solo subtareas independientes y acotadas. Máximo dos subagentes activos simultáneamente. Los especialistas no crean otros agentes ni repiten investigaciones del director.
- Especialistas disponibles: `visual` (Slint y defectos visuales), `hardware` (ARM64 y diagnóstico físico seguro), `qa` (regresiones y evidencia). Heredar modelo, esfuerzo y proveedor actuales; no cambiarlos automáticamente.
- Entregar conclusiones breves: resultado, evidencia, límites y bloqueo. Detenerse al completar la tarea o alcanzar su punto de parada; consultar antes de ampliar el alcance.

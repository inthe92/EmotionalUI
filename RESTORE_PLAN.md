# Restauración independiente

`restore.sh --destination DIRECTORIO` usa Python estándar, sin ejecutar EmotionalUI. Consulta el diario propio `.emotional-install.json` y verifica todos los archivos frente a sus hashes. Rechaza archivos modificados, añadidos, symlinks y diarios desconocidos; no sobrescribe trabajo posterior.

La instalación actual no modifica el frontend ni configura el arranque, por lo que `system_changes=[]` y `backup_required=false`. Restore cambia el estado propio a restored y deshabilita el trial; conserva instalación y datos personales. **No pretende restaurar una configuración de EmulationStation que nunca se cambió**, ni ejecutar un systemctl inventado. Un diario con cambios de sistema desconocidos se rechaza.

`uninstall.sh --destination DIRECTORIO --yes` es distinto: exige restore previo, comprueba conflictos y elimina únicamente archivos inventariados y directorios propios vacíos. La SQLite personal se guarda fuera de ese árbol y permanece. Nunca borra ROMs ni guardados. No se ejecuta rm -rf sobre una instalación ajena.

Si una copia se interrumpe queda .installing con un diario propio. Si solo faltan archivos y los presentes coinciden, se puede reintentar. Un archivo parcial, un .new de estado incompleto o conflicto detiene el proceso y conserva evidencia; requiere inspección, no limpieza automática. El pequeño intervalo entre escritura del estado final y rename también falla de forma conservadora si se interrumpe. No se declara transacción infalible frente a pérdida de corriente.

Recuperación física independiente: pendiente de comprobar en la versión concreta de dArkOS. Los scripts nuevos no detienen servicios ni crean temporizadores; no garantizan recuperar una pantalla negra por sí solos. Antes de permitir instalación principal se necesitan prueba supervisada, copia/restauración del mecanismo real, rutas/PIDs y acceso independiente probado. No usar un temporizador fijo que mate juegos legítimos.

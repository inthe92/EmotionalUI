# EmotionalUI 0.2.0-beta.2 · paquete corregido e instalación

27-10-2026. Revisión de distribución de la beta azul/dorado. Se conservan la
interfaz, el giro de los degradados, los materiales GPU, Atlas y la lógica de
juegos de [beta.1](../beta-0.2.0-beta.1/REPORT.md). Sus capturas y resultados de
desarrollo identifican esa build anterior; no son capturas físicas de beta.2.

## Corrección

La comprobación en la consola detectó que el ZIP de beta.1 omitía `menu.py`,
dependencia transitiva del lanzador. Falló antes de registrar una entrada de Ports
o abrir gráficos. Beta.2 incluye ese módulo y el constructor verifica en un proceso
Python aislado que los ocho módulos de entrada se importan desde el propio paquete.
Una prueba de regresión elimina `menu.py` y comprueba que la verificación rechaza
el paquete incompleto. El tag y el ZIP anteriores se conservan como historial;
se recomienda descargar beta.2.

## Evidencia nueva

| Comprobación | Resultado observado |
|---|---|
| Regresión del paquete | 1 prueba aprobada, incluidos payload completo y dependencia ausente |
| ARM64 release | Compilación `--locked` completada con `linuxkms-gpu-material`; avisos heredados |
| ZIP copiado a RG353V / dArkOS | SHA-256 comprobado antes de extraer |
| Archivos de la instalación | 20 archivos propios verificados por manifiesto |
| Cargador dinámico en la consola | Verificación aprobada; sin iniciar la aplicación |
| `player_menu.py --check` en la consola | 114 perfiles compatibles, 16 omitidos; herramientas de Opciones detectadas |
| Entrada de Ports | `EmotionalUI - Beta`, creada solo después de superar la planificación |
| Preservación | 11 entradas anteriores sin cambios; mismo proceso activo de EmulationStation |
| Intervención gráfica / juego | No se iniciaron |
| Arranque, servicios permanentes y datos personales | Sin modificar |

La instalación usa una carpeta versionada nueva y el directorio de progreso del
jugador ya establecido. No incluye ni copia ROMs, BIOS o controladores. Los perfiles
detectados indican disponibilidad de ejecutables/cores y configuración, no que
114 emuladores o juegos hayan sido ejecutados con éxito.

## Artefactos

- ELF SHA-256: `438fc98872020572be166993aafd946063a9f915bcd1cec4f7aa2e8443a09ee7`.
- ZIP SHA-256: `be2ee5863d4aeffe5df895a0bec5be093a8de11bc6f073c2245f5a2a7d365dd8`.
- Atlas SHA-256: `bb5d2304b9b9ad86a35ac559fa998e5e41a2f090f741013542fe0601dd24c7f4`.
- [Recibo público de comprobaciones](installation-verification.json), sin dirección
  de red, credenciales, rutas personales o inventario de la biblioteca.

## Límites y recomendación

No se ha abierto esta revisión en el LCD ni iniciado un emulador. Faltan imagen,
giro del foco, audio, controles físicos/táctiles, juego, Continuar y retorno visible
del mismo ELF, además de estabilidad, latencia y rendimiento. No se atribuyen
60 FPS ni se propone sustituir EmulationStation. La suite Python histórica con
fixtures/contratos pendientes sigue documentada en el informe anterior.

Mantener la prerelease y el PR en borrador hasta una prueba física supervisada.
La próxima comprobación debe usar esta entrada de Ports con recuperación
independiente; la verificación del cargador no sustituye esa aceptación.

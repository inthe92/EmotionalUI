# Ajustes dArkOS · candidato beta.3

27-10-2026. Primera integración del menú de sistema en la build azul/dorado.
La home, las tarjetas, los materiales y el Atlas de beta.2 se conservan. No se ha
cambiado el arranque predeterminado ni publicado una nueva release estable.

## Cambios concretos

- Menú por categorías con etiquetas en español y un catálogo completo del
  dispositivo. Las aplicaciones con wrapper propio no exponen sus helpers.
- Wi-Fi y Bluetooth de la cápsula llegan a sus gestores originales mediante una
  confirmación cancelable, con Cancelar seleccionado inicialmente.
- El supervisor libera la aplicación antes de abrir herramientas; permite que
  sus copias/ajustes usen la vista original del sistema sin quitar la protección
  de la biblioteca durante los juegos. Su servicio transitorio queda vinculado
  a la sesión funcional para la recuperación.
- Se evita reabrir Slint cuando la herramienta ha devuelto el control a ES. Se
  contemplan reinicios explícitos en segundo plano y estados de servicio desconocidos.
- Las preferencias de ES admiten su formato XML de nodos hermanos. Se respetan
  las selecciones explícitas compatibles y se informan las claves sin sistema
  correspondiente, sin renombrarlas ni sustituirlas automáticamente.

## Evidencia de desarrollo

| Prueba | Resultado |
|---|---|
| Aplicación Rust | 119 aprobadas |
| Servicios Rust | 84 aprobadas |
| Python focalizado | 50 aprobadas |
| Recorrido de la aplicación | 40 pasos de botones/callbacks TouchArea, sin ejecutar herramientas; salida limpia |
| Defecto encontrado por el replay | B conservaba foco del encabezado al llegar desde la cápsula; corregido y recorrido repetido con éxito |
| Capturas GPU en Mac | Componentes reales a 640×480, claro/oscuro y confirmación; datos de inventario de desarrollo |
| ARM64 | Release `linuxkms-gpu-material`, GLIBC máxima 2.35, sin RPATH; avisos heredados |
| Paquete | Importaciones aisladas, manifiesto, hash de copia y cargador comprobados |

[Menú claro](light.png) · [Red en oscuro](network-dark.png) ·
[Confirmación Wi-Fi](wifi-confirm.png). Son poses de la interfaz real en desarrollo,
no fotografías del LCD, pruebas táctiles físicas ni mediciones de rendimiento.

## Comprobación nueva en RG353V / dArkOS

Instalada en carpeta propia nueva y entrada **Ports → EmotionalUI - Ajustes prueba**.
La beta.2 y las otras 12 entradas previas conservan sus hashes. Se verificaron 20
archivos propios y el cargador dinámico. El inventario se ejecutó mediante
`--system-settings-check`, que termina antes de inicializar Slint.

El dispositivo devuelve **40 herramientas**: 8 de red, 4 de pantalla/energía,
1 de sonido, 12 de mandos/emuladores, 8 de almacenamiento y 7 de mantenimiento.
La planificación conserva 114 perfiles compatibles y 16 omisiones. Detecta dos
claves de selección sin sistema de nombre exacto; quedan intactas y pendientes
de revisar, sin asignación por aproximación.

El archivo de ajustes de ES no cambió y EmulationStation mantiene el mismo
proceso activo. No se abrieron gráficos, juegos o herramientas del sistema en
esta comprobación. No hubo cambios de arranque, servicios permanentes, ROMs,
guardados, redes o datos personales.

- ELF SHA-256: `1e4d68e85a4d59091c5af696ec9fbef2ef0012a887aaec3d0cb334c999bb4bb5`.
- ZIP SHA-256: `7b69a7dfd03af44b9c8d4557fe7450bdd899e16bbf63fa9e99826f862521473d`.
- [Instalación e inventario sanitizados](installation-verification.json).
- [Artefacto](artifact-verification.json) · [fuentes exactas del candidato](source-manifest.json).

## Puerta pendiente

La paridad completa todavía no está conseguida: quedan selectores avanzados del
menú Start de ES, governor/overrides, emuladores independientes y controles sin
herramienta equivalente. La cesión real de terminal/mandos, el retorno de un gestor,
las escrituras de ajustes, juego/Continuar y estabilidad aún requieren supervisión.

No se prueban restauraciones, actualizaciones, desconexión de red o suspensión
para demostrar que existen. Tampoco se atribuyen FPS de desarrollo a la consola.
Mantener la entrada de prueba separada y completar estas verificaciones antes
de configurar EmotionalUI como predeterminado. [Cobertura y límites](../../SYSTEM_SETTINGS.md).

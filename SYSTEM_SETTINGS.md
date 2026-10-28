# Ajustes de dArkOS en EmotionalUI · integración en curso

Objetivo vigente: tener acceso a los controles de la consola desde ambos
lanzadores antes de valorar EmotionalUI como interfaz predeterminada. La rama
`experimental/darkos-settings-parity` conserva la beta.2 publicada. El candidato
beta.3 mantiene la home, los fondos, los materiales y la navegación existentes.

## Implementado

- Ajustes → Ajustes de dArkOS: red, pantalla/energía, sonido, mandos/emuladores,
  almacenamiento, mantenimiento y catálogo completo. Las herramientas desconocidas
  nuevas conservan acceso en el catálogo. No se muestran los scripts internos de
  aplicaciones que ya tienen su propio lanzador, como PortMaster.
- Volumen, brillo y batería reutilizan el adaptador nativo asíncrono existente.
  Los controles Wi-Fi/Bluetooth de la cápsula abren la confirmación de su gestor
  original. No hay una segunda base de redes, contraseñas o emparejamientos.
- Cada herramienta requiere una apertura explícita; Cancelar recibe el foco
  inicial. B vuelve al mismo elemento/categoría. El retorno sigue la identidad de
  la herramienta aunque el inventario cambie de orden. Botones y TouchArea usan
  el mismo controlador.
- Se conservan los programas y configuraciones de dArkOS. Los nombres conocidos
  se traducen al español sin renombrar archivos; los títulos de metadatos propios
  prevalecen. Las acciones de restaurar, borrar, actualizar o desconectar explican
  su efecto antes de ceder la pantalla.
- El lector de preferencias acepta el XML nativo de nodos hermanos que escribe
  EmulationStation, además del formato con raíz. Una selección explícita de
  emulador/core no se sustituye por otro. Una preferencia inválida o duplicada
  detiene la planificación. Las claves sin sistema correspondiente se cuentan;
  no se renombran ni se asocian por aproximación.

## Selector de emulador: preparación sin activar

El planificador identifica combinaciones RetroArch y núcleo que el XML declara,
que tienen archivos ELF de arquitectura compatible y que admiten extensiones
de la biblioteca. Si varios directorios usan el mismo nombre de sistema, sólo
propone la intersección para evitar romper una colección al cambiar otra. No
ofrece emuladores independientes ni núcleos inferidos, y comprobar cabeceras ELF
no acredita que funcionen los juegos. La lectura en la RG353V encontró 64
sistemas con 202 combinaciones y un sistema con varias carpetas; no abrió la
interfaz ni ejecutó juegos.

El inventario adicional se calcula bajo demanda con `player_menu.py --check`,
no al abrir normalmente la aplicación. Una medición única de lectura en esa
consola tardó 6,6 s con el plan anterior y 9,5 s con el inventario adicional;
no es una prueba de arranque repetida ni una medida de rendimiento gráfico.

Se puede preparar en memoria un XML con la nueva elección preservando el resto
de bytes, tanto en el formato de nodos hermanos como en el formato con raíz. Se
rechazan declaraciones DTD, duplicados y estructuras ambiguas. Las pruebas
reconstruyen perfiles usando únicamente archivos temporales propios. **Todavía no
hay selector en pantalla ni escritura de preferencias de la consola**: faltan
la transacción atómica con copia de seguridad, la protección ante cambios
concurrentes, la integración con el supervisor y la prueba física supervisada.

## Cesión a las herramientas originales

La aplicación libera gráficos, entrada y audio antes de abrir una herramienta.
Un servicio transitorio ejecuta únicamente el archivo resuelto del catálogo,
como el mismo usuario y con su terminal original. Se vincula mediante `BindsTo`
y `PartOf` a la sesión funcional identificada desde cgroup v2. No hay una unidad
permanente nueva ni comandos obtenidos del XML.

Las herramientas de backup/USB/configuración necesitan escribir sus destinos
originales. Su servicio usa la vista del sistema del anfitrión; la biblioteca
continúa de solo lectura dentro del servicio de jugador/emulación. No se relaja
la protección de los juegos para permitir ejecutar una herramienta.

Algunos scripts de dArkOS reinician EmulationStation. En ese caso termina la
sesión funcional y la recuperación original conserva la consola. No se abre Slint
sobre un ES activo, iniciándose o cuyo estado no se pueda verificar. La detección
conservadora del script también cubre los reinicios explícitos en segundo plano.
La cesión real de terminal, mandos y recuperación todavía requiere ensayo físico.

## Cobertura y límites para cambiar el arranque

| Control | Integración actual | Falta comprobar o portar |
|---|---|---|
| Batería, volumen y brillo | Adaptador nativo y cápsula existente | Lectura/escritura, persistencia y teclas reales de esta build |
| Wi-Fi, Bluetooth, contraseña, servicios remotos | Gestores originales desde Ajustes/cápsula | Entrada y retorno; conexiones sólo mediante elección local |
| Backups, restauración, USB, archivos, mantenimiento | Herramientas originales del inventario | Permisos efectivos y recuperación; no se ejecutan restauraciones como prueba |
| Mandos, reparación de audio y ajustes de emuladores | Herramientas originales disponibles | Ensayo no destructivo y configuración física |
| Emulador/core seleccionado en ES | Lectura del formato nativo y preferencias exactas | Selector propio y soporte de emuladores independientes |
| Governor global/por juego, overrides, opciones específicas de ES | Conservadas en sus archivos originales | Paridad de ejecución y edición desde EmotionalUI |
| Suspensión, LEDs, avisos, panel, undervolt y funciones ligadas al menú Start de ES | Sólo las que ofrece una herramienta instalada están enlazadas | Adaptadores y selectores nativos restantes; persistencia y retorno |
| Apariencia y sonido ambiental de EmotionalUI | Ajustes propios existentes | No equivalen al tema, salvapantallas o audio del menú de ES |
| Arranque predeterminado | Se conserva EmulationStation | Cerrar las diferencias anteriores, juego/Continuar, mandos/audio, estabilidad y recuperación supervisados |

El acceso a las herramientas originales es una primera integración funcional,
no un port nativo completo del menú Start de ES. No se afirma igualdad total de
controles ni se cambia el arranque mientras existan estas diferencias.

## Pruebas y reproducción

Las pruebas Rust cubren catálogo, límites de rutas, ocultación de helpers,
categorías, confirmación/cancelación, foco y vinculación del servicio transitorio.
Las pruebas Python verifican preferencias nativas, selecciones explícitas,
preservación del paquete y recuperación. El replay combina botones y los callbacks
de TouchArea: abrir categorías, cancelar Wi-Fi, abrir la cápsula, volver y cambiar
de sección. No activa scripts del sistema ni acredita una pantalla táctil física.

```sh
cargo test --locked -p emotional-services -p emotional-ui --no-default-features --features simulator
python3 -m unittest discover -s tests -p test_darkos_launcher_profiles.py
# Sólo inventario; no inicializa Slint ni abre herramientas:
emotional-ui --system-settings-check /opt/system
```

La auditoría del dispositivo se hace por lectura. No se ejecutan actualizadores,
restauraciones, cambios de red, suspensión o scripts de arranque para demostrar
que existen. Los archivos originales y sus secretos permanecen fuera de Git.

Fuentes consultadas: [dArkOS](https://github.com/christianhaitian/dArkOS), su
[compilador de ES para RK3566](https://github.com/christianhaitian/dArkOS/blob/master/build_emulationstation-rk3566.sh)
y el [menú de EmulationStation utilizado por ese build](https://github.com/christianhaitian/EmulationStation-fcamod/blob/503noTTS/es-app/src/guis/GuiMenu.cpp).
Se reutilizan programas instalados; no se redistribuyen sus fuentes por arrastre.

[Informe y capturas del candidato](docs/darkos-settings-2026-10-27/REPORT.md).

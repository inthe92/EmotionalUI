# Desarrollo Linux ARM64 sin consola

Se reutiliza el contexto Docker `colima-emotional-ui` ya instalado. No hace falta reconstruir la VM ni conectar SSH a la RG353V. El contenedor reproduce un espacio de usuario Debian 13 ARM64/glibc 2.41; su kernel pertenece a la VM del Mac. No reproduce Rockchip, DRM DSI, evdev físico, táctil ni EmulationStation.

## Compilar y comprobar dentro de Docker

```sh
./platform/simulator/validate.sh
./platform/simulator/run.sh
```

El primer comando reutiliza las cachés `emotional-ui-cargo-registry` y `emotional-ui-build`, ejecuta fmt, Clippy, tests y build del workspace, y comprueba adicionalmente la variante `simulator`. Genera dos ejecutables ELF ARM64 en `dist/simulator/`: `emotional-ui-simulator` y `emotional-ui-kms`, junto con los tres ejecutables de pruebas. Son artefactos de desarrollo; no constituyen un paquete certificado para dArkOS.

El segundo ejecuta la batería sin red, sin capacidades, sin dispositivos del host, con raíz de contenedor y fuentes de solo lectura, `/tmp` temporal y una carpeta nueva de resultados. Usa el UID local para guardar capturas. Las imágenes Docker se construyen solo si faltan; las bases Debian y Rust 1.99.1 tienen digest fijado. Los paquetes apt se registran en `dist/simulator/packages.txt`; sus actualizaciones no están fijadas bit a bit. La primera construcción necesita red para paquetes y crates fijados por Cargo.lock; los tests de ejecución funcionan offline.

Para guardar una ejecución:

```sh
./platform/simulator/run.sh docs/simulator/mi-prueba
```

Utiliza un directorio nuevo. Una ejecución existente no se sobrescribe. `report.json` contiene códigos y resultado; `environment.json`, arquitectura, glibc y hashes; cada caso conserva stdout/stderr. Las capturas provienen del renderer software de Slint a 640 × 480. `rasterized_frames` cuenta trabajo de CPU: **no son fotogramas presentados en LCD ni FPS físicos**.

El constructor necesita acceso de escritura solo a cachés y artefactos, y red para completar dependencias. Las fuentes se montan de solo lectura. La batería de ejecución no hereda ese acceso a red. No se cambia el contexto Docker global. Para detener la VM cuando no se necesite: `colima stop --profile emotional-ui`.

Si iCloud ha apartado archivos, hay que descargarlos antes de montar las fuentes. Docker puede devolver `Resource deadlock avoided` al leer un placeholder; eso es un fallo de acceso al archivo, no del controlador ni del algoritmo. Se conservaron las ejecuciones fallidas y los placeholders anteriores en `docs/simulator/icloud-placeholder-backups/`.

## Uso interactivo de escritorio

```sh
cargo run --locked -p emotional-ui -- --db :memory: --dev-moment autumn-rain
```

F2/F3 recorren los seis escenarios; `--at`, `--weather`, `--unknown-time` permiten contexto manual. `--dev-weather-age 181` simula datos caducados; 0–180 minutos conserva el clima y los datos futuros se ignoran. No se modifica el reloj del ordenador. `--dev-moment` y `--dev-weather-age` se rechazan en builds release sin `simulator`. Las utilidades CLI históricas de fecha y clima siguen existiendo para pruebas; no son un servicio meteorológico real.

Los seis nombres son `summer-morning`, `autumn-rain`, `halloween`, `christmas`, `winter-sunday` y `quiet-dawn`. Región y fiestas proceden de la configuración TOML; las fechas no fuerzan tradiciones a otros países. La estación usa el hemisferio norte y el día semanal se calcula con Chrono. El modo `simulator` selecciona expresamente una plataforma Slint sin pantalla; el build normal sigue abriendo una ventana nativa.

## Catálogo importado en la GUI

```sh
cargo run --locked -p emotional-ui -- --import-gamelist /ruta/gamelist.xml --rom-root /ruta/juegos --db /ruta/emotional.sqlite
cargo run --locked -p emotional-ui -- --library-root /ruta/juegos --db /ruta/emotional.sqlite
```

El segundo comando consume las tablas que crea el primero. La importación y las comprobaciones de disponibilidad se realizan antes del event loop; la navegación y sus escrituras SQLite no esperan al disco. La biblioteca presenta seis filas visibles y desplaza la ventana con el foco. No carga imágenes externas ni lanza emuladores. Si no quedan archivos disponibles muestra un catálogo vacío.

Favoritos, nombre, descripción y género se trasladan al modelo. Playcount/lastplayed permanecen como metadatos originales: no prueban una partida reanudable ni se convierten en elecciones aprendidas por EmotionalUI. Las emociones desconocidas son neutrales, sin deducirlas del género. Las ilustraciones son recursos originales del proyecto y no carátulas del juego importado.

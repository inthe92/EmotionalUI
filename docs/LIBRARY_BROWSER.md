# Biblioteca por sistemas y buscador · beta.6

Biblioteca abre con **Buscar juegos**, **Todos los juegos** y las consolas
presentes en la colección, con su recuento. Dentro de una consola, los juegos
se ordenan por título. El sistema procede de los metadatos importados y las
carpetas declaradas; los títulos sin sistema conocido quedan en «Sin sistema».
No se reorganizan archivos ni se modifican `gamelist.xml`, ROMs o partidas.

**X** abre el buscador desde Biblioteca. El teclado admite cruceta y A o pulsar
las teclas en pantalla; X borra, Start o «Buscar» ejecutan la consulta y B o
«Volver» cancelan. La búsqueda recorre los títulos de toda la colección, sin
distinguir mayúsculas o tildes. Los resultados muestran también la consola.
B vuelve a la lista de sistemas; otro B sale de Biblioteca. El aviso «Volver»
del pie también se puede tocar. Las búsquedas sin coincidencias muestran un
estado vacío y permiten volver o escribir otra consulta.

La selección se traduce a la identidad original del juego antes de usar el
lanzador existente. Tras salir de un juego se recuperan la consola o consulta
y el juego seleccionado, incluso si el orden de importación ha cambiado. Los
archivos de retorno antiguos siguen admitidos. Al actualizar la colección,
se conserva el foco por identidad de juego o sistema cuando sigue disponible.
El historial conserva su recorrido separado.

`app/src/library_browser.rs` contiene el modelo sin E/S; `library_ui.rs` conecta
sus filas y teclado con `State`. Se cachean los títulos normalizados e índices
filtrados al cambiar la colección o la consulta. Sólo se publican las seis
filas visibles al cambiar la página o el desplazamiento. No se reconstruye la
colección en cada fotograma. La interfaz reutiliza `AmbientPanel`, el foco y
el material GPU existente; Continuar y el fondo aprobado conservan su diseño.

## Verificación reproducible

```sh
cargo test --locked -p emotional-ui --no-default-features --features simulator
cargo build --locked -p emotional-ui --no-default-features --features simulator
python3 tools/verify_player_flow.py --binary target/debug/emotional-ui --output dist/library-flow
```

La suite de aplicación pasó con 129 pruebas. Las siete pruebas específicas
del navegador cubren agrupación, títulos iguales entre sistemas, búsqueda sin
tildes, edición/cancelación, vacíos, importación y recuperación de identidad.
El replay usa los callbacks táctiles de producción y eventos virtuales de
botones; abre un proceso sintético desde un resultado y lo vuelve a abrir en
modo continuar. Comprueba el retorno a la consulta, dos registros de actividad,
persistencia de apariencia y que el guardado original no cambia. No ejecuta
emuladores ni herramientas del sistema.

Las capturas a 640 × 480 se renderizan localmente con el material GPU y datos
de muestra mediante `focus_preview` (`library-systems`, `library-console`,
`library-search`, `library-results`). No son capturas de la RG353V ni una medida
de rendimiento. El paquete ARM64 se prepara separado de la versión instalada.
La navegación física/táctil, los lanzamientos reales y el rendimiento de esta
beta siguen pendientes de ensayo supervisado en la consola. No se cambia el
arranque ni se sustituye EmulationStation.

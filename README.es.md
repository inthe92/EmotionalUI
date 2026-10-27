[🇬🇧 English](README.md) | 🇪🇸 Español
# EmotionalUI
**Menos buscar. Más jugar.**

> **Nueva beta funcional:** [v0.2.0-beta.1](https://github.com/inthe92/EmotionalUI/releases/tag/v0.2.0-beta.1) · [Instalación y límites](BETA.md). Ambiente GPU azul y dorado, lanzamiento de juegos, Continuar y Atlas experimental offline. RG353V / dArkOS ARM64, con entrada propia en Ports y sin sustituir el arranque. Falta validar físicamente esta beta exacta. La [preview anterior de interfaz](https://github.com/inthe92/EmotionalUI/releases/tag/v0.1.0-preview.1) sigue disponible.


### Videojuegos para cada momento.

EmotionalUI nació una tarde lluviosa de domingo.

La luz era tenue, sonaba música tranquila y acababa de terminar un videojuego que llevaba tiempo queriendo disfrutar.

Fue uno de esos pequeños momentos en los que todo encaja.

Después regresé al menú de mi consola. Tenía miles de videojuegos disponibles. Aventuras extraordinarias, clásicos inolvidables y títulos que probablemente ni siquiera conocía.

Empecé a recorrer la lista, pasando de un juego a otro, sin decidirme por ninguno.

Finalmente apagué la consola.

**Tenía prácticamente todos los videojuegos del mundo, pero no sabía a cuál quería jugar.**

Y de aquella sensación nació EmotionalUI.

---

### Una forma diferente de disfrutar de los videojuegos

EmotionalUI es una interfaz alternativa para consolas retro que busca transformar una biblioteca de juegos en una experiencia más personal, acogedora y significativa.

No quiere mostrarte cientos de carátulas ni convencerte de que siempre necesitas jugar a algo nuevo.

Quiere ayudarte a encontrar ese videojuego que encaja con tus gustos, con el momento y con las ganas que tienes de jugar.

Una tarde lluviosa de otoño. Una noche tranquila de verano. Un domingo sin planes. O simplemente unos minutos para desconectar.

Cada momento puede ser una buena ocasión para descubrir una aventura.

### Tus aventuras tienen su lugar

EmotionalUI quiere recordar los videojuegos a los que te gusta regresar, ayudarte a continuar tus partidas y dar una oportunidad a aquellas pequeñas joyas que suelen quedar olvidadas entre los grandes clásicos.

Porque no todos los juegos necesitan ser famosos para ser especiales.

Y no todas las aventuras necesitan terminarse para haber merecido la pena.

### Una interfaz que también se siente

La luz, el clima, las estaciones, la música y los pequeños detalles visuales forman parte de la experiencia.

Queremos que encender la consola resulte tan agradable como acomodarse en un rincón favorito.

Sin prisas. Sin estímulos innecesarios. Sin interminables menús que te hagan olvidar por qué encendiste la consola.

**La tecnología debe acompañar, no reclamar nuestra atención.**

### Tu colección sigue siendo tuya

EmotionalUI está pensado para aprovechar los videojuegos que ya tienes en tu consola, respetando tu colección y la experiencia que has construido alrededor de ella.

No es una tienda. No es un servicio de suscripción. No pretende venderte nada.

Es otra forma de encontrarte con tus propios videojuegos.

### Un proyecto independiente, hecho con cariño

EmotionalUI sigue en desarrollo. Nació como una idea personal y está creciendo poco a poco, experimentando con nuevas maneras de descubrir, recordar y disfrutar de los videojuegos.

No sabemos hasta dónde llegará.

Pero si alguna vez has sentido que tienes demasiados juegos y, aun así, no sabes a cuál jugar, quizá entiendas exactamente por qué estamos construyéndolo.

Si compartes esta forma de entender los videojuegos, nos encantará conocer tu opinión, tus ideas o simplemente saber que este proyecto también significa algo para ti.

---

**EmotionalUI**

*No se trata de tener más videojuegos.*

**Se trata de darles el lugar que merecen en nuestros momentos.**


## Preview experimental para RG353V

El frontend local ya se ha ejecutado en una Anbernic RG353V real con dArkOS, controles físicos y regreso supervisado a EmulationStation. Los resultados de instalación y retorno del artefacto concreto constan en las notas de la release.

Las cuatro secciones son **Continuar, Tu momento, Biblioteca y Ajustes**. La misma habitación responde a hora local, estación, clima opcional y eventos editoriales. El modo offline no necesita meteorología; el clima real requiere ubicación y consentimiento privados. El clima DEMO no representa una observación.

El paquete `v0.1.0-preview.1` usa **software por defecto** y **Mali-G52 / FemtoVG experimental**. No activa el swap inmediato. La GPU mejora la navegación percibida, pero no se han demostrado 30 ni 60 FPS estables; hay pequeñas diferencias de rasterizado y salto de línea.

El modo prueba permite explorar la interfaz e importar una biblioteca existente a una SQLite propia. **No permite lanzar juegos.** No se incluyen ROMs, BIOS, partidas ni carátulas comerciales. `install-main` continúa bloqueado.

[Instalación y recuperación](PREVIEW_INSTALL.md) · [Preparación y notas de release](RELEASE_READINESS.md) · [Pruebas](TESTING.md) · [Licencias](THIRD_PARTY_LICENSES.md)

Rust + Slint 1.13.1; código propio GPL-3.0-only. Fuentes y dependencias conservan sus avisos incluidos. El propietario declara propia la referencia de la habitación; la procedencia está registrada. Esta demo no incluye gestión real de Wi-Fi/Bluetooth ni lanzamiento de videojuegos.

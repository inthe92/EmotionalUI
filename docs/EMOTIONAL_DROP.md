# Emotional Drop · beta de transferencia local

Desde la cápsula de controles, selecciona **Emotional Drop · Añadir juegos**.
La consola prepara un QR temporal. Conecta el teléfono a la misma red local,
escanea el código y elige uno o varios archivos. La página propone un destino
cuando sólo hay uno; si la extensión sirve para varios sistemas, solicita una
elección. Muestra el progreso y el resultado de cada archivo. Al cerrar con A,
B o el botón táctil, termina la sesión y EmotionalUI vuelve a leer la biblioteca.

La opción sólo se habilita en la sesión funcional con perfil configurado. El
servidor se inicia por petición y se detiene al cerrar el panel, al terminar la
aplicación, tras 20 minutos sin actividad o, como máximo, una hora. Usa una
clave aleatoria específica de la sesión en el QR, no habilita un servicio
permanente y escucha en la dirección local seleccionada. El transporte HTTP
en la red local no está cifrado: usa una red de confianza. No requiere una
cuenta, acceso a Internet ni datos meteorológicos.

Los destinos proceden del `es_systems.cfg` instalado y se limitan a las raíces
y extensiones del perfil de lanzamiento funcional actual. Esto asegura que el
importador de EmotionalUI pueda volver a detectar el archivo; los sistemas sin
perfil compatible aún no aparecen en Drop. No se descomprimen archivos ni se
adivina el sistema de un ZIP. Se excluyen scripts, ejecutables de sistema,
BIOS, guardados y destinos fuera de la biblioteca. La extensión es una pista,
no una validación de que la ROM sea correcta o vaya a funcionar. Ficheros
relacionados, como CUE y BIN, deben enviarse al mismo sistema con sus nombres
originales. El límite es 16 GiB por archivo y depende también del espacio y del
sistema de archivos de la tarjeta.

Cada transferencia escribe primero un archivo temporal propio y publica sin
reemplazar ningún nombre existente. Se utilizan operaciones sin sustitución o,
en sistemas de archivos sin ellas, creación exclusiva y copia. Una interrupción
normal limpia el archivo parcial; si se corta la energía en mitad de la copia
alternativa de un sistema de archivos sin enlaces ni renombrado sin sustitución,
podría quedar un archivo incompleto que deberá revisarse manualmente. Nunca se
eliminan partidas, ROMs previas ni configuraciones para completar un envío.

La sesión de juego conserva su vista de la biblioteca de sólo lectura. El
receptor se ejecuta como el mismo usuario mediante un servicio transitorio
separado, vinculado a la sesión y vigilando la vida del proceso gráfico. No se
cambia el arranque ni se reinicia EmulationStation. El servicio y su escritura
real necesitan todavía ensayo supervisado en la RG353V: la compilación ARM64,
la simulación y las pruebas locales no verifican la red, la tarjeta SD, el
lector QR del teléfono ni la recuperación en hardware.

## Comprobación local de esta beta

El paquete `0.2.0-beta.4` se compiló para AArch64 con GLIBC máximo requerido
2.35; no se ejecutó en la RG353V. El SHA-256 del ejecutable es
`20f2733ae9d03ecad29448056b485b934f21a0a4b59e9f4587f7361e306e5a32` y el del ZIP
`238d288a9410f62c9bcc03ea55b21aa22428c8de45d44e92c7e7299c71e3ca3a`.
Una prueba de navegador en el Mac envió dos archivos sintéticos a carpetas
temporales y exigió elegir sistema para un ZIP ambiguo; un detector QR leyó
correctamente la captura de desarrollo. Las pruebas automatizadas incluyen
autenticación, duplicados, rutas, interrupción y copia exclusiva alternativa.
El replay en simulador prueba la navegación, no los mandos o la pantalla físicos.

La beta.5 sustituye la capa opaca del panel QR por el material compartido del
panel de controles, y oculta los controles mientras se muestra el código. En
capturas GPU locales de los modos claro y oscuro, el QR se pudo decodificar y
un replay de 42 pasos pasó. El ejecutable ARM64 tiene SHA-256
`c81483a36b15c89fbab874218bfb2f03c6499c48fb217424c8ba90c561da608c` y el ZIP
`18715f8845e4c69314768e796d7f781013efb8743b88b69d9c9e443bc8b1e253`.
La apariencia y la lectura del QR siguen pendientes de validación en la consola.

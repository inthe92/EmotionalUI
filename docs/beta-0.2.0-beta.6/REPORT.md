# EmotionalUI 0.2.0-beta.6 · Biblioteca

Biblioteca agrupada por sistema con recuentos, lista completa y búsqueda por
nombre sin distinguir tildes o mayúsculas. Teclado táctil y de cruceta, atajo X,
borrado, cancelación y retorno a la consulta tras salir de un juego. Conserva
la identidad del juego al filtrar o cambiar el orden de importación. Se reutilizan
el lanzador y el cristal GPU existentes; la pantalla Continuar mantiene su diseño.

## Evidencia

- 129 pruebas de aplicación pasadas; las siete de Biblioteca se repitieron tras
  completar la cancelación táctil y pasaron.
- Replay final de la aplicación con selección de consola, callbacks táctiles de
  búsqueda/cancelación y botones virtuales: abre un proceso sintético, vuelve a la
  consulta y lo reanuda. Dos sesiones registradas y guardado original intacto.
- Capturas GPU locales de sistemas, teclado claro/oscuro y resultados a 640 × 480,
  con datos sintéticos. No acreditan presentación o latencia del LCD.
- Paquete y CRC del ZIP verificados; ELF AArch64, requisito máximo GLIBC 2.35,
  sin RPATH/RUNPATH. Test del empaquetador pasado.
- Instalada como **EmotionalUI - Biblioteca beta.6** en Ports. Se verificaron
  21 archivos propios y dependencias mediante el cargador nativo. La comprobación
  sin gráficos detectó 114 perfiles de lanzamiento y 16 omisiones.
- Las 14 entradas anteriores se conservaron; EmulationStation siguió activo con
  el mismo PID y su configuración conservó el hash anterior.

Ejecutable SHA-256:
`5688fbf9eb1ea5ba23aeddc1cc77856c933ce7cf6c89455339cbcfc2b5463197`

ZIP SHA-256:
`5e4126381fa70d7ec5696d26f93ecd104825e916d32c409a427b4114033e9dc2`

No se inició la interfaz, un juego, una herramienta ni el servidor Drop desde
SSH. Siguen pendientes las pruebas físicas de esta versión y la medición de
rendimiento. No se cambió el arranque ni se sustituyó EmulationStation. No se
recomienda todavía convertirla en el frontend predeterminado.

[Funcionamiento y pruebas reproducibles](../../docs/LIBRARY_BROWSER.md).

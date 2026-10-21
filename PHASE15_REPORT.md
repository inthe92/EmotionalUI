# Fase 15 · composiciones cacheadas · 21-10-2026

**Optimización parcial comprobada localmente y probada en RG353V. 60 FPS no alcanzados; recorte de título señalado por el propietario pendiente de corrección.**
Base Git d65c03c + modificaciones anteriores conservadas. Sin commit/push.
Referencia física F14 conservada: `2f0ed1698ee1fbd6dfb01251747f1c3512497e22ae60af21a2bdf8297e1e6164`.
Candidato ARM64: `8f135951f07b8438dd5ba09685e5a97cb42e9f7cc7f3ba55095166abfa0b2d1e`.

## Coste y corrección

Los82ms históricos cronometraban exclusivamente `SoftwareRenderer::render` en RAM:
evaluación de propiedades de dibujo, cálculo de regiones sucias, preparación/dibujo
de imágenes y glyphs y mezcla alfa. No separaban esas operaciones entre sí; no eran
un perfil CPU por función ni FPS. Quedaban fuera State.paint, mapeo/asignación,
RAM→mmap, espera del flip anterior y envío DRM. Ahora se mide también el tramo
completo del backend hasta submit, separado de los timestamps del kernel.

El movimiento lejano invalida grandes regiones de las pinturas y máscaras. Las
máscaras SVG ya se reutilizaban como rasters desde F14, pero sus imágenes seguían
mezclándose. Slint software1.13.1 no implementa caché de subárboles mediante
cache-rendering-hint; no se cambia motor ni EmbedFiles/fuentes.

- `scene_cache.rs`: una composición RGB640×480 de las dos pinturas base y otra
  RGB664×129 de la parte **opaca** del primer plano. Se mantiene el escalado anterior.
- Las24filas de unión difuminada siguen originales. Parallax±6/±12px, orden de
  iluminación/grade/clima/eventos y velo de lectura permanecen intactos.
- Claves: ambas identidades de imagen, mezcla cuantizada como el renderer, tamaño
  y escala. Foco/desplazamiento no regeneran. Clima/estación/eventos/paleta de UI
  siguen capas independientes; no se hornean ni deben invalidar estas dos pinturas.
- Un buffer por composición, sustitución al cambiar clave, fallback original ante
  tamaño/decodificación/alpha incompatibles. Sin temporizador nuevo de repintado.
  Memoria de píxeles adicional máxima:1.178.568bytes (~1,124MiB); no cientos de escenas.

## Comparación local reproducible — Mac, NO RG353V

Mismo ejecutable release simulator, mismo catálogo/ambiente/recorrido, XRGB RAM.
Dos pasadas intercaladas por variante; medias de las medianas/p95 de cada pasada:

| Variante | Raster mediana | p95 | CPU proceso | Pico RSS máximo |
|---|---:|---:|---:|---:|
| Caché desactivada |8,824ms|16,119ms|1,322s|103,03MiB|
| Dos composiciones |8,283ms|15,089ms|1,273s|102,77MiB|

Reducción local modesta:6,1% de la mediana. No atribuir la diferencia de RSS a una
reducción de memoria de la caché. Preparación local inicial ~0,226+0,183ms; una vez
por composición en7pintados del modelo. Experimento B sólo base: tres pasadas
7,94–8,10→7,28–7,41ms; C cerca opaca aporta ~0,3ms adicional en otro bloque A/B.
Se conservan todas las pasadas; no mezclar sus condiciones para fabricar una cifra.

18 capturas reales Slint/XRGB640×480 idénticas píxel a píxel: invierno/nieve/Navidad,
otoño/lluvia/Halloween, verano/despejado y madrugada/niebla, ambas vistas; incluye
trayectorias R/L muestreadas exactamente en0/90/180/270/450ms. Sonda de congelación
sólo simulator; no se usa para tiempos ni se compila en KMS. El control coincide
además con una captura de la referencia F14. Fuente, saltos, colores y geometría
no cambian en estas comprobaciones; no equivale a aprobación artística humana.

## Presentación e integración

Backend optativo: requests/coalescing, intentos/raster completos, submit aceptado,
modeset inicial, evento PageFlip del CRTC, secuencia/timestamp monotónico del kernel,
intervalos, espera y pendientes. P99 sólo>=100muestras, almacenamiento acotado16.384,
sin log por frame. El último flip puede quedar pendiente; no se añade drenaje.
La cadencia global incluye reposo: no llamarla FPS de animación ni medir fotones.
Slots de refresco sin nueva UI no equivalen a eventos perdidos.

Build macOS simulator y ARM64 linuxkms-direct correctos; SDK/ELF y loader real
glibc2.41 pasan, sin sustituir bibliotecas. Pruebas focalizadas:4caché,8motion,
2máscaras,3simulator,2estadísticas DRM y32pasos de inversión/modal/reducido.
fmt y diff-check pasan. No se repitió la suite completa ni benchmarks de biblioteca.

Evidencias privadas: `PRIVATE_VALIDATION/phase15-20261021-45cc1153`.
Reproducción: `tools/check_phase15_cache.py --binary ELF_SIM --seed SEED --fixtures FIXTURES --out NUEVA --mode visual|profile`.
Paquete privado: `package/payload.tar.gz`, arte provisional embebido, NO redistribuir.

## Límites

No hay separación CPU interna glyph/alpha/decodificador ni medida de fragmentación.
No hay latencia input-to-photon ni garantía de duración visible360ms bajo carga.
Se conserva el ritmo del modelo360/240ms y las animaciones interrumpibles.
Duración visible de cada transición y eventos físicos perdidos no medidos. No inferirlos de360ms ni de las acciones recibidas.

## Ensayo físico supervisado · mismo ELF, caché off / layers

Ambos60s, salida0, biblioteca propia clonada del mismo seed, contexto DEMO idéntico,
sin audio/juegos. A39acciones y B36acciones: recorrido manual, **no idéntico**;
la diferencia observada no es una garantía causal del14,1% en cualquier recorrido.
La mejora controlada reproducible sigue siendo la prueba Mac anterior.

| Último agregado disponible | A sin caché | B dos composiciones |
|---|---:|---:|
| Raster mediana /p95 /p99 |76,146 /89,192 /93,824ms|65,402 /80,713 /91,038ms|
| Backend hasta submit mediana /p95 |77,668 /91,154ms|66,573 /82,091ms|
| Copia RAM→DRM mediana |0,813ms|0,809ms|
| Intervalo kernel entre flips, mediana |83,395ms|66,716ms|
| Rasterizados /flips confirmados |141 /139|172 /170|
| Pico RSS muestreado |112,23MiB|111,97MiB|
| CPU acumulada del servicio |21,017s|22,224s|

Se trata del último agregado (el backend puede sobrevivir al cierre del bucle),
no de un contador completo final. Un flip pendiente sin observar y un modeset
inicial por variante;0errores submit/receive, timestamps monotónicos sin errores.
Ventanas kernel completas incluyen reposo:2,500 y3,243actualizaciones/s; **no son
FPS de animación**. Medianas de intervalos tampoco certifican cadencia estable.
El modo DRM nominal es60Hz; eso no implica60 imágenes nuevas/s.

Reposo observado sin acciones, excluyendo11s iniciales y1s de asentamiento tras
acción: A3,19% de un núcleo durante10,03s; B24,07% durante15,04s, distinto estado
UI. No se ha demostrado reposo bajo en ambos ni aislado ese consumo de B.
Las esperas bloqueantes medianas del flip anterior ~0,02ms; copia ~0,8ms frente
al raster65ms: el coste dominante medido sigue dentro de SoftwareRenderer::render.
No se atribuye su resto a un componente particular sin otro perfil.

El propietario confirma retorno visible a EmulationStation y una interfaz fluida,
pero no60FPS; señala texto de la tarjeta alternativa de Tu momento cortado arriba.
Ese componente no se modificó en F15. Una captura sintética adicional antes/después
coincide, pero **no reproduce con certeza el título físico observado**; origen y
corrección pendientes, sin cambiar fuente/tamaño para ocultarlo. No hay aprobación
artística de esta entrega ni afirmación humana de preferencia A/B.

Recuperación: EmulationStation activo con DRM master, brillo130 intacto, ningún
EmotionalUI restante; dos SSH independientes cerrados tras confirmación visual.
Sin tocar arranque, ROMs, guardados, configuraciones permanentes ni bibliotecas.
Evidencia: `physical/summary.json`, logs, eventos y muestras1Hz; hash del archivo
recuperado verificado. Próximo trabajo mínimo: reproducir y corregir el recorte
concreto; para rendimiento, separar el coste interno restante antes de otra caché.

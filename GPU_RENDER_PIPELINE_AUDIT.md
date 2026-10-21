# GPU render pipeline · investigación dirigida · 21-10-2026

Estado: inspección estática y prueba A preparadas. **Sin ensayo físico nuevo**: el propietario indica que no está junto a la consola. Publicación pausada; no se cambia el renderer predeterminado ni se modifica VSync.

## Código comprobado

Slint y FemtoVG fijados en **1.13.1**, según Cargo y fuentes locales instaladas. Se inspeccionan `internal/renderers/femtovg/{itemrenderer,images,lib}.rs`, `core/graphics/image.rs`, `core/items.rs` y el lowering de `cache-rendering-hint` del compilador de esa versión. No se modifican esas fuentes ni se crea otro fork del renderer.

En EmotionalUI: `assets/scenes/daycycle/assets.slint`, `app/ui/integrated.slint`, `home.slint`, `system-controls.slint`, el backend LinuxKMS ya existente y sus trazas DRM/etapas de Fases15.2–15.3.

[Renderer_GLES10.cpp de FCAMOD, rama503noTTS](https://github.com/christianhaitian/EmulationStation-fcamod/blob/503noTTS/es-core/src/renderers/Renderer_GLES10.cpp): creación/actualización/destrucción de texturas separadas y dibujo de geometría mediante GL. Referencia conceptual, sin copiar código ni atribuirle FPS físicos. Las consultas a los enlaces específicos TextureData.cpp/Font.cpp no devolvieron contenido; no se afirma haber verificado sus cachés en esta intervención.

## Nine-slice: resultado contrastado

Hay **32 referencias nine-slice**: ocho momentos por cuatro superficies (`material`, `material-focus`, `tile`, `tile-focus`). Los PNG son64×64; bordes16px o12px. AmbientPanel/CompactGame las consumen mediante AmbientSurface. Acciones, dock y cápsula usan otros PNG sin nine-slice.

La [rama sospechosa de draw_image_impl](https://github.com/slint-ui/slint/blob/v1.13.1/internal/renderers/femtovg/itemrenderer.rs) requiere **ambas** condiciones: `fit.tiled.is_some()` y región fuente distinta del original. Genera una imagen temporal repetible y la añade a `textures_to_delete_after_flush`.

EmotionalUI no configura `horizontal-tiling` ni `vertical-tiling`. Su valor predeterminado es `ImageTiling::None`. La función real `fit9slice`1.13.1, con `(None,None)`, deja `tiled=None` en sus nueve regiones. Dos tests ejecutados llaman esa función exacta: seis tamaños reales de tarjeta con PNG64×64; todas las regiones quedan fuera de la rama. El control positivo con `Repeat` sí entra en la condición.

**Conclusión B:** descartada la creación temporal *por esa rama de repetición*. Su número esperado por evaluación de materiales actuales es0 por construcción, **no un contador GL medido físicamente**. No se prepara B2 para corregirla. Nine-slice aún implica varias regiones/dibujos; su coste total no se ha medido ni se declara gratuito. Asignaciones por tintes, máscaras, opacidad u otros mecanismos siguen sin contar; esta conclusión no significa cero texturas temporales en toda la aplicación.

## Capas: qué puede reutilizarse

El compilador transforma `cache-rendering-hint` en `Layer`. `visit_layer` usa `render_and_blend_layer`; `render_layer` consulta `graphics_cache`, reutiliza una textura del mismo tamaño y sólo repinta hijos si la entrada queda invalidada. La propiedad no garantiza reutilización si cambian tamaño, hijos, texto, foco o contexto.

EditorialArtwork ya utiliza clipping redondeado: `visit_clip` llama a `render_layer`. Algunas opacidades también generan capas automáticamente. No se añaden cachés duplicadas a esas carátulas ni a toda la pantalla.

Un candidato C, si A permite continuar, es el contenido estático de una tarjeta principal, con su traslación en el padre exterior a la capa. Debe comprobarse creación, repintados/invalidaciones, aspecto y memoria. No se activa aún. Almacenamiento teórico RGBA8 de448×192:0,328MiB; de400×208:0,317MiB por superficie, **sin** buffers/driver/stencil/alineación ni contenido fuera de caja. Memoria GPU residente e invalidaciones reales: NO MEDIDAS.

## Puertas A/B/C

| Prueba | Estado | Métricas nuevas |
|---|---|---|
| A: una imagen RGB opaca preparada192×160, movida a través de640×480 | ARM64 compilado y auditado; paquete privado preparado; NO EJECUTADA en consola | FPS/pageflips, CPU, RSS y errores físicos: NO MEDIDOS |
| B1/B2: evitar textura temporal de repetición | Omitida: predicado falso en los componentes actuales | Dos pruebas puras de Slint pasan; no benchmark GPU ni ahorro inventado |
| C1/C2: caché explícita de una tarjeta | Pendiente del resultado A | Asignaciones, reutilización, coste y memoria: NO MEDIDOS |

A usa el mismo backend LinuxKMS/FemtoVG, driver del sistema, política de swap intacta y trazas DRM/etapas. No tiene biblioteca, textos, máscaras, materiales, clima ni audio. La imagen RGB se prepara una vez en Rust; asignaciones GL reales todavía no contadas. Movimiento nativo Slint de360ms, repetido cada900ms durante24s. No es el replay de13acciones del producto: prueba capacidad mínima del backend; no certifica rendimiento de EmotionalUI completo ni su planificador Rust.

Inicio de cada movimiento y llegada del offset al destino se registran desde AfterRendering, antes de submit. `tools/analyze_pipeline_probe.py` utiliza exclusivamente confirmaciones DRM entre esas marcas, conserva grupos incompletos y no cuenta el último flip pendiente. Mediana/p95; p99 sólo con100intervalos. No concede automáticamente una etiqueta60FPS por mediana, temporizador o llamadas de dibujo. CPU/RSS requieren el muestreador externo existente.

## Artefactos, pruebas y siguiente paso

- ELF A: `ecd6ed82ffec36872b75c7747a5e323961383b8c08c50ef275af5875d941bdad`,5.141.248bytes, ELF64AArch64, dependencias SDK auditadas. Loader físico de este artefacto NO PROBADO.
- Paquete privado local: `~/Library/Application Support/EmotionalUI-Validation/gpu-pipeline-20261021-202744/package/pipeline-A-private.tar.gz`; SHA-256 `2d317570b0c9587367f24fe32ddd6bd9136472ccee72eb7a47568b53d18c19be`. Incluye el software anterior sólo para reutilizar el contrato de seguridad/manifest existente; no es un paquete público.
- Dos tests Rust de fit9slice, cuatro Python de análisis/seguridad y un test de límites del prototipo en host pasan. Prototipo ARM64 compila; fmt/diff-check pasan. La combinación LinuxKMS-only no compila en macOS por features OpenGL; el test host usa desktop+linuxkms-gpu y no abre una ventana. No se alteran dependencias para ocultarlo.
- Fuentes/cambios anteriores y binarios se conservan. Sólo se añaden diagnóstico opcional, analizador y pruebas; no hay optimización integrada, cambios de diseño, del renderer predeterminado, commits o push.

**Recomendación:** realizar únicamente A con supervisión y nueva autorización, dos SSH, preflight y retorno independiente. Si A no presenta con cadencia suficiente, detener tarjetas/cachés y acotar el camino mínimo. Si A lo permite, medir C en una tarjeta; B por texturas temporales queda descartada. Los13,10/19,84/22,00Hz de Fase15.3 son históricos del frontend completo, no resultados de esta prueba ni evidencia de60FPS.

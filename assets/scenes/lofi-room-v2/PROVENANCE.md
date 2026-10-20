# Habitación lofi — adaptación de la referencia elegida

Referencia: Foto 1.jpg adjuntada por el usuario el 18-10-2026. Ruta y SHA-256 en `docs/visual-correction/reference.json`. Se inspeccionó antes de editar. El original con UI y carátulas comerciales no se copia al repositorio.

Herramienta: imagegen integrada, modo edición de la referencia, después de aplicar la habilidad imagegen. Salida conservada en `.codex/generated_images/01a14bff-9e20-7112-a20d-1c9d67d2718c/exec-af584420-55b6-4f15-8e8e-06289a4b746c.png`. Reducción técnica con `sips -z 480 640` a `master.png`, RGB opaco de 640 × 480.

Se retiraron textos, iconos, paneles, botones y carátulas de la referencia mediante edición de imagen; se conserva la composición reconocible de habitación oscura, ventana al lago y montaña, plantas, mesa, libros, taza, lámpara cálida y cama. La zona antes tapada por controles necesita relleno interpretado: no se afirma una reconstrucción exacta de píxeles ocultos.

La dirección visual es la elegida por el usuario. Este fondo es todavía un recurso candidato, no una ilustración definitiva aprobada. Las portadas de la aplicación son SVG ficticios del proyecto y siguen siendo recursos de prueba. La imagen del fondo no se presenta como captura del software: los PNG de `docs/visual-correction/final` proceden de Slint.

## Prompt íntegro

Use case: precise-object-edit. Edit target: the provided EmotionalUI conceptual reference. Produce ONLY the same underlying room background, with ALL interface content removed and inpainted. Remove every logo, letter, clock, date, status icon, separator line, translucent UI panel, button, game title, game artwork, character/cover illustration and bottom navigation. Keep the room's existing exact camera, perspective and composition: dark intimate lofi wood room; large window centered/right looking onto a dusk lake, little hillside village lights and distant mountain under muted peach sunset clouds; warm glowing lantern on the desk at lower right, books and dark mug beside it, trailing plants around the right and left edges, quiet bed/cushion at lower right. Preserve the blue-grey dusk outside and warm amber interior lantern light. Keep left third dark, naturally shaded and low detail so native UI can later be layered over it. Inpaint the formerly obscured parts as continuous room/background, no new furniture or changed positions. Original 4:3 landscape framing, no crop, no zoom. Do not create a UI mockup or a new design direction. No lettering, symbols, interface, posters, game covers, recognizable copyrighted game art, people or watermark. This background will be embedded in actual native software and reused with lighting layers, not presented as an application screenshot.

## Alcance del checkpoint local propuesto

Los derechos de redistribución de la referencia aportada y de su adaptación siguen pendientes. master.png queda expresamente fuera del checkpoint propuesto; la revisión anterior lo necesitaba para compilar la GUI. El checkpoint actual referencia el provisional original assets/scenes/checkpoint-room-v1/master.png y no necesita recuperar esta imagen privada para compilar. Conservar este documento no concede autorización para distribuir la imagen. El registro privado docs/visual-correction/reference.json se conserva en los respaldos y no se incorpora automáticamente al checkpoint.

Identificación del PNG original conservado: SHA-256 `d0f93e1d8ac75eb5dd5365fc2dc9a4add3ae5c06280f492a9465d9d44c9c7d2c`. Esta nota describe la propuesta; no declara creado ni aprobado el commit.

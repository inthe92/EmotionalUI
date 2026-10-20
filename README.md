# EmotionalUI

**Menos buscar. Más jugar.** Frontend nativo Rust + Slint, diseñado para 640 × 480 y una futura integración con dArkOS en Anbernic RG353V.

Referencia vigente: [especificación maestra y estado real](docs/EMOTIONALUI_MASTER.md) · [hitos](ROADMAP.md) · [pruebas existentes](TESTING.md).

Versión de trabajo 0.2.0, no estable. Interfaz de cuatro destinos comprobada en simulación; el build actual no está validado físicamente en RG353V. Las pruebas físicas anteriores corresponden a otra revisión. Hito activo: consolidación documental, pendiente de revisión del propietario. Los apartados numerados por fase de este README son históricos; no definen el estado vigente. [Simulador y comandos Docker](SIMULATOR.md).

## Checkpoint local de fuentes

La GUI usa `assets/scenes/checkpoint-room-v1/master.png`, un recurso provisional original generado localmente; los fondos anteriores con revisión pendiente permanecen intactos y fuera del commit. No es una aprobación artística ni una release pública.

Las pruebas musicales generan los exports omitidos en temporales; la producción de piano muestreado necesita las muestras CC0 fijadas, NumPy y FFmpeg (sin descargas automáticas). Dos contratos de arte histórico y las comparaciones con capturas anteriores son opcionales y pendientes: solo se ejecutan con originales autorizados y `EMOTIONAL_PRIVATE_SCENE_TESTS=1`. Los demás tests conservan sus comprobaciones. La lista pública de avisos transitivos sigue pendiente de revisión.

## Ejecutar

```sh
. "$HOME/.cargo/env" # si cargo no está en PATH
cargo run --locked -p emotional-ui
cargo run --locked -p emotional-ui -- --at 2026-10-31T21:30 --weather rain
cargo run --locked -p emotional-ui -- --config assets/themes/default.toml --reduced-motion
```

También puedes abrir `run.command` desde Finder. Rust y las versiones de dependencias están fijadas. En macOS se requieren herramientas de compilación; en Linux de escritorio, bibliotecas y una sesión X11/Wayland para Winit. No se utiliza Electron ni un navegador.

## Experiencia y controles

Dock compartido con Continuar, Descubre, Biblioteca y Ajustes. En la misma habitación lofi, Continuar ordena hasta tres partidas y Descubre ofrece una recomendación y una alternativa. Biblioteca, cuatro paletas, Glass sin blur dinámico y movimiento reducido. Los **seis juegos de demostración son ficticios**. Con biblioteca importada y un perfil autorizado, Continuar utiliza el adaptador de procesos y regresa al frontend al finalizar. El ciclo se ha probado con procesos controlados; todavía no con un emulador en la RG353V.

| Teclado | Acción |
|---|---|
| Abajo / arriba | Entrar en alternativas / volver al protagonista |
| Izquierda / derecha | Navegar entre partidas o destinos del dock; derecha desde el protagonista enfoca Biblioteca en el dock |
| Z / Enter / espacio | Elegir; promover alternativa al protagonista |
| X / Esc | Cerrar panel o volver |
| N | Otro camino |
| A | Biblioteca; arriba/abajo recorren filas |
| S / F1 | Menú rápido; Esta partida permite fijar/retirar/completar |
| D | Opciones del juego enfocado (atajo opcional) |
| Q / E | Alternar Continuar / Descubre (L/R) |
| F2 / F3 | Escenarios emocionales de desarrollo |
| [ / ] | Franjas de desarrollo |
| M | Alternar movimiento reducido durante la sesión |

Arriba desde el protagonista enfoca la cápsula; A/confirmar abre y B/volver cierra. Los cinco estados de sistema aún dicen «Sin verificar» y no modifican hardware. El dock distingue sección activa de foco.

`--at YYYY-MM-DDTHH:MM` congela una hora civil local. Sin esa opción se lee el reloj del sistema, sin afirmar sincronización NTP. El clima es simulado y no hay llamadas de red. Batería/Wi-Fi permanecen sin integrar; no se inventan lecturas.

## Datos y diagnósticos

SQLite propia en `$XDG_DATA_HOME/emotional-ui/demo.sqlite` o `~/.local/share/emotional-ui/demo.sqlite`. Se conserva y migra de forma aditiva el esquema anterior. `--db :memory:` evita persistencia. `--db RUTA` selecciona una base propia. `--demo-count 0|1|2` permite comprobar catálogos pequeños; `--unknown-time` prueba el estado sin fecha fiable.

```sh
cargo run --locked -p emotional-ui -- --import-gamelist /ruta/gamelist.xml --rom-root /raiz/roms --db /ruta/emotional.sqlite
```

Importa solo archivos existentes dentro de la raíz y conserva metadatos/procedencia en una tabla separada. El XML y los juegos originales permanecen intactos. La importación termina por CLI; `--library-root /raiz/roms --db /ruta/emotional.sqlite` carga ese catálogo en la GUI. Sin un perfil explícito no se ejecutan juegos. El adaptador requiere `--launcher-config /ruta/launcher.toml`, biblioteca y base persistente con rutas absolutas; véase [GAME_LAUNCHER.md](GAME_LAUNCHER.md). Las referencias a carátulas se conservan sin duplicar la colección; la GUI puede decodificar miniaturas PNG/JPEG locales de forma acotada en un worker.

## Verificación

```sh
./tests/verify.sh
EMOTIONAL_UI_GRAPHICAL_TESTS=1 ./tests/verify.sh
cargo check --locked -p emotional-core -p emotional-services --target aarch64-unknown-linux-gnu
SLINT_SCALE_FACTOR=1 cargo run --locked -p emotional-ui -- --db :memory: --at 2026-10-18T21:30 --snapshot docs/refugio.png
```

La verificación gráfica necesita una sesión de escritorio. Capturas reales a 640 × 480: [Despertar](docs/despertar.png), [Horizonte](docs/horizonte.png), [Refugio](docs/refugio.png), [Silencio](docs/silencio.png).

## Documentación

[Informe de auditoría](AUDIT_REPORT.md) · [Pruebas reales](TESTING.md) · [Arquitectura](ARCHITECTURE.md) · [Diseño](DESIGN_SYSTEM.md) · [Integración dArkOS](DARKOS_INTEGRATION.md) · [Hoja de ruta](ROADMAP.md) · [Licencias](THIRD_PARTY_LICENSES.md)

[Build ARM64](ARM64_BUILD.md) · [Diagnóstico físico](HARDWARE_DIAGNOSTIC.md) · [Informe de fase 3](PHASE3_REPORT.md).

Siguiente hito: validar este build y sus controles en la RG353V mediante una prueba supervisada, con recuperación independiente. 60 FPS, respuesta <100 ms y RAM <150 MB son objetivos sin verificar físicamente.

Docker: `./platform/simulator/validate.sh` y `./platform/simulator/run.sh`. Mapeo configurable: [INPUT_MAPPING.md](INPUT_MAPPING.md).

### Sonido local y movimiento discreto (fase 8)

Start/S → Sonido permite elegir Minimalista, Ambiental o Silencio y ajustar los volúmenes; izquierda/derecha regula, A/Z cambia y B/X vuelve. Los sonidos de interfaz están desactivados inicialmente. Los ajustes se guardan en la base local. Para una ejecución silenciosa: `cargo run -p emotional-ui --locked -- --audio-mode silence`. `--audio-volume 0..100` y `--audio-assets DIR` son opciones de desarrollo; no escriben preferencias por sí solas; un ajuste posterior en el menú o el lanzamiento configurado conserva el estado efectivo para el regreso.

La composición de estudio está en `assets/audio/previews/la-habitacion-permanece.m4a` (también WAV y Ogg), con MIDI y score editables en `assets/audio/source`. Dura 100 segundos y contiene silencios intencionados; no es un felt piano definitivo. No necesita Internet ni un instrumento instalado para ejecutarse. Generación reproducible: `python3 tools/compose_audio.py --output-root CARPETA_NUEVA`; exportación con FFmpeg ya instalado: `tools/export_audio.sh CARPETA_NUEVA`.

La habitación se desplaza unos pocos píxeles al cambiar de vista. Es un panorama de un solo plano, anulado por movimiento reducido o desactivar movimiento ambiental. No hay animación permanente ni bloqueo de controles. El protocolo de audio ahora está conectado con Continuar y el supervisor de procesos. Espera el cierre del frontend antes de abrir el proceso configurado; el regreso crea una instancia nueva y recupera ajustes y foco. Validado con procesos sintéticos, pendiente RetroArch y hardware. Detalles en [PHASE9_REPORT.md](PHASE9_REPORT.md) y [LAUNCH_LIFECYCLE.md](LAUNCH_LIFECYCLE.md).

### Dos candidatas musicales de fase 9

Preescuchas originales con piano vertical muestreado: [A · Refugio](assets/audio/phase9/candidates/refugio/refugio.m4a) y [B · Silencio](assets/audio/phase9/candidates/silencio/silencio.m4a). También en `Escritorio/EmotionalUI-Fase9`. Conservan MIDI, partitura, WAV/Ogg y procedencia CC0 del instrumento. Son candidatas pendientes de escucha humana; el banco anterior continúa como predeterminado. Producción reproducible en [MUSIC_PRODUCTION.md](MUSIC_PRODUCTION.md).

### Continuar inteligente y Atlas piloto (fase 10)

Hasta tres partidas según actividad confirmada y decisiones explícitas, sin inventar guardados. Start → Esta partida permite fijar, retirar, recuperar desde biblioteca, completar/en curso y corregir interés. [SMART_RESUME.md](SMART_RESUME.md) explica los pesos configurables.

`--atlas assets/atlas/emotional-atlas.db` activa el piloto offline y la reserva persistente. Tras importar una biblioteca, `--db BASE --atlas ATLAS --atlas-link ID_LOCAL=atlas:snes:000001` enlaza una identidad de forma explícita. No se identifica una ROM por parecido de nombre. El piloto contiene 52 títulos SNES contrastados y propuestas emocionales pendientes de revisión; no se afirma una mejora editorial validada. [Pipeline y formato](EMOTIONAL_ATLAS.md) · [Comparación real](RECOMMENDATION_EVALUATION.md).

## Estado vigente · Fase 11

Atlas dispone de un subconjunto experimental de 21 obras SNES con fuentes primarias y revisión humana pendiente. Activación optativa: `--atlas assets/atlas/emotional-atlas-0.2.0-experimental.1.db --atlas-experimental`; los enlaces locales siguen siendo explícitos. [Revisión editorial](ATLAS_EDITORIAL_REVIEW.md).

`--trial --db BASE_PROPIA --library-root RAÍZ` abre una vista previa sin demos ni lanzadores: importa XML en segundo plano y reutiliza miniaturas locales PNG/JPEG. [Modo prueba y límites](TRIAL_MODE.md). Scripts de archivos propios en platform/distribution; no configuran arranque ni sustituyen EmulationStation. Instalación principal y distribución pública **no disponibles** todavía. Véanse [preparación](RELEASE_READINESS.md), [instalación](INSTALLATION_DESIGN.md) y [restauración](RESTORE_PLAN.md).

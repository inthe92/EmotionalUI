# EmotionalUI KMS software patch

Source: crates.io `i-slint-backend-linuxkms` 1.13.1, same pinned dependencies and
upstream source/attribution/license notices. Used under GPL-3.0-only, consistent
with EmotionalUI. No GPU backend or system library is added.

`renderer/sw.rs` renders into one retained CPU RAM buffer, then copies that
complete result once to the DRM dumb buffer. Original triple buffering, page
flips, device selection, rotation and input remain unchanged. At 640×480 XRGB,
the retained buffer uses 1,228,800 bytes. The RAM buffer preserves its content,
so repaint tracking uses `ReusedBuffer` after its first initialization.

With explicit `EMOTIONAL_PERF=full|static|simple`, `EMOTIONAL_KMS_DIRECT=1` selects
upstream direct-to-mmap rendering for a controlled same-binary diagnostic.
Outside diagnostics the variable has no effect. Bounded timings (16,384 samples
per category) are printed at most once every two seconds of rendering and at adapter drop: CPU raster, copy, and present
scheduling. These are not measured LCD frame or input-to-photon timings.

Fase 14 fixes diagnostics flushing: backend lifetime did not guarantee Drop at app close. Aggregates also flush during rendering; no timer or log work is introduced in idle.

Memory microbenchmark `tools/profile_drm_memory.c` allocates its own off-screen
buffer; it never takes DRM master or modifies displayed framebuffers. Physical
results and acceptance are recorded by the supervised validation, not inferred
from compilation.

Fase 14 pixel fast paths: valid premultiplied texels with alpha0 leave the destination untouched; alpha255 replaces it directly. Partial alpha still uses the upstream formula. Simulator mirrors the XRGB target; tests cover all65,536 source/destination alpha pairs and real frame equivalence. Same retained RAM/copy/presentation path.


Fase 15 measures WindowAdapter redraw calls/coalescing, backend render attempts,
and completed CPU raster calls independently. `raster_ms` wraps `render()`:
drawing-time property evaluation, dirty-region handling, image/glyph work and
pixel composition are inside; prior application model updates, shadow allocation,
DRM mapping, shadow-to-mmap copy and presentation are outside.

DRM diagnostics consume the existing page-flip completion events without adding
waits, draining at shutdown, timers or repaints. Kernel sequence and microsecond
timestamp endpoints, confirmed count and interval quantiles are distinct from
successful page-flip ioctl submissions and initial modesets (no completion event).
`drm` 0.14's PageFlipEvent `duration` field actually contains kernel tv_sec/tv_usec;
intervals are differences between those timestamps. Reported update Hz requires
the driver's monotonic timestamp capability. The window includes idle periods;
sequence gaps are refresh slots without a new UI update, not proof of lost events.
The last accepted flip may remain unobserved until the next render; it stays
explicitly pending. CPU submit-to-event-observation includes delayed event reading
and is not kernel submit-to-flip latency. These counters still do not measure
LCD photon timing or physical input-to-photon latency.

New DRM wait/interval/observation samples are bounded to 16,384 per category
(~384 KiB of f64 data plus bounded report copies). Aggregate output remains
limited to the existing once-per-two-seconds-of-rendering and shutdown reports.
P99 is omitted below 100 samples. LinuxFB has no DRM event confirmation and does
not produce fictitious kernel counters.

`backend_draw_copy_submit_ms_NOT_LCD` spans a successful backend draw request:
map/allocation, property/draw work, RAM raster, mmap copy, wait for the previous
flip, and submission of this flip. It excludes aggregate reporting afterward,
the earlier application/input queue, and this frame's eventual kernel flip/LCD
scanout. Its samples preserve the same bounded/opt-in behavior as the partials.

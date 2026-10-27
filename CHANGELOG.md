# Changelog

## 0.2.0-beta.2 — 2026-10-27

Fixes the beta package's missing transitive `menu.py` launcher dependency, found
by the device preflight before a Ports entry was registered or graphics started.
The package builder now imports all entry modules in an isolated Python process
and verifies they resolve from the payload. A regression test removes `menu.py`
and confirms packaging verification rejects the incomplete delivery.

Visuals, rotating gradients, Atlas data and game-launch logic are unchanged.
Supersedes the incomplete beta.1 ZIP; existing installations remain preserved.

## 0.2.0-beta.1 — 2026-10-27

Restores the blue/gold GPU build while retaining its Slint screens and navigation.
Adds the existing supervised player launcher, resume, isolated progress and return
to the frontend. Unprofiled games remain launchable from Biblioteca. Appearance
stays manual and persistent; light-mode focus now uses warm amber/linen, with the
previous night focus retained. Focus colours rotate on GPU; reduced motion freezes
the gradient and the software fallback stays static.

Connects the latest completed experimental Atlas export: 1,841 provisional
identities and 274 proposed profiles, with unknown dimensions preserved and no
claim of complete review. Exact metadata/platform matching uses the existing
reader; explicit SQLite profiles cannot be mixed silently with an older fallback.

Ships as a separate, reversible ARM64 Ports beta. No ROMs, BIOS, drivers, personal
history or boot/service configuration changes. Current-beta hardware acceptance,
full legacy visual-suite reconciliation and sustained-FPS validation remain open.

## 0.1.0-preview.1 — 2026-10-22

First experimental interface preview for Anbernic RG353V / the tested dArkOS environment. Four sections, original ambient room, contextual recommendations, existing-library import and preserved phase13–15 navigation, 360 ms interruptible transitions, parallax and composition cache.

Software default; Mali-G52/FemtoVG experimental, without immediate-swap default or a sustained30/60FPS promise. Trial mode cannot launch games. System controls remain informational. No boot replacement, ROMs, BIOS, commercial covers, drivers or system libraries are bundled.

Graphical Ports entries, own-files installation/removal, independent transient return to EmulationStation, corresponding sources and dependency notices. The exterior masks now respect the window sill and foreground cup; original paintings are unchanged. Audio starts quietly and may require a menu volume adjustment. Hardware/artifact acceptance and checksums are recorded in the release notes.

## Unreleased · 0.2.0 development

Phase 11: opt-in, source-assisted experimental SNES subset (21/52 works), separate approved profiles, original explanations and reproducible comparison worksheet. Human editorial review remains pending. Brief-session suitability now participates in the existing bounded ranking.

Trial preview uses real XML/SQLite with background import, unchanged-source stamps and bounded local PNG/JPEG thumbnails. Missing libraries remain empty. No boot/frontend changes or emulator launches in trial mode.

Own-files-only package/install/restore/uninstall prototypes, checksums and conflict/interruption tests. Main installation is explicitly unavailable. No public release or physical certification.

Earlier phases: three-game Continue, personal preferences, contextual discovery reserve, two lofi views, scene/audio engines, controlled-process supervisor and ARM64 builds. Their desktop/container validation does not demonstrate a real emulator cycle in RG353V.

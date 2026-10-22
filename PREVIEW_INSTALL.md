# EmotionalUI v0.1.0-preview.1 — installation and recovery

Experimental, opt-in interface preview. Software is the default; Mali-G52 / FemtoVG is experimental. Trial mode cannot launch games or replace your default frontend. Keep EmulationStation and your original library. No Rust, Codex, compilation, SSH or terminal is needed for normal use.

## Compatibility

Target: Anbernic RG353V, Linux ARM64, dArkOS. The observed image reports Debian 13 in `os-release`, dArkOS `(08272026)` in its Plymouth title, and RG353V in its device marker. Kernel 5.10.226 and glibc 2.41 were observed. Other images and RG353VS/R36S are not certified. An ELF ARM64 header alone does not establish compatibility. Python 3, systemd, noninteractive permission to run the existing transient-unit mechanism, the native loader and required libraries must already exist; no drivers or system packages are installed.

## Graphical installation

1. Download `EmotionalUI-RG353V-v0.1.0-preview.1.zip` and verify the SHA-256 shown with that release. Use the SHA256SUMS asset from the same release.
2. Extract its contents into the **existing Ports directory configured by EmulationStation** on the ROM card. Keep the hidden `.emotional-ui-preview-package` folder. Do not overwrite an existing EmotionalUI extraction; preserve it separately. The selected console currently uses its second card's Ports directory, but paths must be checked rather than guessed.
3. Refresh EmulationStation's game list using its normal controls if necessary. Select **EmotionalUI — Modo prueba**. Its first run verifies hashes, model, firmware and native loader, then copies only manifest-owned files to `~/.local/opt/EmotionalUI-preview-1`.
4. The software entry is the default. **EmotionalUI — GPU experimental** selects FemtoVG explicitly and checks its native dependencies before releasing the screen. If unavailable, select software; no driver replacement occurs.
5. Explore Continue / Your moment / Library / Settings with L/R, D-pad, A/B and Start. Some system capsule controls remain informational. Analog sticks and all system-capsule hardware functions are not certified.
6. Close through Ajustes → Volver a la consola. The independent transient systemd unit starts EmulationStation after normal exit, failure or the 15-minute trial safety limit. Trial mode cannot launch games.

This path requires no Rust, compilation, SSH or terminal for normal use. Installation time depends on your card and firmware; a ten-minute installation is not claimed.

## Recovery and removal

- Close EmotionalUI first. Select **EmotionalUI — Restaurar** to disable this own-files installation. It does not restore an OS image: boot and system files were never changed.
- Select **EmotionalUI — Desinstalar** to restore and remove only hash-verified installed files. Modified or unknown files cause refusal. The original extracted ZIP contents and personal database remain; remove only those known extraction files with the file manager if desired.
- Personal data, cover cache and diagnostic errors stay under `~/.local/share/emotional-ui`, outside the install directory. Existing ROMs, BIOS, saves and `gamelist.xml` are not edited.
- A GPU failure does not need a working GPU process for fallback: recovery is owned by PID1, and the separate software menu entry remains available.
- The supervised acceptance test retains two independent SSH connections for recovery. That is a developer safety measure, not an end-user installation step. If the exact installer cannot restore the menu reliably, do not release it.

## Audio, library and privacy

Only original runtime audio and licensed fonts are included. Audio starts quietly; hardware audibility is pending. The importer reads the existing library; without gamelists it may be empty. No commercial game artwork or synthetic play history is bundled. Weather remains optional, with no location, weather credentials or automatic network consent supplied by the package. Demo weather must be labelled DEMO.

## Español

Preview experimental y reversible. Extrae el ZIP en **Ports**, conservando la carpeta oculta del paquete, y abre **EmotionalUI — Modo prueba**. Software es la opción inicial; GPU es experimental. No necesita Rust ni SSH durante el uso normal. El modo prueba no lanza videojuegos. Sal desde Ajustes para volver a EmulationStation. Restaurar deshabilita la instalación propia; Desinstalar sólo elimina archivos propios verificados y conserva tus datos. No cambia arranque, ROMs, BIOS, partidas ni gamelist. No se promete una instalación en diez minutos ni 30/60 FPS estables.

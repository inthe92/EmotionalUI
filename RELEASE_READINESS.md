# EmotionalUI v0.1.0-preview.1 — release candidate preparation

21-10-2026. **BINARY RELEASE BLOCKED ON PHYSICAL ZIP ACCEPTANCE.** The source repository is now PUBLIC by explicit owner instruction; anonymous access returns HTTP200. No release tag or downloadable GitHub Release has been created. Current source synchronization is authorized; the original working tree is preserved.

## Source and renderer

Local base: `d65c03c8306ed95e42a3110d734133a1dbba04f9` plus preserved working-tree changes from phases 13–15. Remote main is `2f0176f8ebb218b276a65b5fcb87725865a8ff95`, two documentation-only commits ahead. Both remote README texts are preserved as the base of the updated bilingual README; no remote source replaced local work and HEAD was not moved.

Last physically tested GPU: `209d68e5dc0eaced18e2b0baff84abfcb5ef9b56bd26bf98bd847513795a0966`. Software reference: `ea717b33d7d45efa47c9a8294a28e775dd0ff056aedc6a318fa280b6f4c67695`. These are **private development binaries**, not public release artifacts; they embed the selected artwork, now declared project-owned by the owner; they still are not the final tested ZIP. The later title-overflow correction has local evidence but still needs the final physical package test.

Software remains default; Mali-G52/FemtoVG remains experimental. `EMOTIONAL_GPU_SWAP=immediate` is not a release default. Three equivalent phase15.3 trials yielded grouped cadence 13.10 / 19.84 / 22.00 Hz (software / GPU / immediate experiment), p95 approximately 100 ms. No sustained 30/60 FPS claim. Do not resume optimization during distribution work.

## Release gates

| Gate | Status / evidence |
|---|---|
| Selected room redistribution | **CLEARED FOR PACKAGE PREPARATION BY OWNER DECLARATION (21-10-2026):** “La foto1 es nuestra.” Recorded in selected-room-v1/PROVENANCE.md under the existing project artwork license. No independent ownership verification is claimed. Seven paintings are embedded in both ELF variants; the original reference with UI/covers is not bundled. Publication is expressly authorized after verification; changing PRIVATE visibility still requires a specific decision. |
| Dependencies and notices | Cargo ARM64/GPU inventory refreshed, notices collection reports any missing upstream license texts. 15 missing license texts recovered from pinned commits; the two MIT-only upstream declarations are preserved verbatim with identified canonical MIT standard supplements and no invented copyright. Notes distinguish runtime drm-fourcc from build-time simd_helpers. Source and notice inventory reviewed for candidate packaging. |
| Current source | Explicit hash-pinned allowlist in platform/distribution/preview-files.json; includes patched vendor backend and embedded-resource sources. No staging or source consolidation commit yet. |
| Dual binaries | Existing private ELF variants preserved. Fresh ARM64 software/GPU candidates rebuilt as Cargo 0.1.0-preview.1, with Rust/C source-path remapping to remove personal Mac build paths. The exact ZIP still needs hardware acceptance. Existing physically tested development binaries are retained. |
| Installer | Own-files installer retained. dArkOS Debian identity handled conservatively. Prepared graphical Ports entries use independent transient recovery, software default and explicit GPU alternative. 11 new and 16 existing installer tests pass locally. ZIP extraction mode loss is handled for the two explicit ELF entry points. Local tests are not hardware acceptance. |
| Exact ZIP | Local candidate generated from reviewed pinned sources and matching binaries by tools/package_preview.py; checksums and full corresponding sources included. This is not physical installation certification or public publication. Exact artifact identity is in the private candidate result until acceptance. |
| Physical installation | **NO PROBADA:** obtain current supervision and specific authorization; install the exact ZIP, exercise controls/views/audio, exit, restore and uninstall. Keep two SSH connections and independent recovery. Do not modify boot/games. |
| Screenshots/video | Existing captures are real program renders. Room provenance is now declared project-owned; inspect their remaining contents and obtain final publication approval before exposing them. Record authentic LCD video after ZIP acceptance, no mockups as evidence. |
| Public history/privacy | Limited scan of the two local commits found no tested text-secret patterns; both retain the real author name/email. A new scan covered all 284 historical blobs and the explicit source selection; no checked secret/path/ROM signatures or PNG textual/EXIF metadata found. Author names/emails remain normal Git metadata; changing visibility is a separate pending decision. No blanket repository export. The full clearance_evidence record is also included in the package and must contain only reviewed public-safe evidence, never private correspondence, local paths or credentials. |
| Publication | Owner expressly authorizes verified commit, push, tag and pre-release. The visibility change is expressly authorized and verified PUBLIC; actual physical installation acceptance is still pending. Do not publish the ZIP before that acceptance. |

## Prepared release metadata — not created

Tag `v0.1.0-preview.1`; title **EmotionalUI — First Public Preview**; type **Pre-release**.

Draft notes: experimental exploration-only frontend for RG353V/dArkOS. Four sections, ambient room, optional offline-first weather, compact cards and 360 ms transitions. Software default; Mali-G52 GPU experimental, with independent return and software entry. No games launched in trial mode, no ROMs/BIOS/commercial covers, no boot replacement. Small renderer text differences and variable frame pacing remain known limitations. Installation/removal instructions: PREVIEW_INSTALL.md; Spanish introduction: README.es.md. Attach the tested ZIP and its actual SHA-256 only after validation; no fabricated download link/hash or ten-minute claim.

## Next acceptance step

Room provenance is cleared for preparation by owner declaration. Notices/privacy selection reviewed and both variants rebuilt with path remapping. Verify the candidate ZIP, then complete the requested supervised installation/open/exit/restore/uninstall test and visibility decision. Publication approval itself has already been granted. Reddit draft and 35–45-second real-device storyboard remain unpublished in docs/phase11/presentation/.

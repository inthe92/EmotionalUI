# Security

This is an experimental development build, not a supported stable release. Public hosting and a private reporting channel have not been configured; do not publish secrets in an issue or include private library paths/logs in a public report. A release must establish a private contact before requesting vulnerability submissions.

Gamelist XML is untrusted data, never a launcher command. Launch profiles require explicit authorization and structured arguments. Trial preview blocks emulator launching. Installer/restore scripts modify only hash-verified owned files, refuse conflicts and do not configure boot/services. Checksums provide integrity, not publisher authentication.

Current boundaries: no general hostile filesystem race guarantee, incomplete hardware recovery validation, no authenticated automatic updater, no descendant-daemon process contract, and asset/dependency redistribution review pending. Do not replace EmulationStation with this candidate. Preserve independent recovery and backups for any later supervised integration.

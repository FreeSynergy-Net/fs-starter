# fs-starter v0.0.6

Release fix for 0.0.5 (0.0.5 is tagged but has no release: the package build was red because the test helper `fs-starter-fake-module` was a `[[bin]]`; it is an `[[example]]` now). Same pins (fs-forms v0.0.11, fs-interfaces v0.0.31, fs-rpc v0.0.7, fs-components v0.0.34). This is the first release since v0.0.4, so it carries the host changes of 0.0.5 as well: command registry, `SystemPackaged` verifier and system paths, sources adapter, RPM package renamed `fs-starter` (obsoletes `fs-starter-host`; the deb file is still named `fs-starter-host_...`). Details: CHANGELOG.md 0.0.6.

Windows: der Host findet und startet noch keinen Dienst; Programme mit Modulen laufen unter Windows noch nicht. Sockets/Herzschlag-Grenzen 4, 5-e2e, 7 ungeprüft.

RPM: the folders below /usr/share/freesynergy and /usr/libexec/freesynergy are not owned by any package yet (`rpm -qf`: not owned); empty folders stay behind after the uninstall (cargo-generate-rpm cannot build directory assets yet).

RPM: die Ordner unter /usr/share/freesynergy und /usr/libexec/freesynergy gehören noch keinem Paket (rpm -qf: not owned); leere Ordner bleiben nach der Deinstallation liegen.

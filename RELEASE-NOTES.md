# fs-starter v0.0.7

v0.0.6 was tagged but never published: the origin check of the publish step expected the old org name (FreeSynergy instead of FreeSynergy-Net). 0.0.7 = the same code + new workflow pins. It is the first release since v0.0.4, so it carries everything of 0.0.5 and 0.0.6 (command registry, `SystemPackaged` verifier and system paths, sources adapter, RPM package renamed `fs-starter`, obsoletes `fs-starter-host`; the deb file is still named `fs-starter-host_...`). Details: CHANGELOG.md 0.0.7 and 0.0.6.

Windows: der Host findet und startet noch keinen Dienst; Programme mit Modulen laufen unter Windows noch nicht. Sockets/Herzschlag-Grenzen 4, 5-e2e, 7 ungeprüft.

RPM: the folders below /usr/share/freesynergy and /usr/libexec/freesynergy are not owned by any package yet (`rpm -qf`: not owned); empty folders stay behind after the uninstall (cargo-generate-rpm cannot build directory assets yet).

RPM: die Ordner unter /usr/share/freesynergy und /usr/libexec/freesynergy gehören noch keinem Paket (rpm -qf: not owned); leere Ordner bleiben nach der Deinstallation liegen.

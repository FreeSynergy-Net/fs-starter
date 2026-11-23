# fs-starter v0.0.6

Release fix for 0.0.5 (0.0.5 is tagged but has no release: the package build was red because the test helper `fs-starter-fake-module` was a `[[bin]]`; it is an `[[example]]` now). Same pins (fs-forms v0.0.11, fs-interfaces v0.0.31, fs-rpc v0.0.7, fs-components v0.0.34); no change of the host.

Windows: der Host findet und startet noch keinen Dienst; Programme mit Modulen laufen unter Windows noch nicht. Sockets/Herzschlag-Grenzen 4, 5-e2e, 7 ungeprüft.

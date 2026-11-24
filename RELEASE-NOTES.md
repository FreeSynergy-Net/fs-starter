# fs-starter v0.0.8

What is new since v0.0.7 (details in CHANGELOG.md): the host binds the command names of a module itself (`<module>.<key>`, SW-5; a module names only the contract key); one way of writing a version need, `name@<req>` (V-5); the deb package is now called `fs-starter` like the rpm and replaces `fs-starter-host` (apt upgrade from 0.0.4 and 0.0.7 removes the old package); Windows host groundwork: one place for the program file name, the shared crate `fs-trusted-path` for the system-place check, anchor `FOLDERID_ProgramFiles\FreeSynergy`; pins on the FreeSynergy-Net chain (fs-forms v0.0.12, fs-rpc v0.0.8). The code of this release was written with the help of Claude (Anthropic).

Honest note on 0.0.7: its release carries two package names (rpm `fs-starter`, deb `fs-starter-host`); from 0.0.8 both are `fs-starter`.

Not proven: the Windows path code is built and checked in CI, but not run on a Windows machine with a real installed module; the host does not yet find and start a module service on Windows.

Neu in 0.0.8 (Details in CHANGELOG.md): der Host bindet die Befehlsnamen eines Moduls selbst (`<modul>.<schlüssel>`, SW-5); eine Schreibweise für einen Versionsbedarf, `name@<req>` (V-5); das deb heißt jetzt `fs-starter` wie das rpm und ersetzt `fs-starter-host`; Windows-Grundlage (eine Stelle für den Dateinamen, das gemeinsame Crate `fs-trusted-path`, Anker `FOLDERID_ProgramFiles\FreeSynergy`); Pins auf die FreeSynergy-Net-Kette (fs-forms v0.0.12, fs-rpc v0.0.8). Der Code wurde mit Hilfe von Claude (Anthropic) geschrieben.

Ehrlicher Hinweis zu 0.0.7: das Release trägt zwei Paketnamen (rpm `fs-starter`, deb `fs-starter-host`); ab 0.0.8 heißen beide `fs-starter`.

Nicht belegt: der Windows-Pfadcode ist gebaut und in der CI geprüft, aber nicht auf einem Windows-Rechner mit einem echten installierten Modul gelaufen; der Host findet und startet unter Windows noch keinen Modul-Dienst.

RPM: the folders below /usr/share/freesynergy and /usr/libexec/freesynergy are not owned by any package yet (`rpm -qf`: not owned); empty folders stay behind after the uninstall (cargo-generate-rpm cannot build directory assets yet).

RPM: die Ordner unter /usr/share/freesynergy und /usr/libexec/freesynergy gehören noch keinem Paket (rpm -qf: not owned); leere Ordner bleiben nach der Deinstallation liegen.

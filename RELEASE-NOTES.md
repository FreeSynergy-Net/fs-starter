# fs-starter v0.0.10

**Arbeitsversion**

Das ist eine Arbeitsversion von FreeSynergy, gedacht zum Ausprobieren. Sie ist bisher nur oberflächlich getestet; die gründliche Sicherheitsprüfung steht noch aus. Vieles kann sich noch ändern, auch die Art, wie Daten gespeichert werden – Daten aus dieser Version überstehen ein Update möglicherweise nicht. Vertrau ihr deshalb nichts an, was du nicht verlieren darfst. Fällt dir etwas auf, freuen wir uns über deine Rückmeldung.

**Working version**

This is a working version of FreeSynergy, meant for trying things out. So far it has only been tested lightly; the thorough security review is still to come. A lot can still change, including how data is stored – data from this version may not survive an update. Please do not trust it with anything you cannot afford to lose. If you notice something, we are glad to hear from you.

What is new since v0.0.9 (details in CHANGELOG.md, section 0.0.10): `--check-system` no longer reads a form of the kind `single-record` (a module refuses it without a record id, so a healthy module showed red); it names such a form as `yellow single-record-not-probed` instead; the host bridge now has a clock: under `--sample-data` the window takes the "now" of the sample data, and `--sample-now T` (RFC 3339) overrides it; pins moved up the FreeSynergy-Net chain (pin waves 11 and 12: fs-forms, fs-rpc, fs-interfaces, fs-components f2e90244 with the calendar words and the clock in the draw context `DrawCtx`, fs-modules-planning); pin-12 was checked beforehand in a Fedora container with installed packages: `--check-system --program tasks` red=0. Written with the help of Claude (Anthropic).

Honest note: session, help, MCP and the update source are still placeholders (`--check-system` reports them yellow; the last run in the container showed red=0 yellow=5). The host does not find or start a module service on Windows yet; programs with modules do not run there. This is a pre-release, not for production use.

Neu seit v0.0.9 (Details im CHANGELOG.md, Abschnitt 0.0.10): `--check-system` liest keine Form der Art `single-record` mehr (ein Modul lehnt sie ohne Datensatz-Id ab, so zeigte ein gesundes Modul rot); stattdessen meldet es eine solche Form als `yellow single-record-not-probed`; die Host-Brücke hat jetzt eine Uhr: unter `--sample-data` nimmt das Fenster das „Jetzt“ der Beispieldaten, `--sample-now T` (RFC 3339) überschreibt es; Pins in der FreeSynergy-Net-Kette angehoben (Wellen pin-11 und pin-12: fs-forms, fs-rpc, fs-interfaces, fs-components f2e90244 mit den Kalenderwörtern und der Uhr im Zeichenkontext `DrawCtx`, fs-modules-planning); pin-12 wurde vorab in einem Fedora-Container mit installierten Paketen geprüft: `--check-system --program tasks` red=0. Der Code wurde mit Hilfe von Claude (Anthropic) geschrieben.

Ehrlicher Hinweis: Sitzung, Hilfe, MCP und Update-Quelle sind noch Platzhalter (`--check-system` meldet sie gelb; der letzte Lauf im Container zeigte red=0 yellow=5). Der Host findet und startet unter Windows noch keinen Modul-Dienst; Programme mit Modulen laufen dort noch nicht. Vorabversion, nicht für den Produktivbetrieb.

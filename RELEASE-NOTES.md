# fs-starter v0.0.9

**Arbeitsversion**

Das ist eine Arbeitsversion von FreeSynergy, gedacht zum Ausprobieren. Sie ist bisher nur oberflächlich getestet; die gründliche Sicherheitsprüfung steht noch aus. Vieles kann sich noch ändern, auch die Art, wie Daten gespeichert werden – Daten aus dieser Version überstehen ein Update möglicherweise nicht. Vertrau ihr deshalb nichts an, was du nicht verlieren darfst. Fällt dir etwas auf, freuen wir uns über deine Rückmeldung.

**Working version**

This is a working version of FreeSynergy, meant for trying things out. So far it has only been tested lightly; the thorough security review is still to come. A lot can still change, including how data is stored – data from this version may not survive an update. Please do not trust it with anything you cannot afford to lose. If you notice something, we are glad to hear from you.

What is new since v0.0.8 (details in CHANGELOG.md, section 0.0.9): `--check-system` (a check command: it reports what the system is missing, red or yellow); the host takes up a watch that the service ended (`ended`, status 410) again and catches up through `changes`; the guarded read right `form.read` (fs-rpc `Guarded`) now applies; `--sample-data`, the host bridge: every source is served by sample data made from the description, no service, no socket, no store; the RPM now `Requires: freesynergy-filesystem` (a package of the FreeSynergy Repository); many pins moved up the FreeSynergy-Net chain (fs-forms, fs-rpc, fs-interfaces, fs-components). Written with the help of Claude (Anthropic).

Honest note: session, help, MCP and the update source are still placeholders (`--check-system` reports them yellow). `--sample-now` has no effect until F4b. This is a pre-release, not for production use.

Neu seit v0.0.8 (Details im CHANGELOG.md, Abschnitt 0.0.9): `--check-system` (ein Prüfbefehl: er meldet, was dem System fehlt, rot oder gelb); der Host nimmt eine vom Dienst beendete Beobachtung (`ended`, Status 410) wieder auf und holt über `changes` nach; das Lese-Recht `form.read` unter `Guarded` (fs-rpc) gilt jetzt; `--sample-data`, die Host-Brücke: jede Quelle liefert Beispieldaten aus der Beschreibung, ohne Dienst, Socket und Speicher; das RPM hat jetzt `Requires: freesynergy-filesystem` (ein Paket aus dem FreeSynergy-Repository); viele Pins in der FreeSynergy-Net-Kette angehoben (fs-forms, fs-rpc, fs-interfaces, fs-components). Der Code wurde mit Hilfe von Claude (Anthropic) geschrieben.

Ehrlicher Hinweis: Sitzung, Hilfe, MCP und Update-Quelle sind noch Platzhalter (`--check-system` meldet sie gelb). `--sample-now` hat bis F4b keine Wirkung. Vorabversion, nicht für den Produktivbetrieb.

# FreeSynergy starter

FreeSynergy starter: the host that starts a program from its blueprint package (`scaffold.toml` + `forms/` + texts) without compiling it. The host owns the parts that are the same in every program (session, update, appearance, help, the MCP switch, SCIM/OAuth); the modules are built once and run as services. Built with help from Claude (AI). Licensed under MIT OR Apache-2.0.

Status (0.0.2): the library `crates/fs-starter` (words, server gate, frontend pick, exit codes) and the host binary `fs-starter` (`crates/fs-starter-host`). The host opens the window of a program from its description: `fs-starter --description PATH` (PATH is a blueprint folder, a `forms/` folder or one file; a bare PATH works too). Single instance, session, update, help and MCP are not built yet and are named once on stderr (`skeleton-part-pending`). Plan: ADR draft "Program without build".

## Auf Deutsch

FreeSynergy starter: der Host, der ein Programm aus seinem Bauplan-Paket (`scaffold.toml` + `forms/` + Texte) startet, ohne es zu kompilieren. Der Host haelt die Teile, die in jedem Programm gleich sind (Sitzung, Update, Darstellung, Hilfe, MCP-Schalter, SCIM/OAuth); die Module werden einmal gebaut und laufen als Dienste. Entstanden mit Hilfe von Claude (KI). Lizenz: MIT OR Apache-2.0.

Stand (0.0.2): die Bibliothek `crates/fs-starter` (Woerter, Server-Tor, Frontend-Wahl, Exit-Codes) und das Host-Binary `fs-starter` (`crates/fs-starter-host`). Der Host oeffnet das Fenster eines Programms aus seiner Beschreibung: `fs-starter --description PFAD` (PFAD ist ein Bauplan-Ordner, ein `forms/`-Ordner oder eine Datei; ein blosser PFAD geht auch). Einzel-Instanz, Sitzung, Update, Hilfe und MCP sind noch nicht gebaut und werden einmal auf stderr genannt (`skeleton-part-pending`). Plan: ADR-Entwurf "Programm ohne Build".

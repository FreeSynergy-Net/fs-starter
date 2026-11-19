# FreeSynergy starter

FreeSynergy starter: the host that starts a program from its blueprint package (`scaffold.toml` + `forms/` + texts) without compiling it. The host owns the parts that are the same in every program (session, update, appearance, help, the MCP switch, SCIM/OAuth); the modules are built once and run as services. Built with help from Claude (AI). Licensed under MIT OR Apache-2.0.

Status: empty workspace (scaffold only, one empty library crate `crates/fs-starter`, no code yet). The crate `fs-starter` moves in from the repo `fs-rpc` (code only moves, with its history); the host binary (`fs-starter <program>` / `--description PATH`) follows. Name decided on 2026-11-19 (user proposal, strategy). Plan: ADR draft "Program without build".

## Auf Deutsch

FreeSynergy starter: der Host, der ein Programm aus seinem Bauplan-Paket (`scaffold.toml` + `forms/` + Texte) startet, ohne es zu kompilieren. Der Host haelt die Teile, die in jedem Programm gleich sind (Sitzung, Update, Darstellung, Hilfe, MCP-Schalter, SCIM/OAuth); die Module werden einmal gebaut und laufen als Dienste. Entstanden mit Hilfe von Claude (KI). Lizenz: MIT OR Apache-2.0.

Stand: leerer Workspace (nur Geruest, noch kein Code). Das Crate `fs-starter` zieht aus dem Repo `fs-rpc` um (Code zieht nur um, mit Historie); danach folgt das Host-Binary (`fs-starter <programm>` / `--description PFAD`). Name beschlossen am 19.11.2026 (Vorschlag des Nutzers, Strategie). Plan: ADR-Entwurf "Programm ohne Build".

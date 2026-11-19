# Changelog

Version = tag.

## Unreleased

- The crate `fs-starter` moves in from the repo `fs-rpc` (`crates/fs-starter` with its 12 commits of history, as of fs-rpc v0.0.3). The dependency on `fs-rpc` becomes a git tag (v0.0.3), `Cargo.lock` is regenerated, `.fs-lock-sources.toml` (strict) is added. The code is unchanged; tests (all features) and clippy pedantic are green. Next: `fs-rpc` v0.0.4 without the crate; `fs-forms-bin` points at this repo.
- Auf Deutsch: Das Crate `fs-starter` zieht aus dem Repo `fs-rpc` hierher um (`crates/fs-starter` mit 12 Commits Historie, Stand fs-rpc v0.0.3). Die Abhaengigkeit auf `fs-rpc` wird ein Git-Tag (v0.0.3), `Cargo.lock` neu erzeugt, `.fs-lock-sources.toml` (strict) ergaenzt. Der Code ist unveraendert. Danach: `fs-rpc` v0.0.4 ohne das Crate; `fs-forms-bin` zeigt auf dieses Repo.

- New repository with a Cargo workspace holding one empty library crate `fs-starter` (scaffold only, no code): licences (MIT OR Apache-2.0), README, `check.yml` (fs-release v0.0.15) and `windows.yml`. The crate `fs-starter` of the repo `fs-rpc` moves in later. Written with the help of Claude (AI).
- Auf Deutsch: Neues Repository mit Cargo-Workspace und einer leeren Bibliothek `fs-starter` (nur Geruest, kein Code): Lizenzen (MIT OR Apache-2.0), README, `check.yml` (fs-release v0.0.15) und `windows.yml`. Das Crate `fs-starter` aus dem Repo `fs-rpc` zieht spaeter ein. Entstanden mit Hilfe von Claude (KI).

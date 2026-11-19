# fs-starter

The program shell of every FreeSynergy program as a library: the `main` of a program becomes `fs_starter::main_exit(&parts)`. It reads the words (`--version`, `--help`, `serve`, `mcp --as ACTOR`, `--frontend`, `--socket`, `--fresh`, `--resumed`), makes sure the server runs (over `fs-rpc`), picks the frontend (word, then the person's choice, then the standard frontend; the standard frontend is a default the person can change), reads its AppStream manifest, checks it, and opens it, or falls back and says so. Every failure is a code with parameters, never a sentence.

Status: probe of the trait-delta process, not released. The feature `conformance` (never in `default`) adds the suite `starter` on top of `fs-conformance`; a program uses it as a dev-dependency only.

Written with the help of Claude (Anthropic). Licence: MIT OR Apache-2.0.

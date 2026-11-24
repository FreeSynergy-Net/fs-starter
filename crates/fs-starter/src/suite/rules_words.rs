//! The rules of the words that open nothing: `--version`, `--help`, a wrong word, `mcp`.
//!
//! Pattern: Strategy (one struct per rule, generic over the fixture: fs-conformance review condition A1).

use super::check::{exit, gate, nothing_opened, rule, stdout, told};
use super::codes::{SuiteCode, keys};
use crate::codes::{StarterCode, keys as starter_keys};
use crate::names::parse_version;
use crate::program::Exit;
use crate::tell::Level;
use crate::words::synopsis;

/// The actor the MCP rules hand over.
const ACTOR: &str = "claude-strategie";

rule!(
    /// `--version` writes `fs-<name> <major.minor.patch>` and nothing else, exit 0, no server, no frontend.
    Version,
    "starter.version",
    |f, _cx| {
        let (want, form) = {
            let p = f.program();
            (
                format!("fs-{} {}\n", p.name(), p.version()),
                parse_version(p.version()).is_some(),
            )
        };
        let o = f.start(&["--version"]);
        exit(&o, Exit::DONE)?;
        stdout(&o, &want)?;
        if !form {
            return Err(fs_conformance::Coded::new(SuiteCode::WrongOutput)
                .with_text(keys::STREAM, "version")
                .into());
        }
        gate(&o, 0)?;
        nothing_opened(&o)?;
        Ok(())
    }
);

rule!(
    /// `--help` writes the synopsis, exit 0, no server, no frontend.
    Help,
    "starter.help",
    |f, _cx| {
        let want = synopsis(f.program().name());
        let o = f.start(&["--help"]);
        exit(&o, Exit::DONE)?;
        stdout(&o, &want)?;
        gate(&o, 0)?;
        nothing_opened(&o)?;
        Ok(())
    }
);

rule!(
    /// A wrong word: exit 2, `usage` with the word, told as an error; no server, no frontend.
    Usage,
    "starter.usage",
    |f, _cx| {
        let o = f.start(&["--no-such-word"]);
        exit(&o, Exit::USAGE)?;
        let note = told(&o, Level::Error, StarterCode::Usage)?;
        super::check::param(note, starter_keys::WORD, "--no-such-word")?;
        gate(&o, 0)?;
        nothing_opened(&o)?;
        Ok(())
    }
);

rule!(
    /// `mcp` without `--as`: exit 2, `actor-missing`; the MCP entry is not called, nothing else runs.
    McpNeedsActor,
    "starter.mcp-needs-actor",
    |f, _cx| {
        let o = f.start(&["mcp"]);
        exit(&o, Exit::USAGE)?;
        told(&o, Level::Error, StarterCode::ActorMissing)?;
        if !o.mcp_actors.is_empty() {
            return Err(fs_conformance::Coded::new(SuiteCode::Launched)
                .with_text(keys::WHAT, "mcp")
                .into());
        }
        gate(&o, 0)?;
        nothing_opened(&o)?;
        Ok(())
    }
);

rule!(
    /// `mcp --as <actor>`: the entry is called once, with exactly that actor (never one taken from the session).
    McpActor,
    "starter.mcp-actor",
    |f, _cx| {
        let o = f.start(&["mcp", "--as", ACTOR]);
        exit(&o, Exit::DONE)?;
        if o.mcp_actors != [ACTOR] {
            return Err(fs_conformance::Coded::new(SuiteCode::ActorChanged).into());
        }
        nothing_opened(&o)?;
        Ok(())
    }
);

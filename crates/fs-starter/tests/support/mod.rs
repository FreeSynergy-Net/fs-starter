//! Shared fakes of the probe tests: a demo program and small recording fakes for every port of the starter.
#![allow(dead_code)] // every test file uses a part of it

use std::ffi::OsString;
use std::path::PathBuf;

use fs_starter::{
    Exit, Failure, Found, FrontendId, FrontendManifest, InterfaceVersion, Place, Program,
    ProgramName, Trust,
};
use fs_types::coded::{Code, Coded, ErrorCode, Param};

/// The codes of the demo program: `demo-error-x`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemoCode {
    X,
}

impl ErrorCode for DemoCode {
    const PREFIX: &'static str = "demo-error-";
    const ALL: &'static [Self] = &[Self::X];

    fn as_str(self) -> &'static str {
        "x"
    }
}

/// `demo 0.0.1`, app id `net.freesynergy.Demo`, contract `demo@1.2.0`, standard frontend `simple`.
pub struct Demo {
    pub name: ProgramName,
    pub version: String,
    pub contract: InterfaceVersion,
    pub standard: FrontendId,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            name: ProgramName::parse("demo").unwrap(),
            version: "0.0.1".to_owned(),
            contract: "demo@1.2.0".parse().unwrap(),
            standard: FrontendId::parse("simple").unwrap(),
        }
    }
}

impl Program for Demo {
    fn name(&self) -> &ProgramName {
        &self.name
    }
    fn version(&self) -> &str {
        &self.version
    }
    fn app_id(&self) -> &'static str {
        "net.freesynergy.Demo"
    }
    fn contract(&self) -> &InterfaceVersion {
        &self.contract
    }
    fn standard_frontend(&self) -> &FrontendId {
        &self.standard
    }
    fn serve(&self, words: &[OsString]) -> Result<(), Failure> {
        if words.iter().any(|w| w == "--fail") {
            return Err(Failure {
                exit: Exit::USAGE,
                code: Coded::new(DemoCode::X).erase(),
            });
        }
        Ok(())
    }
}

pub fn os(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

pub fn fid(s: &str) -> FrontendId {
    FrontendId::parse(s).unwrap()
}

/// The text parameter `key` of a code.
pub fn text<C: ErrorCode>(c: &Coded<C>, key: &str) -> Option<String> {
    c.params().find(|(k, _)| *k == key).and_then(|(_, v)| {
        if let Param::Text(t) = v {
            Some(t.clone())
        } else {
            None
        }
    })
}

/// The text parameter `key` of an erased code.
pub fn code_text(c: &Code, key: &str) -> Option<String> {
    if let Some(Param::Text(t)) = c.param(key) {
        Some(t.clone())
    } else {
        None
    }
}

/// A usable manifest of frontend `id` for the demo program.
pub fn manifest(id: &str) -> FrontendManifest {
    FrontendManifest {
        id: fid(id),
        extends: "net.freesynergy.Demo".to_owned(),
        exec: format!("fs-demo-{id}"),
        requires: vec!["demo@^1.0.0".parse().unwrap()],
        min_program: None,
        default: false,
    }
}

pub fn found(id: &str, trust: Trust) -> Found {
    let place = if trust == Trust::Packaged {
        Place::System
    } else {
        Place::User
    };
    Found {
        manifest: manifest(id),
        place,
        trust,
        file: PathBuf::from(format!("/share/{id}.metainfo.xml")),
    }
}

mod fakes;
mod world;

#[allow(unused_imports)]
pub use fakes::*;
#[allow(unused_imports)]
pub use world::*;

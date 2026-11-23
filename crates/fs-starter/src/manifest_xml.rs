//! The standard manifest source: AppStream metainfo files in the two XDG levels (ADR 0094 items 2-3), read with roxmltree
//! (already used for the catalog: fs-packages `fs-packages-catalog/src/catalog.rs:126`).
//!
//! Pattern: Repository (filesystem) + Parser (one function per file; every refusal is a code with the field, never skipped
//! silently). The manifests exist today (activities, forms, ... `packaging/frontends/simple.metainfo.xml`), nothing reads them.

use std::path::{Path, PathBuf};

use fs_types::coded::Coded;

use crate::codes::{StarterCode, keys};
use crate::frontend::{Found, FrontendManifest, Listing, ManifestSource, Place, Trust};
use crate::names::{ContractReq, FrontendId, ProgramName, Version3};
use crate::system::SystemPlaces;

fn refuse(file: &Path, field: &str) -> Coded<StarterCode> {
    Coded::new(StarterCode::ManifestInvalid)
        .with_text(keys::FILE, file.to_string_lossy())
        .with_text(keys::FIELD, field)
}

/// Reads one manifest. `file` is its path: its name must be `<fs:frontend-id>.metainfo.xml`.
///
/// # Errors
/// `manifest-invalid` with `file` and `field` (`xml`, `component`, `fs:kind`, `fs:frontend-id`, `file-name`, `extends`,
/// `fs:exec`, `fs:requires`, `requires`).
pub fn read_manifest(xml: &str, file: &Path) -> Result<FrontendManifest, Coded<StarterCode>> {
    let doc = roxmltree::Document::parse(xml).map_err(|_| refuse(file, "xml"))?;
    let root = doc.root_element();
    if root.tag_name().name() != "component" || root.attribute("type") != Some("addon") {
        return Err(refuse(file, "component"));
    }
    let child = |tag: &str| {
        root.children()
            .find(|n| n.is_element() && n.tag_name().name() == tag)
    };
    let custom: Vec<(&str, &str)> = child("custom")
        .map(|c| {
            c.children()
                .filter(|n| n.is_element() && n.tag_name().name() == "value")
                .filter_map(|n| Some((n.attribute("key")?, n.text().unwrap_or("").trim())))
                .collect()
        })
        .unwrap_or_default();
    let value = |key: &str| custom.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
    if value("fs:kind") != Some("frontend") {
        return Err(refuse(file, "fs:kind"));
    }
    let id = value("fs:frontend-id")
        .and_then(FrontendId::parse)
        .ok_or_else(|| refuse(file, "fs:frontend-id"))?;
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if name != format!("{id}.metainfo.xml") {
        return Err(refuse(file, "file-name"));
    }
    let extends = child("extends")
        .and_then(|n| n.text())
        .map(str::trim)
        .filter(|t| !t.is_empty());
    let extends = extends.ok_or_else(|| refuse(file, "extends"))?.to_owned();
    let exec =
        value("fs:exec").filter(|e| !e.is_empty() && !e.contains(['/', '\\', ' ']) && e.is_ascii());
    let exec = exec.ok_or_else(|| refuse(file, "fs:exec"))?.to_owned();
    let mut requires = Vec::new();
    for (_, v) in custom.iter().filter(|(k, _)| *k == "fs:requires") {
        requires.push(ContractReq::parse(v).ok_or_else(|| refuse(file, "fs:requires"))?);
    }
    let min_program = match child("requires") {
        None => None,
        Some(r) => {
            let own = r
                .children()
                .find(|n| n.is_element() && n.text().map(str::trim) == Some(extends.as_str()));
            match own {
                None => None,
                Some(n) if n.attribute("compare").unwrap_or("ge") == "ge" => Some(
                    n.attribute("version")
                        .and_then(Version3::parse)
                        .ok_or_else(|| refuse(file, "requires"))?,
                ),
                Some(_) => return Err(refuse(file, "requires")),
            }
        }
    };
    let default = match value("fs:default") {
        None | Some("false") => false,
        Some("true") => true,
        Some(_) => return Err(refuse(file, "fs:default")),
    };
    Ok(FrontendManifest {
        id,
        extends,
        exec,
        requires,
        min_program,
        default,
    })
}

/// Reads at most `max` bytes of a file; more is `size`, unreadable or not text is `read`.
fn read_limited(file: &Path, max: u64) -> Result<String, Coded<StarterCode>> {
    use std::io::Read;
    let f = std::fs::File::open(file).map_err(|_| refuse(file, "read"))?;
    let mut text = String::new();
    f.take(max.saturating_add(1))
        .read_to_string(&mut text)
        .map_err(|_| refuse(file, "read"))?;
    if text.len() as u64 > max {
        return Err(refuse(file, "size"));
    }
    Ok(text)
}

/// The two levels, user before system (ADR 0094 item 3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XdgManifests {
    /// `$XDG_DATA_HOME/freesynergy/frontends` (or `~/.local/share/...`); `None` without a home.
    pub user: Option<PathBuf>,
    /// Each `<XDG_DATA_DIRS>/freesynergy/frontends` (default `/usr/local/share`, `/usr/share`), in order.
    pub system: Vec<PathBuf>,
    /// The most bytes read from one manifest file; a bigger one is refused as `manifest-invalid{field=size}`.
    pub max_bytes: u64,
    /// What counts as the package manager's (see [`SystemPlaces`]): a file of the system level gets [`Trust::Packaged`] only if it
    /// passes this check. `XDG_DATA_DIRS` is set by the user and proves nothing alone.
    pub places: SystemPlaces,
}

/// The default of [`XdgManifests::max_bytes`]: 64 KiB.
pub const MAX_MANIFEST_BYTES: u64 = 64 * 1024;

impl XdgManifests {
    /// The levels from the environment (XDG Base Directory Specification).
    #[must_use]
    pub fn from_env() -> Self {
        let var = |name: &str| std::env::var_os(name);
        Self::from_vars(var("XDG_DATA_HOME"), var("HOME"), var("XDG_DATA_DIRS"))
    }

    /// The levels from the values of `XDG_DATA_HOME`, `HOME` and `XDG_DATA_DIRS`. A relative path is ignored (the
    /// specification says so for both): `XDG_DATA_HOME` then falls back to `HOME`.
    #[must_use]
    pub fn from_vars(
        data_home: Option<std::ffi::OsString>,
        home: Option<std::ffi::OsString>,
        data_dirs: Option<std::ffi::OsString>,
    ) -> Self {
        let sub = |base: PathBuf| base.join("freesynergy").join("frontends");
        let user = data_home
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| home.map(|h| PathBuf::from(h).join(".local").join("share")))
            .map(sub);
        let dirs = data_dirs.filter(|v| !v.is_empty());
        let dirs = dirs.unwrap_or_else(|| "/usr/local/share:/usr/share".into());
        let system = std::env::split_paths(&dirs)
            .filter(|p| p.is_absolute())
            .map(sub)
            .collect();
        Self {
            user,
            system,
            max_bytes: MAX_MANIFEST_BYTES,
            places: SystemPlaces::standard(),
        }
    }

    fn read_level(&self, dir: &Path, place: Place, out: &mut Listing) {
        let max_bytes = self.max_bytes;
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        }; // a missing level is no failure
        let mut files: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
        files.retain(|p| p.to_string_lossy().ends_with(".metainfo.xml"));
        files.sort();
        for file in files {
            let manifest =
                read_limited(&file, max_bytes).and_then(|xml| read_manifest(&xml, &file));
            match manifest {
                Ok(m) if out.get(&m.id).is_some() => {} // the user level has it already
                Ok(manifest) => {
                    // Until fs-packages leaves a receipt, the user level cannot prove a package: stricter, never laxer (O4).
                    // The system level too only after the check: below a system place, root's, no link, nobody else may write.
                    // Else it stays shown as "local, not checked" (the place name alone is user input: `XDG_DATA_DIRS`).
                    let trust = if place == Place::System && self.places.check(&file).is_ok() {
                        Trust::Packaged
                    } else {
                        Trust::LocalUnchecked
                    };
                    out.found.push(Found {
                        manifest,
                        place,
                        trust,
                        file,
                    });
                }
                Err(code) => out.refused.push(code),
            }
        }
    }
}

impl ManifestSource for XdgManifests {
    fn frontends(&self, program: &ProgramName) -> Listing {
        let mut out = Listing::default();
        if let Some(user) = &self.user {
            self.read_level(&user.join(program.as_str()), Place::User, &mut out);
        }
        for system in &self.system {
            self.read_level(&system.join(program.as_str()), Place::System, &mut out);
        }
        out
    }
}

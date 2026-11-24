//! The answer of `invokeIf` and `revert` as plain data: where a change came from ([`Change`], foreign data, clean and cut) and what the
//! module said ([`Reply`]). It leaves the thread of the connection, so it holds no capnp reader.
//!
//! Pattern: Value Object (what the wire said, copied once) + Adapter (capnp readers in, plain values out).

use fs_interfaces::common_capnp;
use fs_interfaces::formsource_capnp::{change_ref, conflict, write_result};

use super::answer::clean;

/// Where a change came from, as the module says it (foreign data: clean, cut).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Change {
    pub activity: u128,
    pub program: String,
    pub instance: String,
    pub person: String,
}

impl Change {
    /// The activity as text (8-4-4-4-12).
    #[must_use]
    pub(super) fn id(&self) -> String {
        let h = format!("{:032x}", self.activity);
        format!(
            "{}-{}-{}-{}-{}",
            &h[..8],
            &h[8..12],
            &h[12..16],
            &h[16..20],
            &h[20..]
        )
    }
}

fn text(t: capnp::Result<capnp::text::Reader<'_>>) -> capnp::Result<String> {
    Ok(clean(t?.to_str()?, 120))
}

/// A `ChangeRef` of the wire as a [`Change`].
pub(super) fn change_of(c: change_ref::Reader<'_>) -> capnp::Result<Change> {
    let u = c.get_activity()?;
    let origin = c.get_origin()?;
    Ok(Change {
        activity: (u128::from(u.get_hi()) << 64) | u128::from(u.get_lo()),
        program: text(origin.get_program())?,
        instance: text(origin.get_instance())?,
        person: text(origin.get_person())?,
    })
}

/// The answer of `invokeIf` or `revert`, as plain data (it leaves the thread of the connection).
#[derive(Debug, Default)]
pub(super) struct Reply {
    pub failed: bool,
    pub kind: String,
    pub title: String,
    pub detail: String,
    pub status: u16,
    pub record: String,
    pub text: String,
    pub change: Option<Change>,
    pub by: Option<Change>,
}

pub(super) fn read_reply(
    failed: bool,
    problem: capnp::Result<common_capnp::problem::Reader<'_>>,
    result: capnp::Result<write_result::Reader<'_>>,
    clash: capnp::Result<conflict::Reader<'_>>,
) -> capnp::Result<Reply> {
    let mut r = Reply {
        failed,
        ..Reply::default()
    };
    if failed {
        let p = problem?;
        p.get_type()?.to_str()?.clone_into(&mut r.kind);
        p.get_title()?.to_str()?.clone_into(&mut r.title);
        p.get_detail()?.to_str()?.clone_into(&mut r.detail);
        r.status = p.get_status();
        if let Ok(by) = clash
            .and_then(fs_interfaces::formsource_capnp::conflict::Reader::get_by)
            .and_then(change_of)
        {
            r.by = (by.activity != 0).then_some(by);
        }
        return Ok(r);
    }
    let w = result?;
    r.record = text(w.get_record())?;
    r.text = text(w.get_text())?;
    r.change = Some(change_of(w.get_change()?)?).filter(|c| c.activity != 0);
    Ok(r)
}

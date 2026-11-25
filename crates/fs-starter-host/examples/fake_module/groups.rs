//! The made-up groups data of the fake module: a tree of two depths and a member list, made by `fs-forms-testdata` from the wish
//! `tests/data/groups-parity.toml` (a copy of the fixture of fs-forms-run) and served by its `SampleModule`.
//!
//! Pattern: Fixture (the same wish gives the same data on every platform) behind the Adapter of `fake_module.rs`.

use fs_forms_testdata::{Relation, SampleModule, SampleSet, SourceSpec};
use fs_interfaces::module::{Actor, Caller as ModuleCaller, Context, ModuleCore, PageQuery};
use fs_types::{BasicValue, PersonArt, PersonRef, Reference};

/// The made-up groups: the set (declarations, slices) and the module that serves it.
pub struct Groups {
    pub set: SampleSet,
    pub module: SampleModule,
}

/// The wish of the groups data (a tree of two depths and a member list), the same file the tests of fs-forms-run use.
const WISH: &str = include_str!("../../tests/data/groups-parity.toml");
const PERSON: &str = "urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e01";

impl Groups {
    pub fn new() -> Self {
        let mut source = SourceSpec::from_wish_text(WISH).expect("the wish reads");
        // the tree needs its parent relation (the catalog of the pinned fs-interfaces does not carry it yet; once it does, not twice)
        let parent = Relation::Parent {
            child: "parent".to_owned(),
        };
        if !source.relations.contains(&parent) {
            source.relations.push(parent);
        }
        let set = source.build().expect("the wish builds");
        let module = SampleModule::new(&set).expect("the module");
        Self { set, module }
    }

    pub fn context() -> Context {
        let person = PersonRef {
            id: Reference::parse(PERSON).expect("an absolute reference"),
            art: PersonArt::Person,
            abzug: None,
        };
        let caller = ModuleCaller::Connection {
            system_id: "1000".into(),
            system_name: None,
        };
        Context::from_connection(caller, Actor::Person, person)
    }

    /// The records of the form `name`: for each the id and its values as (field, channel, text). The module says the CHANNEL of a value;
    /// the wire carries the field it is made of (the map of the declaration). `None`: no such form.
    pub async fn read(&self, name: &str) -> Option<Result<Vec<Row>, String>> {
        let (slice, declared) = self
            .set
            .slices
            .iter()
            .zip(&self.set.declared)
            .find(|(_, d)| d.form.name == name)?;
        let mut query = PageQuery::default();
        query.form.clone_from(&slice.form);
        query.limit = 500;
        let made = match self.module.page(&Self::context(), query).await {
            Ok(page) => page,
            Err(refusal) => return Some(Err(refusal.detail)),
        };
        Some(Ok(made
            .records
            .iter()
            .map(|r| {
                let values = r
                    .values
                    .iter()
                    .map(|v| {
                        let field = declared
                            .map
                            .iter()
                            .find(|(c, _)| *c == v.path)
                            .and_then(|(_, f)| f.first())
                            .cloned()
                            .unwrap_or_default();
                        (field, v.path.clone(), v.text.clone())
                    })
                    .collect();
                (r.id.clone(), values)
            })
            .collect()))
    }
}

/// One record: its id and its values (field, channel, text).
pub type Row = (String, Vec<(String, String, String)>);

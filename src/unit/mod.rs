use std::{collections::BTreeMap, fmt::Display, path::PathBuf};

mod activation;
mod parser;

#[derive(Debug)]
pub struct UnitName {
    unit_file: PathBuf,
    name: String,
    template: String,
}

#[derive(Debug)]
pub enum UnitType {
    Service,
    Mount,
    Swap,
    Socket,
    Target,
    Device,
    Automount,
    Timer,
    Path,
    Slice,
    Scope,
}

#[derive(Debug)]
pub struct UnitInfo {
    unit_type: UnitType,
    unit: Section,
    install: Section,
    section: Option<Section>,
}

#[derive(Debug)]
pub struct Unit {
    name: UnitName,
    info: UnitInfo,
    requires: Vec<String>,
    wants: Vec<String>,
    after: Vec<String>,
    before: Vec<String>,
}

pub type Section = BTreeMap<String, Vec<String>>;

pub use activation::walk;

impl Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("  Unit: ")?;
        f.write_str(&self.name.name)?;
        f.write_str(" REQUIRES ")?;
        for dep in self.requires.clone() {
            f.write_str(dep.as_str())?;
        }
        f.write_str(" WANTS ")?;
        for dep in self.wants.clone() {
            f.write_str(dep.as_str())?;
        }
        f.write_str(" AFTER ")?;
        for dep in self.after.clone() {
            f.write_str(dep.as_str())?;
        }
        f.write_str(" BEFORE ")?;
        for dep in self.before.clone() {
            f.write_str(dep.as_str())?;
        }
        f.write_str("\n")?;
        Ok(())
    }
}

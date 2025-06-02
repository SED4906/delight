use std::{collections::BTreeMap, path::PathBuf};

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
}

pub type Section = BTreeMap<String, Vec<String>>;

pub use activation::walk;

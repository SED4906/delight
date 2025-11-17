mod parser;
mod load;
mod traverse;

const SYSTEM_UNIT_PATHS: &[&str] = &[
    "/etc/systemd/system/",
"/usr/local/lib/systemd/system/",
"/usr/lib/systemd/system/",
];

use std::sync::Arc;

pub use load::load_unit;
pub use traverse::traverse_unit;

#[derive(Clone,Debug)]
pub struct Unit {
    // Section
    section: Section,
    // [Unit]
    requires: Vec<String>,
    wants: Vec<String>,
    after: Vec<String>,
    before: Vec<String>,
    // [Install]
    alias: Vec<String>,
    wanted_by: Vec<String>,
    required_by: Vec<String>,
}

#[derive(Clone,Debug)]
pub enum Section {
    Service {
        exec: Exec,
        exec_start: Vec<String>,
        exec_stop: Vec<String>,
    },
    Mount {
        exec: Exec,
        what: String,
        r#where: String,
        r#type: String,
        options: Vec<String>,
    },
    Swap {
        exec: Exec,
        what: String,
        options: Vec<String>,
    },
    Socket {
        exec: Exec,
        service: Option<String>,
    },
    Target,
    Device,
    Automount {
        r#where: String,
        extra_options: Vec<String>,
    },
    Timer {
        unit: Option<String>,
    },
    Path {
        unit: Option<String>,
    },
    Slice,
    Scope,
}

#[derive(Clone,Debug)]
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

#[derive(Clone,Debug)]
pub struct Exec {
    exec_search_path: Option<Vec<String>>,
    working_directory: Option<String>,
    user: Option<String>,
    group: Option<String>,
}

#[derive(Clone,Debug)]
pub struct Job {
    unit: Arc<Unit>,
    template: String,
}

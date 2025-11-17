use std::{collections::BTreeMap, fs, path::PathBuf};

use crate::{SYSTEM_UNIT_PATHS, Unit, load_unit};

pub fn traverse_unit(units: &mut BTreeMap<String, Unit>, name: &str) {
    let unit = units[name].clone();
    for required in &unit.requires {
        match crate::load_unit(units, required) {
            Some(true) => {
                traverse_unit(units, required);
            }
            _ => continue,
        }
    }
    for wanted in &unit.wants {
        match crate::load_unit(units, wanted) {
            Some(true) => {
                traverse_unit(units, wanted);
            }
            _ => continue,
        }
    }
    traverse_extra_directories(units, name);
}

fn traverse_extra_directories(units: &mut BTreeMap<String, Unit>, name: &str) {
    for system_unit_path in SYSTEM_UNIT_PATHS {
        let mut wants_path = PathBuf::new();
        wants_path.push(system_unit_path);
        let mut name_wants = name.to_string();
        name_wants.push_str(".wants");
        wants_path.push(&name_wants);
        if let Ok(in_dir) = fs::read_dir(wants_path) {
            for file in in_dir {
                if let Ok(file) = file {
                    if let Ok(file) = fs::read_link(file.path()) {
                        if let Some(file_name) = file.file_name() {
                            if let Some(file_name) = file_name.to_str() {
                                load_unit(units, file_name);
                            }
                        }
                    }
                }
            }
        }
    }
}

use std::{fs, path::PathBuf};

use crate::{SYSTEM_UNIT_PATHS, load_unit, state::UNITS};

pub fn traverse_unit(name: &str) {
    let unit = UNITS.lock().unwrap()[name].clone();
    for required in &unit.requires {
        match crate::load_unit(required) {
            Some(true) => {
                traverse_unit(required);
            }
            _ => continue,
        }
    }
    for wanted in &unit.wants {
        match crate::load_unit(wanted) {
            Some(true) => {
                traverse_unit(wanted);
            }
            _ => continue,
        }
    }
    traverse_unit_extra_wants(name);
}

fn traverse_unit_extra_wants(name: &str) {
    for system_unit_path in SYSTEM_UNIT_PATHS {
        let mut wants_path = PathBuf::new();
        wants_path.push(system_unit_path);
        let mut name_wants = name.to_string();
        name_wants.push_str(".wants");
        wants_path.push(&name_wants);
        let Ok(dir) = fs::read_dir(wants_path) else {
            continue;
        };
        for file in dir {
            if let Ok(file) = file
                && let Ok(file) = fs::read_link(file.path())
                && let Some(file_name) = file.file_name()
                && let Some(file_name) = file_name.to_str()
            {
                load_unit(file_name);
                UNITS.lock().unwrap().get_mut(name).unwrap().wants.push(file_name.to_string());
            }
        }
    }
}

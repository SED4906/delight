use std::{collections::BTreeMap, fs, path::PathBuf};

use crate::{Unit, UnitType, SYSTEM_UNIT_PATHS};

pub fn load_unit(units: &mut BTreeMap<String, Unit>, name: &str) -> Option<bool> {
    if !units.contains_key(name) {
        units.insert(
            name.into(),
            Unit::new(&load_unit_ini(name)?, unit_type_from_name(name)?)?,
        );
        Some(true)
    } else {
        Some(false)
    }
}

fn unit_type_from_name(name: &str) -> Option<UnitType> {
    Some(match name.rsplit_once(".")?.1 {
        "service" => UnitType::Service,
        "mount" => UnitType::Mount,
        "swap" => UnitType::Swap,
        "socket" => UnitType::Socket,
        "target" => UnitType::Target,
        "device" => UnitType::Device,
        "automount" => UnitType::Automount,
        "timer" => UnitType::Timer,
        "path" => UnitType::Path,
        "slice" => UnitType::Slice,
        "scope" => UnitType::Scope,
        _ => return None,
    })
}

fn load_unit_ini(name: &str) -> Option<String> {
    let mut result = None;
    for system_unit_path in SYSTEM_UNIT_PATHS {
        let mut unit_path = PathBuf::new();
        unit_path.push(system_unit_path);
        unit_path.push(name);
        if let Ok(input) = fs::read_to_string(unit_path) {
            result = Some(input);
            break;
        }
    }
    let _ = result.as_ref()?;
    for system_unit_path in SYSTEM_UNIT_PATHS {
        let mut override_path = PathBuf::new();
        override_path.push(system_unit_path);
        let mut name_d = name.to_string();
        name_d.push_str(".d");
        override_path.push(&name_d);
        if let Ok(in_dir) = fs::read_dir(override_path) {
            for file in in_dir {
                if let Ok(file) = file {
                    if let Ok(input) = fs::read_to_string(file.path()) {
                        result.as_mut()?.push('\n');
                        result.as_mut()?.push_str(&input);
                    }
                }
            }
        }
    }
    result
}

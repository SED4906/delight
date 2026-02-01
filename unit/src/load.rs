use std::{collections::BTreeMap, fs, path::PathBuf};

use crate::{SYSTEM_UNIT_PATHS, Unit, UnitType};

pub fn load_unit(units: &mut BTreeMap<String, Unit>, name: &str) -> Option<bool> {
    if !units.contains_key(name) {
        units.insert(
            name.into(),
            Unit::new(&stitch_unit(name)?, name.try_into().ok()?)?,
        );
        Some(true)
    } else {
        Some(false)
    }
}

impl TryFrom<&str> for UnitType {
    type Error = ();

    fn try_from(name: &str) -> Result<Self, ()> {
        Ok(match name.rsplit_once(".").ok_or(())?.1 {
            "service" => Self::Service,
            "mount" => Self::Mount,
            "swap" => Self::Swap,
            "socket" => Self::Socket,
            "target" => Self::Target,
            "device" => Self::Device,
            "automount" => Self::Automount,
            "timer" => Self::Timer,
            "path" => Self::Path,
            "slice" => Self::Slice,
            "scope" => Self::Scope,
            _ => return Err(()),
        })
    }
}

fn stitch_unit(name: &str) -> Option<String> {
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
    let _ = result.as_ref()?; // return early if None
    for system_unit_path in SYSTEM_UNIT_PATHS {
        // drop-in files for all units of a given type
        let mut override_path = PathBuf::new();
        override_path.push(system_unit_path);
        let mut unit_d = name.rsplit_once(".")?.1.to_string();
        unit_d.push_str(".d");
        override_path.push(&unit_d);
        let Ok(dir) = fs::read_dir(override_path) else {
            continue;
        };
        for file in dir {
            if let Ok(file) = file
                && let Ok(input) = fs::read_to_string(file.path())
                {
                    result.as_mut()?.push('\n');
                    result.as_mut()?.push_str(&input);
                }
        }
    }
    for system_unit_path in SYSTEM_UNIT_PATHS {
        // drop-in files for this unit
        let mut override_path = PathBuf::new();
        override_path.push(system_unit_path);
        let mut name_d = name.to_string();
        name_d.push_str(".d");
        override_path.push(&name_d);
        let Ok(dir) = fs::read_dir(override_path) else {
            continue;
        };
        for file in dir {
            if let Ok(file) = file
                && let Ok(input) = fs::read_to_string(file.path())
            {
                result.as_mut()?.push('\n');
                result.as_mut()?.push_str(&input);
            }
        }
    }
    result
}

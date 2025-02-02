use std::{collections::BTreeMap, path::PathBuf, process};

enum UnitSuffix {
    Target,
    Service,
}

const UNIT_PATHS: &[&str] = &[
    "/usr/lib/systemd/system/",
];

fn read_unit(name: &str) -> Result<String,()> {
    for unit_path in UNIT_PATHS {
        let mut pathbuf = PathBuf::new();
        pathbuf.push(unit_path);
        pathbuf.push(name);
        match std::fs::read_to_string(pathbuf) {
            Ok(result) => return Ok(result),
            Err(_) => {}
        }
    }
    Err(())
}

fn parse_unit(unit_text: String) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    for line in unit_text.lines() {
        if let Some((key, value)) = line.split_once("=") {
            if result.contains_key(key.trim()) {
                if value.trim().is_empty() {
                    result.remove(key.trim());
                } else {
                    result.insert(key.trim().to_owned(), format!("{}\n{}", result[key.trim()], value.trim()).to_owned());
                }
            } else {
                result.insert(key.trim().to_owned(), value.trim().to_owned());
            }
        }
    }
    return result;
}

fn get_unit_suffix(name: &str) -> Result<UnitSuffix,()> {
    if name.ends_with(".target") {
        Ok(UnitSuffix::Target)
    } else if name.ends_with(".service") {
        Ok(UnitSuffix::Service)
    } else {
        Err(())
    }
}

pub fn load_unit(name: &str) -> Result<(), ()> {
    println!("Loading unit {name}");
    let unit_text = read_unit(name)?;
    let keyvalues = parse_unit(unit_text);
    let suffix = get_unit_suffix(name)?;
    if keyvalues.contains_key("Wants") {
        for wants_unit in keyvalues["Wants"].split_whitespace() {
            let _ = load_unit(wants_unit);
        }
    }
    match suffix {
        UnitSuffix::Service => {
            if keyvalues.contains_key("ExecStart") {
                for exec_start in keyvalues["ExecStart"].lines() {
                    println!("Trying process {exec_start}");
                    process::Command::new(exec_start).spawn().or(Err(()))?;
                    println!("Started process {exec_start}");
                }
            }
        }
        _ => {}
    }
    Ok(())
}

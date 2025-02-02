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

pub fn load_units_wanted_by(name: &str) -> Result<(), ()> {
    for unit_path in UNIT_PATHS {
        let mut pathbuf = PathBuf::new();
        pathbuf.push(unit_path);
        let mut wants_dir = name.to_owned();
        wants_dir.push_str(".wants");
        pathbuf.push(wants_dir);
        match std::fs::read_dir(pathbuf) {
            Ok(result) => {
                for entry in result {
                    if let Ok(entry) = entry {
                        if let Some(wants_name) = entry.file_name().to_str() {
                            let _ = load_unit(wants_name);
                        }
                    }
                }
            }
            Err(_) => {}
        }
    }
    Ok(())
}

pub fn load_unit(name: &str) -> Result<(), ()> {
    println!("Loading unit {name}");
    let unit_text = read_unit(name)?;
    let keyvalues = parse_unit(unit_text);
    let suffix = get_unit_suffix(name)?;
    if keyvalues.contains_key("Requires") {
        for wants_unit in keyvalues["Requires"].split_whitespace() {
            load_unit(wants_unit)?;
        }
    }
    if keyvalues.contains_key("Wants") {
        for wants_unit in keyvalues["Wants"].split_whitespace() {
            let _ = load_unit(wants_unit);
        }
    }
    match suffix {
        UnitSuffix::Target => {
            let _ = load_units_wanted_by(name);
        }
        UnitSuffix::Service => {
            if keyvalues.contains_key("ExecStart") {
                for exec_start in keyvalues["ExecStart"].lines() {
                    println!("Trying process {exec_start}");
                    let cmd = exec_start.split_whitespace().next();
                    if let Some(cmd) = cmd {
                        process::Command::new(cmd).args(exec_start.split_whitespace().skip(1).collect::<Vec<&str>>()).spawn().or(Err(()))?;
                        println!("Started process {exec_start}");
                    }
                }
            }
        }
    }
    Ok(())
}

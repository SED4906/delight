use std::{collections::BTreeMap, os::unix::net::UnixListener, path::PathBuf, process};

enum UnitSuffix {
    Target,
    Service,
    Mount,
    Socket,
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
                    result.insert(key.trim().to_owned(), format!("{}\n{}", result[key.trim()], value.trim().replace("%%", "%")).to_owned());
                }
            } else {
                result.insert(key.trim().to_owned(), value.trim().replace("%%", "%").to_owned());
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
    } else if name.ends_with(".mount") {
        Ok(UnitSuffix::Mount)
    } else if name.ends_with(".socket") {
        Ok(UnitSuffix::Socket)
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

pub fn load_exec_start(keyvalues: BTreeMap<String, String>) -> Result<(), ()> {
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
    Ok(())
}

pub fn load_mount_unit(keyvalues: BTreeMap<String, String>) -> Result<(), ()> {
    if keyvalues.contains_key("What") && keyvalues.contains_key("Where") {
        println!("Mounting {} to {}", keyvalues["What"], keyvalues["Where"]);
        let mount_type = keyvalues.get("Type").unwrap_or(&"auto".to_owned()).clone();
        if let Some(options) = keyvalues.get("Options") {
            process::Command::new("mount").args(&["-t",mount_type.as_str(),"-o",options,keyvalues["What"].clone().as_str(), keyvalues["Where"].clone().as_str()]).spawn().or(Err(()))?;
        } else {
            process::Command::new("mount").args(&["-t",mount_type.as_str(),keyvalues["What"].clone().as_str(), keyvalues["Where"].clone().as_str()]).spawn().or(Err(()))?;
        }
    }
    Ok(())
}

pub fn load_socket_unit(keyvalues: BTreeMap<String, String>) -> Result<(), ()> {
    if keyvalues.contains_key("ListenStream") {
        if keyvalues["ListenStream"].starts_with("/") {
            UnixListener::bind(keyvalues["ListenStream"].clone().as_str()).or(Err(()))?;
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
        for requires_unit in keyvalues["Requires"].split_whitespace() {
            load_unit(requires_unit)?;
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
            load_exec_start(keyvalues)?;
        }
        UnitSuffix::Mount => {
            load_mount_unit(keyvalues)?;
        }
        UnitSuffix::Socket => {
            load_socket_unit(keyvalues)?;
        }
    }
    Ok(())
}

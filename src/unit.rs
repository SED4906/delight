use std::collections::BTreeSet;
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::{collections::BTreeMap, os::unix::net::UnixListener, path::PathBuf, process};

enum UnitSuffix {
    Target,
    Service,
    Mount,
    Socket,
}

const UNIT_PATHS: &[&str] = &["/usr/lib/systemd/system/"];

fn read_unit(name: &str) -> Result<String, ()> {
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
    let mut working_lines = vec![];
    let mut working_line = String::new();
    let mut line_continues;
    for line in unit_text.lines() {
        if line.ends_with("\\") {
            line_continues = true;
            let line = line.strip_suffix("\\").unwrap_or(line);
            working_line.push_str(line);
            working_line.push(' ');
        } else if line.starts_with("#") || line.starts_with(";") {
            line_continues = true;
        } else {
            working_line.push_str(line);
            line_continues = false;
        }
        if !line_continues {
            working_lines.push(working_line);
            working_line = String::new();
        }
    }
    for line in working_lines.iter() {
        if let Some((key, value)) = line.split_once("=") {
            if result.contains_key(key.trim()) {
                if value.trim().is_empty() {
                    result.remove(key.trim());
                } else {
                    result.insert(
                        key.trim().to_owned(),
                        format!(
                            "{}\n{}",
                            result[key.trim()],
                            value.trim().replace("%%", "%")
                        )
                        .to_owned(),
                    );
                }
            } else {
                result.insert(
                    key.trim().to_owned(),
                    value.trim().replace("%%", "%").to_owned(),
                );
            }
        }
    }
    return result;
}

fn get_unit_suffix(name: &str) -> Result<UnitSuffix, ()> {
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

pub fn load_units_wanted_by(name: &str, active_units: &mut BTreeSet<String>) -> Result<(), ()> {
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
                            let _ = load_unit(wants_name, active_units, false);
                        }
                    }
                }
            }
            Err(_) => {}
        }
    }
    Ok(())
}

pub fn load_service_unit(keyvalues: BTreeMap<String, String>) -> Result<(), ()> {
    if keyvalues.contains_key("ExecStart") {
        for exec_start in keyvalues["ExecStart"].lines() {
            println!("Trying process {exec_start}");
            let cmd = exec_start.split_whitespace().next();
            if let Some(cmd) = cmd {
                process::Command::new(cmd)
                    .args(exec_start.split_whitespace().skip(1).collect::<Vec<&str>>())
                    .spawn()
                    .or(Err(()))?;
                println!("Started process {exec_start}");
            }
        }
    }
    Ok(())
}

pub fn load_service_unit_with_socket(
    keyvalues: BTreeMap<String, String>,
    fd: UnixListener,
) -> Result<(), ()> {
    if keyvalues.contains_key("ExecStart") {
        if let Some(exec_start) = keyvalues["ExecStart"].lines().next() {
            println!("Trying process {exec_start}");
            let cmd = exec_start.split_whitespace().next();
            if let Some(cmd) = cmd {
                unsafe {
                    process::Command::new(cmd)
                    .pre_exec(move || {
                        std::env::set_var("LISTEN_PID", process::id().to_string());
                        std::env::set_var("LISTEN_FDS", fd.as_raw_fd().to_string());
                        Ok(())
                    })
                    .args(exec_start.split_whitespace().skip(1).collect::<Vec<&str>>())
                    .spawn()
                    .or(Err(()))?;
                }
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
            process::Command::new("mount")
                .args(&[
                    "-t",
                    mount_type.as_str(),
                    "-o",
                    options,
                    keyvalues["What"].clone().as_str(),
                    keyvalues["Where"].clone().as_str(),
                ])
                .spawn()
                .or(Err(()))?;
        } else {
            process::Command::new("mount")
                .args(&[
                    "-t",
                    mount_type.as_str(),
                    keyvalues["What"].clone().as_str(),
                    keyvalues["Where"].clone().as_str(),
                ])
                .spawn()
                .or(Err(()))?;
        }
    }
    Ok(())
}

pub fn load_socket_unit(keyvalues: BTreeMap<String, String>) -> Result<UnixListener, ()> {
    if keyvalues.contains_key("ListenStream") {
        if keyvalues["ListenStream"].starts_with("/") {
            return Ok(UnixListener::bind(keyvalues["ListenStream"].clone().as_str()).or(Err(()))?);
        }
    }
    Err(())
}

pub enum UnitLoadInfo {
    Other,
    AlreadyActive,
    Socket { fd: UnixListener },
}

pub fn load_unit(name: &str, active_units: &mut BTreeSet<String>, is_sidecar_unit: bool) -> Result<UnitLoadInfo, ()> {
    if active_units.contains(name) {
        return Ok(UnitLoadInfo::AlreadyActive);
    };
    active_units.insert(name.to_string());
    println!("Loading unit {name}");
    let unit_text = read_unit(name)?;
    let keyvalues = parse_unit(unit_text);
    let suffix = get_unit_suffix(name)?;
    if keyvalues.contains_key("Requires") {
        for requires_unit in keyvalues["Requires"].split_whitespace() {
            load_unit(requires_unit, active_units, false)?;
        }
    }
    if keyvalues.contains_key("Wants") {
        for wants_unit in keyvalues["Wants"].split_whitespace() {
            let _ = load_unit(wants_unit, active_units, false);
        }
    }
    match suffix {
        UnitSuffix::Target => {
            let _ = load_units_wanted_by(name, active_units);
        }
        UnitSuffix::Service => {
            let mut socket_unit_name = name.strip_suffix(".service").unwrap().to_string();
            socket_unit_name.push_str(".socket");
            if let Ok(UnitLoadInfo::Socket { fd }) = load_unit(name, active_units, true) {
                load_service_unit_with_socket(keyvalues, fd)?;
            } else {
                load_service_unit(keyvalues)?;
            }
        }
        UnitSuffix::Mount => {
            load_mount_unit(keyvalues)?;
        }
        UnitSuffix::Socket => {
            let fd = load_socket_unit(keyvalues)?;
            if is_sidecar_unit {
                return Ok(UnitLoadInfo::Socket { fd });
            } else {
                let mut service_unit_name = name.strip_suffix(".socket").unwrap().to_string();
                service_unit_name.push_str(".service");
                active_units.remove(name);
                load_unit(service_unit_name.as_str(), active_units, true)?;
            }
        }
    }
    Ok(UnitLoadInfo::Other)
}

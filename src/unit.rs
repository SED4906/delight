use std::collections::BTreeSet;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process;
use std::{collections::BTreeMap, os::unix::net::UnixListener, path::PathBuf};

enum UnitSuffix {
    Target,
    Service,
    Mount,
    Socket,
}

const UNIT_PATHS: &[&str] = &["/usr/lib/systemd/system/"];

fn read_unit(name: &str) -> Result<String, UnitLoadError> {
    for unit_path in UNIT_PATHS {
        let mut pathbuf = PathBuf::new();
        pathbuf.push(unit_path);
        pathbuf.push(name);
        match std::fs::read_to_string(pathbuf) {
            Ok(result) => return Ok(result),
            Err(_) => {}
        }
    }
    Err(UnitLoadError::DoesNotExist)
}

fn parse_unit(unit_text: String, template: String) -> BTreeMap<String, String> {
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
                            value
                                .trim()
                                .replace("%i", template.as_str())
                                .replace("%I", template.as_str())
                                .replace("%%", "%")
                        )
                        .to_owned(),
                    );
                }
            } else {
                result.insert(
                    key.trim().to_owned(),
                    value
                        .trim()
                        .replace("%i", template.as_str())
                        .replace("%I", template.as_str())
                        .replace("%%", "%")
                        .to_owned(),
                );
            }
        }
    }
    return result;
}

fn get_unit_suffix(name: &str) -> Result<UnitSuffix, UnitLoadError> {
    if name.ends_with(".target") {
        Ok(UnitSuffix::Target)
    } else if name.ends_with(".service") {
        Ok(UnitSuffix::Service)
    } else if name.ends_with(".mount") {
        Ok(UnitSuffix::Mount)
    } else if name.ends_with(".socket") {
        Ok(UnitSuffix::Socket)
    } else {
        Err(UnitLoadError::Failed)
    }
}

pub fn load_units_wanted_by(name: &str) -> Result<Vec<String>, UnitLoadError> {
    let mut unit_order = vec![];
    for unit_path in UNIT_PATHS {
        let mut pathbuf = PathBuf::new();
        pathbuf.push(unit_path);
        let mut wants_dir = name.to_owned();
        wants_dir.push_str(".wants");
        pathbuf.push(wants_dir);
        if let Ok(result) = std::fs::read_dir(pathbuf) {
            for entry in result {
                if let Ok(entry) = entry {
                    if let Some(wants_name) = entry.file_name().to_str() {
                        unit_order.push(wants_name.to_owned());
                    }
                }
            }
        }
    }
    Ok(unit_order)
}

#[derive(Debug)]
pub enum UnitLoadError {
    Failed,
    DoesNotExist,
}

pub struct Unit {
    suffix: UnitSuffix,
    keyvalues: BTreeMap<String,String>
}

pub fn load_unit(name: &str) -> Result<Unit,UnitLoadError> {
    let (file_name, template) = name
    .rsplit_once("@")
    .and_then(|(name, template)| {
        let mut name = name.to_string();
        name.push('@');
        let (template, suffix) = template.rsplit_once(".").unwrap();
        name.push('.');
        name.push_str(suffix);
        Some((name.clone(), template))
    })
    .unwrap_or((name.to_string(), ""));
    let unit_text = read_unit(file_name.as_str())?;
    let keyvalues = parse_unit(unit_text, template.to_string());
    let suffix = get_unit_suffix(name)?;
    Ok(Unit { suffix, keyvalues })
}

pub fn activate_socket_unit(
    unit: Unit,
) -> Result<UnixListener, UnitLoadError> {
    let Unit { keyvalues, .. } = unit;
    if keyvalues.contains_key("ListenStream") && keyvalues["ListenStream"].starts_with("/") {
        if let Some(dir_path) = Path::new(keyvalues["ListenStream"].clone().as_str()).parent() {
            std::fs::create_dir_all(dir_path).or(Err(UnitLoadError::Failed))?;
            return Ok(UnixListener::bind(keyvalues["ListenStream"].clone().as_str()).or(Err(UnitLoadError::Failed))?);
        }
    }
    Err(UnitLoadError::Failed)
}

pub fn activate_service_unit(
    unit: Unit,
) -> Result<(), UnitLoadError> {
    let Unit { keyvalues, .. } = unit;
    if keyvalues.contains_key("ExecStart") {
        for exec_start in keyvalues["ExecStart"].lines() {
            let cmd = exec_start.split_whitespace().next();
            if let Some(cmd) = cmd {
                process::Command::new(cmd.strip_prefix("-").unwrap_or(cmd))
                .args(exec_start.split_whitespace().skip(1).collect::<Vec<&str>>())
                .spawn()
                .or(Err(UnitLoadError::Failed))?;
            }
        }
    }
    Err(UnitLoadError::Failed)
}

pub fn activate_service_unit_with_socket(
    unit: Unit,
    fd: UnixListener
) -> Result<(), UnitLoadError> {
    let Unit { keyvalues, .. } = unit;
    if keyvalues.contains_key("ExecStart") {
        if let Some(exec_start) = keyvalues["ExecStart"].lines().next() {
            let cmd = exec_start.split_whitespace().next();
            if let Some(cmd) = cmd {
                unsafe {
                    process::Command::new(cmd.strip_prefix("-").unwrap_or(cmd))
                    .pre_exec(move || {
                        std::env::set_var("LISTEN_PID", process::id().to_string());
                        std::env::set_var("LISTEN_FDS", fd.as_raw_fd().to_string());
                        Ok(())
                    })
                    .args(exec_start.split_whitespace().skip(1).collect::<Vec<&str>>())
                    .spawn()
                    .or(Err(UnitLoadError::Failed))?;
                }
            }
        }
    }
    Err(UnitLoadError::Failed)
}


pub fn activate_mount_unit(unit: Unit) -> Result<(), UnitLoadError> {
    let Unit { keyvalues, .. } = unit;
    if keyvalues.contains_key("What") && keyvalues.contains_key("Where") {
        let mount_type = keyvalues.get("Type").unwrap_or(&"auto".to_owned()).clone();
        std::fs::create_dir(Path::new(keyvalues["Where"].clone().as_str())).or(Err(UnitLoadError::Failed))?;
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
            .or(Err(UnitLoadError::Failed))?;
        } else {
            process::Command::new("mount")
            .args(&[
                "-t",
                mount_type.as_str(),
                  keyvalues["What"].clone().as_str(),
                  keyvalues["Where"].clone().as_str(),
            ])
            .spawn()
            .or(Err(UnitLoadError::Failed))?;
        }
    }
    Ok(())
}


pub fn activate_unit(
    name: &str,
    checked_units: &mut BTreeSet<String>
) -> Result<(), UnitLoadError> {
    if checked_units.contains(name) {
        return Ok(());
    }
    checked_units.insert(name.to_owned());
    print!("{name} ");
    let _ = std::io::stdout().flush();
    let unit = load_unit(name)?;

    if unit.keyvalues.contains_key("Requires") {
        for requires_unit in unit.keyvalues["Requires"].split_whitespace() {
            activate_unit(requires_unit, checked_units)?;
        }
    }
    if unit.keyvalues.contains_key("Wants") {
        for wants_unit in unit.keyvalues["Wants"].split_whitespace() {
            let _ = activate_unit(wants_unit, checked_units);
        }
    }
    if unit.keyvalues.contains_key("After") {
        for after_unit in unit.keyvalues["After"].split_whitespace() {
            let _ = activate_unit(after_unit, checked_units);
        }
    }
    match unit.suffix {
        UnitSuffix::Target => {
            if let Ok(wanted_by_result) = load_units_wanted_by(name) {
                for wanted_by_unit in wanted_by_result {
                    let _ = activate_unit(wanted_by_unit.as_str(), checked_units);
                }
            }
        }
        UnitSuffix::Service => {
            let mut socket_unit_name = name.strip_suffix(".service").unwrap().to_string();
            socket_unit_name.push_str(".socket");
            match load_unit(socket_unit_name.as_str()) {
                Ok(socket_result) => {
                    checked_units.insert(socket_unit_name);
                    let fd = activate_socket_unit(socket_result)?;
                    activate_service_unit_with_socket(unit, fd)?;
                }
                Err(UnitLoadError::DoesNotExist) => {
                    activate_service_unit(unit)?;
                }
                _ => {return Err(UnitLoadError::Failed)}
            }
        }
        UnitSuffix::Mount => {
            activate_mount_unit(unit)?;
        }
        UnitSuffix::Socket => {
            let mut service_unit_name = name.strip_suffix(".socket").unwrap().to_string();
            service_unit_name.push_str(".service");
            activate_unit(service_unit_name.as_str(), checked_units)?;
        }
    }
    Ok(())
}

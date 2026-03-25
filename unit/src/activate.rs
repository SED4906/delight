use std::{collections::BTreeMap, process::Command};

use crate::Unit;

fn apply_specifiers(input: &str, name: &str, instance: &str) -> String {
    let mut result = input.to_string();
    result = result.replace("%i", instance);
    result = result.replace("%I", instance);
    result = result.replace("%n", name);
    result = result.replace("%N", name.rsplit_once(".").unwrap().0);
    result = result.replace(
        "%p",
        name.rsplit_once("@")
            .unwrap_or_else(|| (name.rsplit_once(".").unwrap().0, ""))
            .0,
    );
    result = result.replace(
        "%P",
        name.rsplit_once("@")
            .unwrap_or_else(|| (name.rsplit_once(".").unwrap().0, ""))
            .0,
    );
    result = result.replace("%%", "%");
    result
}

pub fn activate_unit(units: &BTreeMap<String, Unit>, name: &str, instance: &str) -> Option<()> {
    println!("activate: {name}\t{instance}");
    let unit = units.get(name)?;
    for require in &unit.requires {
        activate_unit(units, &require, "")?;
    }
    println!("requires: {name}\t{instance}");
    for want in &unit.wants {
        let _ = activate_unit(units, &want, "");
    }
    println!("wants: {name}\t{instance}");
    println!("{name}\t{instance}");
    match &unit.section {
        crate::Section::Service {
            exec,
            exec_start,
            exec_stop,
        } => {
            Command::new(&exec_start[0])
                .args(&exec_start[1..])
                .spawn()
                .ok()?;
        }
        crate::Section::Mount {
            exec,
            what,
            r#where,
            r#type,
            options,
            sloppy_options,
        } => {
            std::fs::create_dir_all(r#where).ok()?;
            let mut cmd = Command::new("mount");
            if *sloppy_options {
                cmd.arg("-s");
            }
            cmd.arg("-o")
                .arg(
                    options
                        .iter()
                        .map(|o| apply_specifiers(o, name, instance))
                        .collect::<Vec<_>>()
                        .join(","),
                )
                .arg("-t")
                .arg(r#type)
                .arg(what)
                .arg(r#where)
                .spawn()
                .ok()?;
        }
        crate::Section::Target => {}
        crate::Section::Slice => {}
        crate::Section::Scope => {}
        _ => {
            println!("{name} activation not implemented");
            return None;
        }
    }
    println!("finished: {name}\t{instance}");
    Some(())
}

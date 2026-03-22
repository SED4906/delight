use std::{collections::BTreeMap, f64::consts::E, process::Command};

use crate::Unit;

pub fn activate_unit(units: &BTreeMap<String, Unit>, name: &str, template: &str) -> Option<()> {
    let unit = units.get(name)?;
    match &unit.section {
        crate::Section::Service { exec, exec_start, exec_stop } => {
            Command::new(&exec_start[0]).args(&exec_start[1..]).spawn().ok()?;
        }
        crate::Section::Mount { exec, what, r#where, r#type, options } => {
            Command::new("mount").args(options).arg("-t").arg(r#type).arg(what).arg(r#where).spawn().ok()?;
        },
        _ => println!("{name} activation not implemented"),
    }
    Some(())
}
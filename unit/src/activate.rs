use crate::{Cmd, state::{ACTIVE, UNITS}};

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

fn mount_options(options: &Vec<String>, name: &str, instance: &str) -> String {
    options
        .iter()
        .map(|o| apply_specifiers(o, name, instance))
        .collect::<Vec<_>>()
        .join(",")
}

pub fn activate_unit(name: &str, instance: &str) -> Option<()> {
    let name_at_instance = if instance.is_empty() {
        name.into()
    } else {
        let (start, end) = name.rsplit_once("@").unwrap();
        [&[start, instance].join("@"), end].join("")
    };
    if ACTIVE.lock().unwrap().insert(name_at_instance) {
        return Some(());
    }
    println!("activate: {name}\t{instance}");
    let unit = UNITS.lock().unwrap().get(name)?.clone();
    for require in &unit.requires {
        activate_unit(&require, "")?;
    }
    println!("requires: {name}\t{instance}");
    for want in &unit.wants {
        let _ = activate_unit(&want, "");
    }
    println!("wants: {name}\t{instance}");
    println!("{name}\t{instance}");
    match &unit.section {
        crate::Section::Service {
            exec,
            exec_start,
            exec_stop,
        } => {
            if exec_stop.is_empty() && exec_start.len() == 1 {
                let cmd = Cmd::new(exec_start[0].clone());
                exec.prog(cmd).spawn().ok()?;
            } else {
                println!("{name} activation not implemented, has {} start and {} stop", exec_start.len(), exec_stop.len());
                return None;
            }
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
            let mut cmd = exec.simple("mount");
            if *sloppy_options {
                cmd.arg("-s");
            }
            cmd.args(["-o", &mount_options(options, name, instance)])
                .args(["-t", r#type, what, r#where])
                .spawn()
                .ok()?;
        }
        crate::Section::Swap {
            exec,
            what,
            priority,
            options,
        } => {
            let mut cmd = exec.simple("swapon");
            if let Some(priority) = priority {
                cmd.args(["-p", priority]);
            }
            cmd.args(["-o", &mount_options(options, name, instance), what])
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

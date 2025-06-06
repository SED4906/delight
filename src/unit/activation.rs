use std::path::PathBuf;

use super::{Unit, UnitInfo, UnitName};

const UNIT_PATHS: &[&str] = &["/etc/systemd/system/","/usr/lib/systemd/system/"];

pub fn walk(node: String) -> Vec<Unit> {
    let mut queue = vec![];
    let mut order = vec![];
    if let Some(name) = check_path(node) {
        if let Ok(info) = name.info() {
            let requires = info.requires();
            for subnode in requires.clone().into_iter() {
                queue.append(&mut walk(subnode));
            }
            let wants = info.wants();
            for subnode in wants.clone().into_iter() {
                queue.append(&mut walk(subnode));
            }
            queue.push(Unit {
                name,
                info,
                requires,
                wants,
            });
        }
    }
    for subunit in queue {
        order.push(subunit);
    }
    order
}

fn check_path(node: String) -> Option<UnitName> {
    for unit_path in UNIT_PATHS {
        let mut unit_file: PathBuf = unit_path.into();
        let (name, template) = {
            match node.rsplit_once("@") {
                Some((name, template)) => {
                    let mut name: String = name.into();
                    name.push('@');
                    let (template,suffix) = template.rsplit_once(".")?;
                    name.push('.');
                    name.push_str(suffix);
                    (name,template.into())
                }
                None => (node.clone(), String::new())
            }
        };
        unit_file.push(name.clone());
        println!("checking {unit_file:?}");
        if std::fs::exists(&unit_file).ok().is_none_or(|p| !p) {
            continue;
        }
        return Some(UnitName { unit_file, name, template });
    }
    None
}

impl UnitInfo {
    pub fn requires(&self) -> Vec<String> {
        let mut requires = vec![];
        if let Some(lines) = self.unit.get("Requires") {
            for line in lines {
                for name in line.split_whitespace() {
                    requires.push(name.into());
                }
            }
        }
        requires
    }

    pub fn wants(&self) -> Vec<String> {
        let mut wants = vec![];
        if let Some(lines) = self.unit.get("Wants") {
            for line in lines {
                for name in line.split_whitespace() {
                    wants.push(name.into());
                }
            }
        }
        wants
    }
}

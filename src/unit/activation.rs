use std::{cmp::Ordering, path::PathBuf};

use super::{Unit, UnitInfo, UnitName, UnitType};

const UNIT_PATHS: &[&str] = &["/etc/systemd/system/","/usr/lib/systemd/system/"];

pub fn walk(node: String) -> Vec<Unit> {
    let mut queue = vec![];
    if let Some(name) = check_path(node.clone()) {
        if let Ok(info) = name.info() {
            let requires = info.depend("Requires");
            let mut wants = info.depend("Wants");
            let mut after = info.depend("After");
            let before = info.depend("Before");
            match info.unit_type {
                UnitType::Target => {
                    let mut subdir = String::new();
                    subdir.push_str(node.as_str());
                    subdir.push_str(".wants");
                    wants.append(&mut check_path_dir(subdir));
                }
                _ => {}
            }
            for subnode in requires.clone().into_iter() {
                queue.append(&mut walk(subnode));
            }
            for subnode in wants.clone().into_iter() {
                queue.append(&mut walk(subnode));
            }
            match info.unit_type {
                UnitType::Target => {
                    after.append(&mut wants.clone());
                    after.append(&mut requires.clone());
                }
                _ => {}
            }
            queue.push(Unit {
                name,
                info,
                requires,
                wants,
                after,
                before,
            });
        }
    }
    queue
}

pub fn depends_sort(queue: &mut Vec<Unit>) {
    queue.sort_by(|a,b| {
        if a.after.contains(&b.name.name) {
            Ordering::Greater
        } else if b.after.contains(&a.name.name) {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    });
    queue.sort_by(|a,b| {
        if a.before.contains(&b.name.name) {
            Ordering::Less
        } else if b.before.contains(&a.name.name) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });
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
        if std::fs::exists(&unit_file).ok().is_none_or(|p| !p) {
            continue;
        }
        return Some(UnitName { unit_file, name, template });
    }
    None
}

fn check_path_dir(subdir: String) -> Vec<String> {
    let mut units = vec![];
    for unit_path in UNIT_PATHS {
        let mut unit_subdir: PathBuf = unit_path.into();
        unit_subdir.push(subdir.clone());
        if std::fs::exists(&unit_subdir).ok().is_none_or(|p| !p) {
            continue;
        }
        if let Ok(read_dir) = std::fs::read_dir(&unit_subdir) {
            for unit_link in read_dir {
                if let Ok(unit_link) = unit_link {
                    if let Ok(node) = unit_link.file_name().into_string() {
                        units.push(node);
                    }
                }
            }
        }

    }
    units
}

impl UnitInfo {
    pub fn depend(&self, key: &str) -> Vec<String> {
        let mut depend = vec![];
        if let Some(lines) = self.unit.get(key) {
            for line in lines {
                for name in line.split_whitespace() {
                    depend.push(name.into());
                }
            }
        }
        depend
    }
}

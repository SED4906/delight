use std::collections::BTreeMap;

use crate::{Exec, Section, Unit, UnitType};

pub struct Ini(BTreeMap<String, BTreeMap<String, Vec<String>>>);

impl Ini {
    pub fn parse(raw: &str) -> Option<Ini> {
        let mut result = BTreeMap::new();
        let mut section = None;
        let mut working_line = String::new();
        let mut continued = false;
        for line in raw.lines() {
            let line = line.trim();
            match line.chars().next() {
                Some('#' | ';') => continue,
                Some('[') if !continued => {
                    if line.ends_with(']') {
                        section = line.strip_prefix('[')?.strip_suffix(']');
                    }
                }
                _ => {
                    if line.ends_with('\\') {
                        continued = true;
                        working_line.push_str(line.strip_suffix('\\')?.trim());
                        working_line.push(' ');
                    } else {
                        continued = false;
                        working_line.push_str(line);
                    }
                }
            }
            if continued || working_line.is_empty() {
                continue;
            }
            let (key, value) = line.split_once('=')?;
            let (key, value) = (key.trim(), value.trim());
            let section = section?;
            if !result.contains_key(section) {
                result.insert(section.into(), BTreeMap::new());
            }
            if value.is_empty() || !result.get_mut(section)?.contains_key(key) {
                result.get_mut(section)?.insert(key.into(), vec![]);
            }
            if !value.is_empty() {
                result.get_mut(section)?.get_mut(key)?.push(value.into());
            }
            working_line.clear();
        }
        Some(Ini(result))
    }

    pub fn get(&self, section: &str, key: &str) -> Option<Vec<String>> {
        self.0.get(section)?.get(key).cloned()
    }

    pub fn get_1(&self, section: &str, key: &str) -> Option<String> {
        self.0.get(section)?.get(key)?.last().cloned()
    }

    pub fn get_delimited(&self, section: &str, key: &str, delimiter: &str) -> Option<Vec<String>> {
        self.0
            .get(section)?
            .get(key)?
            .join(delimiter)
            .split(delimiter)
            .map(|s| Some(s.into()))
            .collect()
    }
}

macro_rules! parse_exec {
    ($ini:expr, $section:literal) => {
        Exec {
            exec_search_path: $ini.get_delimited($section, "ExecSearchPath", ":"),
            working_directory: $ini.get_1($section, "WorkingDirectory"),
            user: $ini.get_1($section, "User"),
            group: $ini.get_1($section, "Group"),
        }
    };
}

impl Unit {
    pub fn new(input: &str, unit_type: UnitType) -> Option<Self> {
        let ini = Ini::parse(input)?;
        let section = match unit_type {
            UnitType::Service => Section::Service {
                exec: parse_exec!(ini, "Service"),
                exec_start: ini.get("Service", "ExecStart").unwrap_or_default(),
                exec_stop: ini.get("Service", "ExecStop").unwrap_or_default(),
            },
            UnitType::Mount => Section::Mount {
                exec: parse_exec!(ini, "Mount"),
                what: ini.get_1("Mount", "What")?,
                r#where: ini.get_1("Mount", "Where")?,
                r#type: ini.get_1("Mount", "Type").unwrap_or("auto".into()),
                options: ini
                    .get_delimited("Mount", "Options", ",")
                    .unwrap_or_default(),
            },
            UnitType::Swap => Section::Swap {
                exec: parse_exec!(ini, "Swap"),
                what: ini.get_1("Swap", "What")?,
                options: ini
                    .get_delimited("Swap", "Options", ",")
                    .unwrap_or_default(),
            },
            UnitType::Socket => Section::Socket {
                exec: parse_exec!(ini, "Socket"),
                service: ini.get_1("Socket", "Service"),
            },
            UnitType::Target => Section::Target,
            UnitType::Device => Section::Device,
            UnitType::Automount => Section::Automount {
                r#where: ini.get_1("Automount", "Where")?,
                extra_options: ini
                    .get_delimited("Automount", "ExtraOptions", ",")
                    .unwrap_or_default(),
            },
            UnitType::Timer => Section::Timer {
                unit: ini.get_1("Timer", "Unit"),
            },
            UnitType::Path => Section::Path {
                unit: ini.get_1("Path", "Unit"),
            },
            UnitType::Slice => Section::Slice,
            UnitType::Scope => Section::Scope,
        };

        let requires = ini
            .get_delimited("Unit", "Requires", " ")
            .unwrap_or_default();
        let wants = ini.get_delimited("Unit", "Wants", " ").unwrap_or_default();
        let after = ini.get_delimited("Unit", "After", " ").unwrap_or_default();
        let before = ini.get_delimited("Unit", "Before", " ").unwrap_or_default();

        let alias = ini.get("Install", "Alias").unwrap_or_default();
        let wanted_by = ini
            .get_delimited("Install", "WantedBy", " ")
            .unwrap_or_default();
        let required_by = ini
            .get_delimited("Install", "RequiredBy", " ")
            .unwrap_or_default();

        Some(Self {
            section,
            requires,
            wants,
            after,
            before,
            alias,
            wanted_by,
            required_by,
        })
    }
}

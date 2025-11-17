use std::collections::BTreeMap;

use crate::{Exec, Section, Unit, UnitType};

fn ini_get_1(
    ini: &BTreeMap<String, BTreeMap<String, Vec<String>>>,
    section: &str,
    key: &str,
) -> Option<String> {
    ini.get(section)?.get(key)?.last().cloned()
}

fn ini_get_delimited(
    ini: &BTreeMap<String, BTreeMap<String, Vec<String>>>,
    section: &str,
    key: &str,
    delimiter: &str,
) -> Option<Vec<String>> {
    ini.get(section)?
        .get(key)?
        .join(delimiter)
        .split(delimiter)
        .map(|s| Some(s.into()))
        .collect()
}

fn ini_get(
    ini: &BTreeMap<String, BTreeMap<String, Vec<String>>>,
    section: &str,
    key: &str,
) -> Option<Vec<String>> {
    ini.get(section)?.get(key).cloned()
}

macro_rules! parse_exec {
    ($ini:expr, $section:literal) => {
        Exec {
            exec_search_path: ini_get_delimited(&$ini, $section, "ExecSearchPath", ":"),
            working_directory: ini_get_1(&$ini, $section, "WorkingDirectory"),
            user: ini_get_1(&$ini, $section, "User"),
            group: ini_get_1(&$ini, $section, "Group"),
        }
    };
}

impl Unit {
    pub fn new(input: &str, unit_type: UnitType) -> Option<Self> {
        let ini = Unit::parse_ini(input)?;
        let section = match unit_type {
            UnitType::Service => Section::Service {
                exec: parse_exec!(ini, "Service"),
                exec_start: ini_get(&ini, "Service", "ExecStart").unwrap_or_default(),
                exec_stop: ini_get(&ini, "Service", "ExecStop").unwrap_or_default(),
            },
            UnitType::Mount => Section::Mount {
                exec: parse_exec!(ini, "Mount"),
                what: ini_get_1(&ini, "Mount", "What")?,
                r#where: ini_get_1(&ini, "Mount", "Where")?,
                r#type: ini_get_1(&ini, "Mount", "Type").unwrap_or("auto".into()),
                options: ini_get_delimited(&ini, "Mount", "Options", ",").unwrap_or_default(),
            },
            UnitType::Swap => Section::Swap {
                exec: parse_exec!(ini, "Swap"),
                what: ini_get_1(&ini, "Swap", "What")?,
                options: ini_get_delimited(&ini, "Swap", "Options", ",").unwrap_or_default(),
            },
            UnitType::Socket => Section::Socket {
                exec: parse_exec!(ini, "Socket"),
                service: ini_get_1(&ini, "Socket", "Service"),
            },
            UnitType::Target => Section::Target,
            UnitType::Device => Section::Device,
            UnitType::Automount => Section::Automount {
                r#where: ini_get_1(&ini, "Automount", "Where")?,
                extra_options: ini_get_delimited(&ini, "Automount", "ExtraOptions", ",")
                    .unwrap_or_default(),
            },
            UnitType::Timer => Section::Timer {
                unit: ini_get_1(&ini, "Timer", "Unit"),
            },
            UnitType::Path => Section::Path {
                unit: ini_get_1(&ini, "Path", "Unit"),
            },
            UnitType::Slice => Section::Slice,
            UnitType::Scope => Section::Scope,
        };

        let requires = ini_get_delimited(&ini, "Unit", "Requires", " ").unwrap_or_default();
        let wants = ini_get_delimited(&ini, "Unit", "Wants", " ").unwrap_or_default();
        let after = ini_get_delimited(&ini, "Unit", "After", " ").unwrap_or_default();
        let before = ini_get_delimited(&ini, "Unit", "Before", " ").unwrap_or_default();

        let alias = ini_get(&ini, "Install", "Alias").unwrap_or_default();
        let wanted_by = ini_get_delimited(&ini, "Install", "WantedBy", " ").unwrap_or_default();
        let required_by = ini_get_delimited(&ini, "Install", "RequiredBy", " ").unwrap_or_default();

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

    fn parse_ini(input: &str) -> Option<BTreeMap<String, BTreeMap<String, Vec<String>>>> {
        let mut result = BTreeMap::<String, BTreeMap<String, Vec<String>>>::new();
        let mut current_section = None;
        let mut line = String::new();
        let mut backslashed = false;
        for input_line in input.lines() {
            let input_line = input_line.trim();
            match input_line.chars().next() {
                Some('#' | ';') => {}
                Some('[') => {
                    if !backslashed && input_line.ends_with(']') {
                        current_section = Some(input_line.strip_prefix("[")?.strip_suffix("]")?);
                        backslashed = false;
                    }
                }
                Some(_) => {
                    if !backslashed {
                        if input_line.ends_with("\\") {
                            backslashed = true;
                            line.push_str(input_line.strip_suffix("\\")?.trim());
                            line.push(' ');
                        } else {
                            backslashed = false;
                            line.push_str(input_line);
                        }
                    }
                }
                None => {
                    backslashed = false;
                }
            }
            if !backslashed && !line.is_empty() {
                let (key, value) = line.split_once('=')?;
                let key = key.trim();
                let value = value.trim();
                // Initialize the section, if necessary
                if !result.contains_key(current_section?) {
                    result.insert(current_section?.into(), BTreeMap::new());
                }
                // If the value is empty, unset the key; if the key doesn't exist yet, create it.
                if value.is_empty() || !result.get_mut(current_section?)?.contains_key(key) {
                    result.get_mut(current_section?)?.insert(key.into(), vec![]);
                }
                // If the value isn't empty, add it to the key.
                if !value.is_empty() {
                    result
                        .get_mut(current_section?)?
                        .get_mut(key)?
                        .push(value.into());
                }
                line = String::new();
            }
        }
        Some(result)
    }
}

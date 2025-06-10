use std::{collections::BTreeMap, error::Error, fmt::Display};

use super::{Section, UnitInfo, UnitName, UnitType};

#[derive(Debug)]
pub enum UnitParseError {
    UnknownType,
    ProbableParserBug,
}

impl Display for UnitParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self:?}"))
    }
}

impl Error for UnitParseError {}

impl UnitName {
    pub fn info(&self) -> Result<UnitInfo, Box<dyn Error>> {
        let unit_file_contents = std::fs::read_to_string(self.unit_file.as_path())?;
        let unit_type = self.unit_type().ok_or(UnitParseError::UnknownType)?;
        let sections = UnitInfo::parse(unit_file_contents.as_str(), &self)
            .ok_or(UnitParseError::ProbableParserBug)?;
        let unit = sections
            .get("Unit")
            .unwrap_or(&BTreeMap::<String, Vec<String>>::new())
            .clone();
        let install = sections
            .get("Install")
            .unwrap_or(&BTreeMap::<String, Vec<String>>::new())
            .clone();
        let section = match unit_type {
            UnitType::Target | UnitType::Device => None,
            UnitType::Service => sections.get("Service"),
            UnitType::Mount => sections.get("Mount"),
            UnitType::Swap => sections.get("Swap"),
            UnitType::Socket => sections.get("Socket"),
            UnitType::Automount => sections.get("Automount"),
            UnitType::Timer => sections.get("Timer"),
            UnitType::Path => sections.get("Path"),
            UnitType::Slice => sections.get("Slice"),
            UnitType::Scope => sections.get("Scope"),
        }
        .cloned();
        Ok(UnitInfo {
            unit_type,
            unit,
            install,
            section,
        })
    }

    pub fn unit_type(&self) -> Option<UnitType> {
        Some(match self.name.rsplit_once('.')?.1 {
            "service" => UnitType::Service,
            "mount" => UnitType::Mount,
            "swap" => UnitType::Swap,
            "socket" => UnitType::Socket,
            "target" => UnitType::Target,
            "device" => UnitType::Device,
            "automount" => UnitType::Automount,
            "timer" => UnitType::Timer,
            "path" => UnitType::Path,
            "slice" => UnitType::Slice,
            "scope" => UnitType::Scope,
            _ => return None,
        })
    }
}

impl UnitInfo {
    pub fn parse(contents: &str, unit_name: &UnitName) -> Option<BTreeMap<String, Section>> {
        let mut sections = BTreeMap::new();
        let mut working_line = String::new();
        let mut current_section_name = String::new();
        let mut line_continuation = false;
        for line in contents.lines() {
            let line = line.trim();
            match line.chars().next() {
                Some('#' | ';') => {}
                Some('[') if !line_continuation => {
                    if line.ends_with(']') {
                        current_section_name = line.strip_prefix("[")?.strip_suffix("]")?.into();
                        working_line = String::new();
                        if !sections.contains_key(&current_section_name) {
                            sections.insert(current_section_name.clone(), BTreeMap::new());
                        }
                    }
                }
                Some(_) => {
                    if line.ends_with("\\") {
                        line_continuation = true;
                        working_line.push_str(line.strip_suffix("\\")?.trim());
                        working_line.push(' ');
                    } else {
                        line_continuation = false;
                        working_line.push_str(line);
                    }
                }
                None => {
                    line_continuation = false;
                }
            }
            if !line_continuation && !current_section_name.is_empty() {
                Self::parse_entry(
                    &mut sections,
                    &working_line,
                    &current_section_name,
                    &unit_name,
                );
                working_line = String::new();
            }
        }
        Some(sections)
    }

    fn parse_entry(
        sections: &mut BTreeMap<String, Section>,
        line: &str,
        current_section_name: &str,
        unit_name: &UnitName,
    ) {
        match line.split_once("=") {
            Some((key, value)) => {
                let key = key.trim();
                let value = Self::apply_template(value.trim(), &unit_name);
                if value.is_empty()
                    || !sections
                        .get_mut(current_section_name)
                        .unwrap()
                        .contains_key(key)
                {
                    sections
                        .get_mut(current_section_name)
                        .unwrap()
                        .insert(key.into(), vec![]);
                }
                if !value.is_empty() {
                    sections
                        .get_mut(current_section_name)
                        .unwrap()
                        .get_mut(key)
                        .unwrap()
                        .push(value.into());
                }
            }
            None => {}
        }
    }

    fn apply_template(value: &str, unit_name: &UnitName) -> String {
        let working_value = value.replace("%i", &unit_name.template);
        let working_value = working_value.replace("%n", unit_name.name.as_str());
        let working_value = working_value.replace(
            "%p",
            &unit_name
                .name
                .as_str()
                .rsplit_once("@")
                .unwrap_or((unit_name.name.as_str().rsplit_once(".").unwrap().0, ""))
                .0,
        );
        let working_value = working_value.replace("%I", &unit_name.template);
        let working_value =
            working_value.replace("%N", unit_name.name.as_str().rsplit_once(".").unwrap().0);
        let working_value = working_value.replace(
            "%P",
            &unit_name
                .name
                .as_str()
                .rsplit_once("@")
                .unwrap_or((unit_name.name.as_str().rsplit_once(".").unwrap().0, ""))
                .0,
        );
        let working_value = working_value.replace("%%", "%");
        working_value
    }
}

use std::{collections::{BTreeMap, BTreeSet}, sync::Mutex};

use crate::Unit;

pub static UNITS: Mutex<BTreeMap<String, Unit>> = Mutex::new(BTreeMap::new());
pub static ACTIVE: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

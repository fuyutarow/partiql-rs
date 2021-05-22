use std::collections::HashMap;
use std::fmt;

use serde_derive::{Deserialize, Serialize};

use crate::sql::Field;
use crate::sql::WhereCond;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonValue {
    Null,
    Str(String),
    Boolean(bool),
    Num(f64),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn get(self, key: &str) -> Option<JsonValue> {
        match self {
            JsonValue::Object(map) => {
                if let Some(value) = map.get(key) {
                    Some(value.to_owned())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn get_path(self, path: &[&str]) -> Option<JsonValue> {
        if let Some((key, path)) = path.split_first() {
            if let Some(obj) = self.get(key) {
                if path.len() > 0 {
                    obj.get_path(path)
                } else {
                    Some(obj)
                }
            } else {
                None
            }
        } else {
            unreachable!();
        }
    }

    pub fn by_path(&self, path: &[&str]) -> Option<JsonValue> {
        match self {
            Self::Object(map) => {
                if let Some((key, tail_path)) = path.split_first() {
                    if let Some(obj) = self.clone().get(key) {
                        obj.by_path(tail_path)
                    } else {
                        None
                    }
                } else {
                    Some(self.to_owned())
                }
            }
            Self::Array(array) => {
                let new_array = array
                    .into_iter()
                    .filter_map(|value| value.by_path(path))
                    .collect::<Vec<_>>();

                Some(JsonValue::Array(new_array))
            }
            _ => {
                dbg!("#2");
                Some(self.clone())
            }
        }
    }

    pub fn _filter(self, path: &[&str]) -> Option<JsonValue> {
        match self {
            JsonValue::Object(map) => {
                let mut new_map = HashMap::<String, JsonValue>::new();

                for key in path {
                    if let Some(value) = map.get(key.to_string().as_str()) {
                        new_map.insert(key.to_string(), value.to_owned());
                    }
                }

                Some(JsonValue::Object(new_map))
            }
            _ => None,
        }
    }

    pub fn _filter_map(self, path: &[&str]) -> Option<JsonValue> {
        match self {
            JsonValue::Array(array) => {
                let new_array = array
                    .into_iter()
                    .filter_map(|value| value._filter(path))
                    .collect::<Vec<_>>();

                Some(JsonValue::Array(new_array))
            }
            _ => None,
        }
    }

    // pub fn select_by_path_list(self, path_list: &[&[&str]]]) -> Option<JsonValue> {
    //     match self {
    //         JsonValue::Object(map) => {
    //             let mut new_map = HashMap::<String, JsonValue>::new();

    //             for field in field_list {
    //                 dbg!("!!", field);
    //                 if let Some(value) = map.get(&field.path) {
    //                     let key = field.alias.as_ref().unwrap_or(&field.path);
    //                     new_map.insert(key.to_string(), value.to_owned());
    //                 }
    //             }

    //             Some(JsonValue::Object(new_map))
    //         }
    //         _ => None,
    //     }
    // }

    pub fn select(self, field_list: &[Field]) -> Option<JsonValue> {
        let mut new_map = HashMap::<String, JsonValue>::new();

        for field in field_list {
            dbg!("!!", field);
            let path = field.path.split(".").collect::<Vec<&str>>();
            dbg!(&path, &self, self.by_path(&path));
            if let Some(value) = self.by_path(&path) {
                let key = field.alias.clone().unwrap_or({
                    let last = path.last().unwrap().to_string();
                    last
                });
                new_map.insert(key, value);
            }
        }

        Some(JsonValue::Object(new_map))
    }

    pub fn select_map(self, field_list: &[Field]) -> Option<JsonValue> {
        match self {
            JsonValue::Array(array) => {
                dbg!("array", &array);
                let new_array = array
                    .into_iter()
                    .filter_map(|value| value.select(field_list))
                    .collect::<Vec<_>>();
                dbg!("new_array", &new_array);

                Some(JsonValue::Array(new_array))
            }
            _ => None,
        }
    }

    pub fn filter_map(self, conditon: WhereCond) -> Option<JsonValue> {
        match self {
            JsonValue::Array(array) => {
                let new_array = array
                    .into_iter()
                    .filter_map(|left| match conditon.to_owned() {
                        WhereCond::Eq { field, right } => {
                            if let Some(value) = left.clone().get(&field.path) {
                                if value == JsonValue::Str(right) {
                                    Some(left)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                        WhereCond::Like { field, right } => {
                            let pattern = match (right.starts_with("%"), right.ends_with("%")) {
                                (true, true) => format!(
                                    "{}",
                                    right.trim_start_matches("%").trim_end_matches("%")
                                ),
                                (true, false) => format!("{}$", right.trim_start_matches("%")),
                                (false, true) => format!("^{}", right.trim_end_matches("%")),
                                (false, false) => format!("^{}$", right),
                            };
                            let re = regex::Regex::new(&pattern).unwrap();
                            dbg!(">>", &left, &pattern);
                            match left.clone().get(&field.path) {
                                Some(JsonValue::Str(s)) if re.is_match(&s) => Some(left),
                                _ => None,
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                Some(JsonValue::Array(new_array))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Atom {
    data: u64,
}

impl From<u64> for Atom {
    fn from(v: u64) -> Self {
        Self { data: v }
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{{");
        writeln!(f, "  '_1': {},", self.data);
        writeln!(f, "}}");
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Array {
    data: Vec<Atom>,
}

impl From<&[u64]> for Array {
    fn from(v: &[u64]) -> Self {
        Self {
            data: v.to_vec().into_iter().map(Atom::from).collect::<Vec<_>>(),
        }
    }
}

impl fmt::Display for Array {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "<<");
        for atom in self.data.iter() {
            writeln!(f, "  {},", atom);
        }
        writeln!(f, ">>");
        Ok(())
    }
}

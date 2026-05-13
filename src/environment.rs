use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::exact::ExactNum;

#[derive(Serialize, Deserialize)]
struct EnvironmentJson {
    vars: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct Environment {
    vars: HashMap<String, ExactNum>,
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Serialize for Environment {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let json = EnvironmentJson {
            vars: self
                .vars
                .iter()
                .map(|(k, v)| (k.clone(), v.to_f64()))
                .collect(),
        };
        json.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Environment {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let json = EnvironmentJson::deserialize(deserializer)?;
        let vars = json
            .vars
            .into_iter()
            .map(|(k, v)| (k, ExactNum::from_f64(v)))
            .collect();
        Ok(Environment { vars })
    }
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            vars: HashMap::new(),
        }
    }

    pub fn get(&self, var: &str) -> Option<f64> {
        self.vars.get(var).map(|n| n.to_f64())
    }

    pub fn get_exact(&self, var: &str) -> Option<&ExactNum> {
        self.vars.get(var)
    }

    pub fn set(&mut self, var: &str, value: f64) {
        self.vars.insert(var.to_string(), ExactNum::from_f64(value));
    }

    pub fn set_exact(&mut self, var: &str, value: ExactNum) {
        self.vars.insert(var.to_string(), value);
    }
}

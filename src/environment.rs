use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Environment {
    pub vars: HashMap<String, f64>,
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            vars: HashMap::new(),
        }
    }

    pub fn get(&self, var: &str) -> Option<f64> {
        self.vars.get(var).cloned()
    }

    pub fn set(&mut self, var: &str, value: f64) {
        self.vars.insert(var.to_string(), value);
    }
}

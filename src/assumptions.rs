use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Assumption {
    Positive,
    NonNegative,
    Negative,
    NonZero,
    Real,
    Integer,
}

impl Assumption {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "positive" => Some(Assumption::Positive),
            "nonnegative" | "non_negative" => Some(Assumption::NonNegative),
            "negative" => Some(Assumption::Negative),
            "nonzero" | "non_zero" => Some(Assumption::NonZero),
            "real" => Some(Assumption::Real),
            "integer" => Some(Assumption::Integer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Assumptions {
    props: HashMap<String, HashSet<Assumption>>,
}

impl Assumptions {
    pub fn new() -> Self {
        Assumptions {
            props: HashMap::new(),
        }
    }

    pub fn assume(&mut self, var: &str, prop: Assumption) {
        self.props
            .entry(var.to_string())
            .or_default()
            .insert(prop);
    }

    pub fn has(&self, var: &str, prop: &Assumption) -> bool {
        self.props
            .get(var)
            .is_some_and(|set| set.contains(prop))
    }

    pub fn is_positive(&self, var: &str) -> bool {
        self.has(var, &Assumption::Positive)
    }

    pub fn is_nonneg(&self, var: &str) -> bool {
        self.is_positive(var) || self.has(var, &Assumption::NonNegative)
    }

    pub fn is_negative(&self, var: &str) -> bool {
        self.has(var, &Assumption::Negative)
    }

    pub fn is_nonzero(&self, var: &str) -> bool {
        self.is_positive(var) || self.is_negative(var) || self.has(var, &Assumption::NonZero)
    }

    pub fn is_real(&self, var: &str) -> bool {
        self.has(var, &Assumption::Real)
    }

    pub fn is_integer(&self, var: &str) -> bool {
        self.has(var, &Assumption::Integer)
    }

    pub fn is_empty(&self) -> bool {
        self.props.is_empty()
    }

    pub fn from_json(value: &Value) -> Result<Self, String> {
        let obj = value
            .as_object()
            .ok_or("assumptions must be a JSON object")?;
        let mut assumptions = Assumptions::new();
        for (var, props) in obj {
            let arr = props
                .as_array()
                .ok_or(format!("assumptions for '{}' must be an array", var))?;
            for prop_val in arr {
                let prop_str = prop_val
                    .as_str()
                    .ok_or("assumption property must be a string".to_string())?;
                let prop = Assumption::from_str(prop_str).ok_or(format!(
                    "unknown assumption '{}'. Valid: positive, nonnegative, negative, nonzero, real, integer",
                    prop_str
                ))?;
                assumptions.assume(var, prop);
            }
        }
        Ok(assumptions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_assumptions() {
        let a = Assumptions::new();
        assert!(a.is_empty());
        assert!(!a.is_positive("x"));
        assert!(!a.is_nonneg("x"));
        assert!(!a.is_integer("n"));
    }

    #[test]
    fn test_positive_implies_nonneg_and_nonzero() {
        let mut a = Assumptions::new();
        a.assume("x", Assumption::Positive);
        assert!(a.is_positive("x"));
        assert!(a.is_nonneg("x"));
        assert!(a.is_nonzero("x"));
        assert!(!a.is_negative("x"));
        assert!(!a.is_integer("x"));
    }

    #[test]
    fn test_negative_implies_nonzero() {
        let mut a = Assumptions::new();
        a.assume("x", Assumption::Negative);
        assert!(a.is_negative("x"));
        assert!(a.is_nonzero("x"));
        assert!(!a.is_positive("x"));
        assert!(!a.is_nonneg("x"));
    }

    #[test]
    fn test_nonneg_does_not_imply_positive() {
        let mut a = Assumptions::new();
        a.assume("x", Assumption::NonNegative);
        assert!(a.is_nonneg("x"));
        assert!(!a.is_positive("x"));
        assert!(!a.is_nonzero("x"));
    }

    #[test]
    fn test_multiple_vars() {
        let mut a = Assumptions::new();
        a.assume("x", Assumption::Positive);
        a.assume("n", Assumption::Integer);
        a.assume("y", Assumption::Real);
        assert!(a.is_positive("x"));
        assert!(a.is_integer("n"));
        assert!(a.is_real("y"));
        assert!(!a.is_positive("n"));
    }

    #[test]
    fn test_from_json() {
        let json: Value = serde_json::json!({
            "x": ["positive"],
            "n": ["integer"],
            "y": ["real", "nonzero"]
        });
        let a = Assumptions::from_json(&json).unwrap();
        assert!(a.is_positive("x"));
        assert!(a.is_integer("n"));
        assert!(a.is_real("y"));
        assert!(a.is_nonzero("y"));
    }

    #[test]
    fn test_from_json_unknown_property() {
        let json: Value = serde_json::json!({
            "x": ["complex"]
        });
        assert!(Assumptions::from_json(&json).is_err());
    }
}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Node {
    // Leaf nodes: numbers or variables
    Number(f64),
    Variable(String),
    Rational(i64, i64), // Numerator and denominator

    // Internal nodes: operators with children (operands)
    Add(Box<Node>, Box<Node>),
    Subtract(Box<Node>, Box<Node>),
    Multiply(Box<Node>, Box<Node>),
    Divide(Box<Node>, Box<Node>),
    Power(Box<Node>, Box<Node>),
    Sqrt(Box<Node>),
    Abs(Box<Node>),
    Negate(Box<Node>), // Add this for unary negation

    // Comparators
    Greater(Box<Node>, Box<Node>),
    Less(Box<Node>, Box<Node>),
    GreaterEqual(Box<Node>, Box<Node>),
    LessEqual(Box<Node>, Box<Node>),

    // Piecewise expressions
    Piecewise(Vec<(Node, Node)>),

    // Function calls
    Function(String, Vec<Node>), // For functions like sin, cos
}

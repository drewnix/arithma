use crate::exact::ExactNum;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Node {
    Num(ExactNum),
    Variable(String),

    // Internal nodes: operators with children (operands)
    Add(Box<Node>, Box<Node>),
    Subtract(Box<Node>, Box<Node>),
    Multiply(Box<Node>, Box<Node>),
    Divide(Box<Node>, Box<Node>),
    Power(Box<Node>, Box<Node>),
    Sqrt(Box<Node>),
    Abs(Box<Node>),
    Negate(Box<Node>),

    // Comparators
    Greater(Box<Node>, Box<Node>),
    Less(Box<Node>, Box<Node>),
    GreaterEqual(Box<Node>, Box<Node>),
    LessEqual(Box<Node>, Box<Node>),
    Equal(Box<Node>, Box<Node>),

    // Equation (left side = right side)
    Equation(Box<Node>, Box<Node>),

    // Piecewise expressions
    Piecewise(Vec<(Node, Node)>),

    // Summation: index_var, start, end, body
    Summation(String, Box<Node>, Box<Node>, Box<Node>),

    // Function calls
    Function(String, Vec<Node>), // For functions like sin, cos
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Node::Num(n) => write!(f, "{}", n),
            Node::Variable(v) => write!(f, "{}", v),
            Node::Add(left, right) => {
                if let Node::Num(n) = &**right {
                    if n.is_zero() {
                        return write!(f, "{}", left);
                    }
                }
                write!(f, "{} + {}", left, right)
            }
            Node::Multiply(left, right) => {
                if let Node::Num(n) = &**left {
                    if n.is_one() {
                        return write!(f, "{}", right);
                    }
                    if n.is_zero() {
                        return write!(f, "0");
                    }
                }
                if let (Node::Num(l), Node::Variable(r)) = (&**left, &**right) {
                    write!(f, "{}{}", l, r)
                } else {
                    write!(f, "{} \\cdot {}", left, right)
                }
            }
            Node::Subtract(left, right) => write!(f, "{} - {}", left, right),
            Node::Divide(left, right) => write!(f, "{} / {}", left, right),
            Node::Power(left, right) => write!(f, "{}^{}", left, right),
            Node::Sqrt(operand) => write!(f, "\\sqrt{{{}}}", operand),
            Node::Abs(operand) => write!(f, "|{}|", operand),
            Node::Negate(operand) => write!(f, "-{}", operand),
            Node::Greater(left, right) => write!(f, "({} > {})", left, right),
            Node::Less(left, right) => write!(f, "({} < {})", left, right),
            Node::GreaterEqual(left, right) => write!(f, "({} >= {})", left, right),
            Node::LessEqual(left, right) => write!(f, "({} <= {})", left, right),
            Node::Equal(left, right) => write!(f, "({} == {})", left, right),
            Node::Equation(left, right) => write!(f, "{} = {}", left, right),
            Node::Piecewise(conditions) => {
                let mut formatted_conditions = String::new();
                for (expr, cond) in conditions {
                    formatted_conditions.push_str(&format!("{} if {}, ", expr, cond));
                }
                write!(f, "piecewise({})", formatted_conditions)
            }
            Node::Summation(index_var, start, end, body) => {
                write!(
                    f,
                    "\\sum_{{{} = {}}}^{{{}}}{{{}}}",
                    index_var, start, end, body
                )
            }
            Node::Function(name, args) => {
                let formatted_args = args
                    .iter()
                    .map(|arg| format!("{}", arg))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{}({})", name, formatted_args)
            }
        }
    }
}

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

impl Node {
    fn precedence(&self) -> u8 {
        match self {
            Node::Equation(_, _) => 0,
            Node::Greater(_, _)
            | Node::Less(_, _)
            | Node::GreaterEqual(_, _)
            | Node::LessEqual(_, _)
            | Node::Equal(_, _) => 1,
            Node::Add(_, _) | Node::Subtract(_, _) => 2,
            Node::Multiply(_, _) | Node::Divide(_, _) => 3,
            Node::Power(_, _) => 4,
            Node::Negate(_) => 5,
            _ => 10, // atoms, functions, sqrt, abs — never need outer parens
        }
    }

    fn fmt_child(
        &self,
        child: &Node,
        parent_prec: u8,
        is_right: bool,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        let child_prec = child.precedence();
        let needs_parens = child_prec < parent_prec
            || (child_prec == parent_prec
                && is_right
                && matches!(self, Node::Subtract(_, _) | Node::Divide(_, _)));

        if needs_parens {
            write!(f, "({})", child)
        } else {
            write!(f, "{}", child)
        }
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Node::Num(n) => write!(f, "{}", n),
            Node::Variable(v) => write!(f, "{}", v),
            Node::Add(left, right) => {
                self.fmt_child(left, 2, false, f)?;
                write!(f, " + ")?;
                self.fmt_child(right, 2, true, f)
            }
            Node::Subtract(left, right) => {
                self.fmt_child(left, 2, false, f)?;
                write!(f, " - ")?;
                self.fmt_child(right, 2, true, f)
            }
            Node::Multiply(left, right) => {
                if let (Node::Num(l), Node::Variable(r)) = (&**left, &**right) {
                    if l.is_one() {
                        return write!(f, "{}", r);
                    }
                    if *l == ExactNum::integer(-1) {
                        return write!(f, "-{}", r);
                    }
                    return write!(f, "{}{}", l, r);
                }
                self.fmt_child(left, 3, false, f)?;
                write!(f, " \\cdot ")?;
                self.fmt_child(right, 3, true, f)
            }
            Node::Divide(left, right) => {
                write!(f, "\\frac{{{}}}{{{}}}", left, right)
            }
            Node::Power(base, exp) => {
                let base_needs_parens = matches!(
                    **base,
                    Node::Add(_, _)
                        | Node::Subtract(_, _)
                        | Node::Multiply(_, _)
                        | Node::Divide(_, _)
                        | Node::Negate(_)
                );
                if base_needs_parens {
                    write!(f, "({})", base)?;
                } else {
                    write!(f, "{}", base)?;
                }
                write!(f, "^{{{}}}", exp)
            }
            Node::Sqrt(operand) => write!(f, "\\sqrt{{{}}}", operand),
            Node::Abs(operand) => write!(f, "|{}|", operand),
            Node::Negate(operand) => {
                let needs_parens = matches!(**operand, Node::Add(_, _) | Node::Subtract(_, _));
                if needs_parens {
                    write!(f, "-({})", operand)
                } else {
                    write!(f, "-{}", operand)
                }
            }
            Node::Greater(left, right) => write!(f, "{} > {}", left, right),
            Node::Less(left, right) => write!(f, "{} < {}", left, right),
            Node::GreaterEqual(left, right) => write!(f, "{} >= {}", left, right),
            Node::LessEqual(left, right) => write!(f, "{} <= {}", left, right),
            Node::Equal(left, right) => write!(f, "{} == {}", left, right),
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

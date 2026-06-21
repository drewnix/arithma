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
    pub fn contains_variable(&self, var: &str) -> bool {
        match self {
            Node::Num(_) => false,
            Node::Variable(v) => v == var,
            Node::Add(l, r)
            | Node::Subtract(l, r)
            | Node::Multiply(l, r)
            | Node::Divide(l, r)
            | Node::Power(l, r)
            | Node::Greater(l, r)
            | Node::Less(l, r)
            | Node::GreaterEqual(l, r)
            | Node::LessEqual(l, r)
            | Node::Equal(l, r)
            | Node::Equation(l, r) => l.contains_variable(var) || r.contains_variable(var),
            Node::Negate(inner) | Node::Sqrt(inner) | Node::Abs(inner) => {
                inner.contains_variable(var)
            }
            Node::Function(_, args) => args.iter().any(|a| a.contains_variable(var)),
            Node::Piecewise(cases) => cases
                .iter()
                .any(|(e, c)| e.contains_variable(var) || c.contains_variable(var)),
            Node::Summation(idx, start, end, body) => {
                if idx == var {
                    start.contains_variable(var) || end.contains_variable(var)
                } else {
                    start.contains_variable(var)
                        || end.contains_variable(var)
                        || body.contains_variable(var)
                }
            }
        }
    }

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

    /// Format a node as if it were a right child of an Add node at the given precedence.
    /// Used when we synthesize a positive version of a negative Multiply term.
    fn fmt_as_add_right_child(
        child: &Node,
        parent_prec: u8,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        let child_prec = child.precedence();
        if child_prec < parent_prec {
            write!(f, "({})", child)
        } else {
            write!(f, "{}", child)
        }
    }

    /// Check if a node is "variable-like" for implicit multiplication purposes:
    /// variables, powers, sqrt, or named functions.
    fn is_var_like(node: &Node) -> bool {
        matches!(
            node,
            Node::Variable(_) | Node::Power(_, _) | Node::Sqrt(_) | Node::Function(_, _)
        )
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Node::Num(n) => write!(f, "{}", n),
            Node::Variable(v) => {
                if v.chars().count() == 1 {
                    if let Some(latex) = crate::tokenizer::latex_name(v.chars().next().unwrap()) {
                        return write!(f, "{}", latex);
                    }
                }
                write!(f, "{}", v)
            }
            Node::Add(left, right) => {
                self.fmt_child(left, 2, false, f)?;
                match right.as_ref() {
                    Node::Negate(inner) => {
                        write!(f, " - ")?;
                        Node::fmt_as_add_right_child(inner, 2, f)
                    }
                    Node::Multiply(l, _r) => {
                        if let Node::Num(n) = l.as_ref() {
                            if n.is_negative() {
                                write!(f, " - ")?;
                                let abs_n = n.abs();
                                let pos = Node::Multiply(Box::new(Node::Num(abs_n)), _r.clone());
                                Node::fmt_as_add_right_child(&pos, 2, f)
                            } else {
                                write!(f, " + ")?;
                                self.fmt_child(right, 2, true, f)
                            }
                        } else {
                            write!(f, " + ")?;
                            self.fmt_child(right, 2, true, f)
                        }
                    }
                    _ => {
                        write!(f, " + ")?;
                        self.fmt_child(right, 2, true, f)
                    }
                }
            }
            Node::Subtract(left, right) => {
                self.fmt_child(left, 2, false, f)?;
                write!(f, " - ")?;
                self.fmt_child(right, 2, true, f)
            }
            Node::Multiply(left, right) => {
                // Coefficient (number) on left, var-like on right: implicit mul
                if let Node::Num(l) = &**left {
                    if Node::is_var_like(right) {
                        if l.is_one() {
                            return write!(f, "{}", right);
                        }
                        if *l == ExactNum::integer(-1) {
                            return write!(f, "-{}", right);
                        }
                        if l.is_integer() {
                            return write!(f, "{}{}", l, right);
                        }
                    }
                }
                // Coefficient (number) on right, var-like on left: swap for display
                if let Node::Num(r) = &**right {
                    if Node::is_var_like(left) {
                        if r.is_one() {
                            return write!(f, "{}", left);
                        }
                        if *r == ExactNum::integer(-1) {
                            return write!(f, "-{}", left);
                        }
                        if r.is_integer() {
                            return write!(f, "{}{}", r, left);
                        }
                    }
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
                write!(f, "\\{}({})", name, formatted_args)
            }
        }
    }
}

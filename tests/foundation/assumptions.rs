use arithma::assumptions::{Assumption, Assumptions};
use arithma::exact::ExactNum;
use arithma::simplify::Simplifiable;
use arithma::{build_expression_tree, Environment, Node, Tokenizer};

fn simplify_with(expr: &str, assumptions: Assumptions) -> String {
    let mut tokenizer = Tokenizer::new(expr);
    let tokens = tokenizer.tokenize();
    let parsed = build_expression_tree(tokens).unwrap();
    let env = Environment::with_assumptions(assumptions);
    let result = parsed.simplify(&env).unwrap_or(parsed);
    format!("{}", result)
}

fn simplify_default(expr: &str) -> String {
    let mut tokenizer = Tokenizer::new(expr);
    let tokens = tokenizer.tokenize();
    let parsed = build_expression_tree(tokens).unwrap();
    let env = Environment::new();
    let result = parsed.simplify(&env).unwrap_or(parsed);
    format!("{}", result)
}

fn simplify_node(node: Node, assumptions: Assumptions) -> Node {
    let env = Environment::with_assumptions(assumptions);
    node.simplify(&env).unwrap_or(node)
}

fn simplify_node_default(node: Node) -> Node {
    let env = Environment::new();
    node.simplify(&env).unwrap_or(node)
}

// === sqrt(x^2) rules ===

#[test]
fn test_sqrt_x2_no_assumptions() {
    let result = simplify_default(r"\sqrt{x^{2}}");
    assert_eq!(result, r"|x|");
}

#[test]
fn test_sqrt_x2_positive() {
    let mut a = Assumptions::new();
    a.assume("x", Assumption::Positive);
    let result = simplify_with(r"\sqrt{x^{2}}", a);
    assert_eq!(result, "x");
}

#[test]
fn test_sqrt_x2_nonnegative() {
    let mut a = Assumptions::new();
    a.assume("x", Assumption::NonNegative);
    let result = simplify_with(r"\sqrt{x^{2}}", a);
    assert_eq!(result, "x");
}

#[test]
fn test_sqrt_x2_no_positive_stays_abs() {
    let mut a = Assumptions::new();
    a.assume("x", Assumption::Real);
    let result = simplify_with(r"\sqrt{x^{2}}", a);
    assert_eq!(result, r"|x|");
}

// === |x| rules ===

#[test]
fn test_abs_x_no_assumptions() {
    let result = simplify_default(r"\left|x\right|");
    assert_eq!(result, r"|x|");
}

#[test]
fn test_abs_x_positive() {
    let mut a = Assumptions::new();
    a.assume("x", Assumption::Positive);
    let result = simplify_with(r"\left|x\right|", a);
    assert_eq!(result, "x");
}

#[test]
fn test_abs_x_nonnegative() {
    let mut a = Assumptions::new();
    a.assume("x", Assumption::NonNegative);
    let result = simplify_with(r"\left|x\right|", a);
    assert_eq!(result, "x");
}

#[test]
fn test_abs_x_negative() {
    let mut a = Assumptions::new();
    a.assume("x", Assumption::Negative);
    let result = simplify_with(r"\left|x\right|", a);
    assert_eq!(result, "-x");
}

// === (-1)^(2n) rules ===

fn neg_one_pow(exp: Node) -> Node {
    Node::Power(Box::new(Node::Num(ExactNum::integer(-1))), Box::new(exp))
}

fn two_times(var: &str) -> Node {
    Node::Multiply(
        Box::new(Node::Num(ExactNum::integer(2))),
        Box::new(Node::Variable(var.to_string())),
    )
}

fn four_times(var: &str) -> Node {
    Node::Multiply(
        Box::new(Node::Num(ExactNum::integer(4))),
        Box::new(Node::Variable(var.to_string())),
    )
}

#[test]
fn test_neg1_pow_2n_no_assumptions() {
    let expr = neg_one_pow(two_times("n"));
    let result = simplify_node_default(expr);
    assert_ne!(result, Node::Num(ExactNum::one()));
}

#[test]
fn test_neg1_pow_2n_integer() {
    let mut a = Assumptions::new();
    a.assume("n", Assumption::Integer);
    let expr = neg_one_pow(two_times("n"));
    let result = simplify_node(expr, a);
    assert_eq!(result, Node::Num(ExactNum::one()));
}

#[test]
fn test_neg1_pow_4n_integer() {
    let mut a = Assumptions::new();
    a.assume("n", Assumption::Integer);
    let expr = neg_one_pow(four_times("n"));
    let result = simplify_node(expr, a);
    assert_eq!(result, Node::Num(ExactNum::one()));
}

#[test]
fn test_neg1_pow_2n_not_integer() {
    let mut a = Assumptions::new();
    a.assume("n", Assumption::Real);
    let expr = neg_one_pow(two_times("n"));
    let result = simplify_node(expr, a);
    assert_ne!(result, Node::Num(ExactNum::one()));
}

#[test]
fn test_neg1_pow_even_numeric() {
    let expr = neg_one_pow(Node::Num(ExactNum::integer(4)));
    let result = simplify_node_default(expr);
    assert_eq!(result, Node::Num(ExactNum::one()));
}

// === Assumptions don't leak between variables ===

#[test]
fn test_assumptions_scoped_to_variable() {
    let mut a = Assumptions::new();
    a.assume("x", Assumption::Positive);
    let result = simplify_with(r"\left|y\right|", a);
    assert_eq!(result, r"|y|");
}

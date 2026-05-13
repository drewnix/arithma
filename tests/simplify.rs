#[cfg(test)]
mod test_simplify {
    use arithma::exact::ExactNum;
    use arithma::simplify::Simplifiable;
    use arithma::{Environment, Evaluator, Node}; // Import the trait

    #[test]
    fn test_simplify_addition() {
        let env = Environment::new();
        let expr = Node::Add(
            Box::new(Node::Num(ExactNum::from_f64(2.0))),
            Box::new(Node::Num(ExactNum::from_f64(2.0))),
        );
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::from_f64(4.0)));
    }

    #[test]
    fn test_simplify_fraction() {
        let env = Environment::new();
        let expr = Node::Num(ExactNum::rational(6, 8));
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        // BigRational automatically reduces 6/8 to 3/4
        assert_eq!(simplified, Node::Num(ExactNum::rational(3, 4)));
    }

    #[test]
    fn test_simplify_fraction_to_integer() {
        let env = Environment::new();
        let expr = Node::Num(ExactNum::rational(8, 4));
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::integer(2))); // 8/4 simplifies to 2
    }

    #[test]
    fn test_multiply_rational() {
        let env = Environment::new();
        let expr = Node::Multiply(
            Box::new(Node::Num(ExactNum::rational(2, 3))),
            Box::new(Node::Num(ExactNum::rational(3, 4))),
        );
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::rational(1, 2))); // (2/3) * (3/4) = 6/12 = 1/2
    }

    #[test]
    fn test_multiply_by_zero() {
        let env = Environment::new();
        let expr = Node::Multiply(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Num(ExactNum::zero())),
        );
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::zero()));
    }

    #[test]
    fn test_multiply_by_one() {
        let env = Environment::new();
        let expr = Node::Multiply(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Num(ExactNum::one())),
        );
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Variable("x".to_string()));
    }

    #[test]
    fn test_divide_by_one() {
        let env = Environment::new();
        let expr = Node::Divide(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Num(ExactNum::one())),
        );
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Variable("x".to_string()));
    }

    #[test]
    fn test_exponentiation_by_zero() {
        let env = Environment::new();
        let expr = Node::Power(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Num(ExactNum::zero())),
        );
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::one()));
    }

    #[test]
    fn test_exponentiation_by_one() {
        let env = Environment::new();
        let expr = Node::Power(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Num(ExactNum::one())),
        );
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Variable("x".to_string()));
    }

    #[test]
    fn test_divide_by_zero_in_rational() {
        let env = Environment::new();
        let expr = Node::Num(ExactNum::rational(5, 0));
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        // ExactNum::rational(5, 0) returns ExactNum::Float(NaN)
        if let Node::Num(n) = &simplified {
            assert!(n.to_f64().is_nan());
        } else {
            panic!("Expected Node::Num with NaN");
        }
    }

    #[test]
    fn test_simplify_like_terms() {
        let expr = Node::Add(
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::from_f64(2.0))),
                Box::new(Node::Variable("x".to_string())),
            )),
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::from_f64(3.0))),
                Box::new(Node::Variable("x".to_string())),
            )),
        );

        let simplified = expr.simplify(&Environment::new()).unwrap();
        assert_eq!(simplified.to_string(), "5x");
    }

    #[test]
    fn test_simplify_like_terms_multiple() {
        let env = Environment::new();
        // Expression: 5x + 3x + 10y + 15y + 10x
        let expr = Node::Add(
            Box::new(Node::Add(
                Box::new(Node::Add(
                    Box::new(Node::Multiply(
                        Box::new(Node::Num(ExactNum::from_f64(5.0))),
                        Box::new(Node::Variable("x".to_string())),
                    )),
                    Box::new(Node::Multiply(
                        Box::new(Node::Num(ExactNum::from_f64(3.0))),
                        Box::new(Node::Variable("x".to_string())),
                    )),
                )),
                Box::new(Node::Add(
                    Box::new(Node::Multiply(
                        Box::new(Node::Num(ExactNum::from_f64(10.0))),
                        Box::new(Node::Variable("y".to_string())),
                    )),
                    Box::new(Node::Multiply(
                        Box::new(Node::Num(ExactNum::from_f64(15.0))),
                        Box::new(Node::Variable("y".to_string())),
                    )),
                )),
            )),
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::from_f64(10.0))),
                Box::new(Node::Variable("x".to_string())),
            )),
        );

        let simplified = expr.simplify(&env).unwrap();

        // Sort the terms in both simplified and expected expressions for consistent comparison
        fn sorted_expr(node: &Node) -> Node {
            match node {
                Node::Add(left, right) => {
                    let mut left = sorted_expr(left);
                    let mut right = sorted_expr(right);

                    // Sort the terms by their structure to ensure deterministic ordering
                    if format!("{:?}", left) > format!("{:?}", right) {
                        std::mem::swap(&mut left, &mut right);
                    }

                    Node::Add(Box::new(left), Box::new(right))
                }
                _ => node.clone(),
            }
        }

        // Expected: 18x + 25y
        let expected = Node::Add(
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::from_f64(18.0))),
                Box::new(Node::Variable("x".to_string())),
            )),
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::from_f64(25.0))),
                Box::new(Node::Variable("y".to_string())),
            )),
        );

        assert_eq!(sorted_expr(&simplified), sorted_expr(&expected));
    }

    #[test]
    fn test_simplify_polynomial_like_terms() {
        let env = Environment::new();
        // x^2 + 3x + x^2 + 2x + 1 should simplify to 2x^2 + 5x + 1
        let expr = Node::Add(
            Box::new(Node::Add(
                Box::new(Node::Power(
                    Box::new(Node::Variable("x".to_string())),
                    Box::new(Node::Num(ExactNum::integer(2))),
                )),
                Box::new(Node::Multiply(
                    Box::new(Node::Num(ExactNum::integer(3))),
                    Box::new(Node::Variable("x".to_string())),
                )),
            )),
            Box::new(Node::Add(
                Box::new(Node::Add(
                    Box::new(Node::Power(
                        Box::new(Node::Variable("x".to_string())),
                        Box::new(Node::Num(ExactNum::integer(2))),
                    )),
                    Box::new(Node::Multiply(
                        Box::new(Node::Num(ExactNum::integer(2))),
                        Box::new(Node::Variable("x".to_string())),
                    )),
                )),
                Box::new(Node::Num(ExactNum::integer(1))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        let mut result_env = Environment::new();
        result_env.set("x", 10.0);
        let val = Evaluator::evaluate(&simplified, &result_env).unwrap();
        assert_eq!(val, 251.0, "2(100) + 5(10) + 1 = 251");
    }
}

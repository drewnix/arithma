#[cfg(test)]
mod test_simplify {
    use arithma::simplify::Simplifiable;
    use arithma::{Environment, Evaluator, Node}; // Import the trait

    #[test]
    fn test_simplify_addition() {
        let env = Environment::new();
        let expr = Node::Add(Box::new(Node::Number(2.0)), Box::new(Node::Number(2.0)));
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Number(4.0));
    }

    #[test]
    fn test_simplify_fraction() {
        let env = Environment::new();
        let expr = Node::Rational(6, 8);
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Rational(3, 4));
    }

    #[test]
    fn test_simplify_fraction_to_integer() {
        let env = Environment::new();
        let expr = Node::Rational(8, 4);
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Number(2.0)); // 8/4 simplifies to 2
    }

    #[test]
    fn test_multiply_rational() {
        let env = Environment::new();
        let expr = Node::Multiply(
            Box::new(Node::Rational(2, 3)),
            Box::new(Node::Rational(3, 4)),
        );
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Rational(1, 2)); // (2/3) * (3/4) = 6/12 = 1/2
    }

    #[test]
    fn test_multiply_by_zero() {
        let env = Environment::new();
        let expr = Node::Multiply(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Number(0.0)),
        );
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Number(0.0));
    }

    #[test]
    fn test_multiply_by_one() {
        let env = Environment::new();
        let expr = Node::Multiply(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Number(1.0)),
        );
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Variable("x".to_string()));
    }

    #[test]
    fn test_divide_by_one() {
        let env = Environment::new();
        let expr = Node::Divide(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Number(1.0)),
        );
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Variable("x".to_string()));
    }

    #[test]
    fn test_exponentiation_by_zero() {
        let env = Environment::new();
        let expr = Node::Power(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Number(0.0)),
        );
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Number(1.0));
    }

    #[test]
    fn test_exponentiation_by_one() {
        let env = Environment::new();
        let expr = Node::Power(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Number(1.0)),
        );
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert_eq!(simplified, Node::Variable("x".to_string()));
    }

    #[test]
    fn test_divide_by_zero_in_rational() {
        let env = Environment::new();
        let expr = Node::Rational(5, 0);
        let simplified = Evaluator::simplify(&expr, &env).unwrap();
        assert!(matches!(simplified, Node::Number(n) if n.is_nan()));
    }

    #[test]
    fn test_simplify_like_terms() {
        let expr = Node::Add(
            Box::new(Node::Multiply(
                Box::new(Node::Number(2.0)),
                Box::new(Node::Variable("x".to_string())),
            )),
            Box::new(Node::Multiply(
                Box::new(Node::Number(3.0)),
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
                        Box::new(Node::Number(5.0)),
                        Box::new(Node::Variable("x".to_string())),
                    )),
                    Box::new(Node::Multiply(
                        Box::new(Node::Number(3.0)),
                        Box::new(Node::Variable("x".to_string())),
                    )),
                )),
                Box::new(Node::Add(
                    Box::new(Node::Multiply(
                        Box::new(Node::Number(10.0)),
                        Box::new(Node::Variable("y".to_string())),
                    )),
                    Box::new(Node::Multiply(
                        Box::new(Node::Number(15.0)),
                        Box::new(Node::Variable("y".to_string())),
                    )),
                )),
            )),
            Box::new(Node::Multiply(
                Box::new(Node::Number(10.0)),
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
                Box::new(Node::Number(18.0)),
                Box::new(Node::Variable("x".to_string())),
            )),
            Box::new(Node::Multiply(
                Box::new(Node::Number(25.0)),
                Box::new(Node::Variable("y".to_string())),
            )),
        );

        assert_eq!(sorted_expr(&simplified), sorted_expr(&expected));
    }
}

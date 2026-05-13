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

    #[test]
    fn test_simplify_rational_expression() {
        let env = Environment::new();
        // (x^2 - 1) / (x + 1) should simplify to x - 1
        let expr = Node::Divide(
            Box::new(Node::Subtract(
                Box::new(Node::Power(
                    Box::new(Node::Variable("x".to_string())),
                    Box::new(Node::Num(ExactNum::integer(2))),
                )),
                Box::new(Node::Num(ExactNum::integer(1))),
            )),
            Box::new(Node::Add(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(1))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        let mut test_env = Environment::new();
        test_env.set("x", 5.0);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        assert_eq!(val, 4.0, "(x^2-1)/(x+1) at x=5 should be 4");
    }

    #[test]
    fn test_simplify_rational_expression_via_latex() {
        let env = Environment::new();
        let expr = arithma::parse_latex("\\frac{x^{2} - 1}{x + 1}", &env).unwrap();
        let mut test_env = Environment::new();
        test_env.set("x", 7.0);
        let val = Evaluator::evaluate(&expr, &test_env).unwrap();
        assert_eq!(val, 6.0, "(x^2-1)/(x+1) at x=7 should be 6");
    }

    #[test]
    fn test_subtract_cancellation() {
        let env = Environment::new();
        // x - x = 0
        let expr = Node::Subtract(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Variable("x".to_string())),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::zero()));
    }

    #[test]
    fn test_subtract_polynomial_normalization() {
        let env = Environment::new();
        // 2x^2 - x^2 = x^2
        let expr = Node::Subtract(
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::integer(2))),
                Box::new(Node::Power(
                    Box::new(Node::Variable("x".to_string())),
                    Box::new(Node::Num(ExactNum::integer(2))),
                )),
            )),
            Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(format!("{}", simplified), "x^{2}");
    }

    #[test]
    fn test_subtract_zero_identity() {
        let env = Environment::new();
        // x - 0 = x
        let expr = Node::Subtract(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Num(ExactNum::zero())),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Variable("x".to_string()));
    }

    #[test]
    fn test_multiply_variable_by_itself() {
        let env = Environment::new();
        // x * x = x^2
        let expr = Node::Multiply(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Variable("x".to_string())),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(format!("{}", simplified), "x^{2}");
    }

    #[test]
    fn test_multiply_polynomials() {
        let env = Environment::new();
        // (x + 1) * (x - 1) = x^2 - 1
        let expr = Node::Multiply(
            Box::new(Node::Add(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::one())),
            )),
            Box::new(Node::Subtract(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::one())),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        let mut test_env = Environment::new();
        test_env.set("x", 5.0);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        assert_eq!(val, 24.0, "(x+1)(x-1) at x=5 = 24");
        assert_eq!(format!("{}", simplified), "x^{2} - 1");
    }

    #[test]
    fn test_power_of_power() {
        let env = Environment::new();
        // (x^2)^3 = x^6
        let expr = Node::Power(
            Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
            Box::new(Node::Num(ExactNum::integer(3))),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(format!("{}", simplified), "x^{6}");
    }

    #[test]
    fn test_subtract_multivar_like_terms() {
        let env = Environment::new();
        // 5x + 3y - 2x = 3x + 3y
        let expr = Node::Subtract(
            Box::new(Node::Add(
                Box::new(Node::Multiply(
                    Box::new(Node::Num(ExactNum::integer(5))),
                    Box::new(Node::Variable("x".to_string())),
                )),
                Box::new(Node::Multiply(
                    Box::new(Node::Num(ExactNum::integer(3))),
                    Box::new(Node::Variable("y".to_string())),
                )),
            )),
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::integer(2))),
                Box::new(Node::Variable("x".to_string())),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        let mut test_env = Environment::new();
        test_env.set("x", 10.0);
        test_env.set("y", 5.0);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        assert_eq!(val, 45.0, "3*10 + 3*5 = 45");
    }

    #[test]
    fn test_double_negation() {
        let env = Environment::new();
        // --x = x
        let expr = Node::Negate(Box::new(Node::Negate(Box::new(Node::Variable(
            "x".to_string(),
        )))));
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Variable("x".to_string()));
    }

    #[test]
    fn test_multiply_distributes_constant() {
        let env = Environment::new();
        // 3 * (x + 2) = 3x + 6
        let expr = Node::Multiply(
            Box::new(Node::Num(ExactNum::integer(3))),
            Box::new(Node::Add(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        let mut test_env = Environment::new();
        test_env.set("x", 4.0);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        assert_eq!(val, 18.0, "3*(x+2) at x=4 = 18");
        assert_eq!(format!("{}", simplified), "3x + 6");
    }

    #[test]
    fn test_power_addition_same_base() {
        let env = Environment::new();
        // x^2 * x^3 → x^5
        let expr = Node::Multiply(
            Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
            Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(3))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(format!("{}", simplified), "x^{5}");
    }

    #[test]
    fn test_power_addition_var_times_power() {
        let env = Environment::new();
        // x * x^3 → x^4
        let expr = Node::Multiply(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(3))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(format!("{}", simplified), "x^{4}");
    }

    #[test]
    fn test_power_addition_power_times_var() {
        let env = Environment::new();
        // x^2 * x → x^3
        let expr = Node::Multiply(
            Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
            Box::new(Node::Variable("x".to_string())),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(format!("{}", simplified), "x^{3}");
    }

    #[test]
    fn test_power_addition_function_base() {
        let env = Environment::new();
        // sin(x)^2 * sin(x)^3 → sin(x)^5
        let sin_x = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
        let expr = Node::Multiply(
            Box::new(Node::Power(
                Box::new(sin_x.clone()),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
            Box::new(Node::Power(
                Box::new(sin_x),
                Box::new(Node::Num(ExactNum::integer(3))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(format!("{}", simplified), "\\sin(x)^{5}");
    }

    #[test]
    fn test_pythagorean_identity() {
        let env = Environment::new();
        // sin²(x) + cos²(x) → 1
        let sin_x = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
        let cos_x = Node::Function("cos".to_string(), vec![Node::Variable("x".to_string())]);
        let expr = Node::Add(
            Box::new(Node::Power(
                Box::new(sin_x),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
            Box::new(Node::Power(
                Box::new(cos_x),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::integer(1)));
    }

    #[test]
    fn test_pythagorean_identity_reversed() {
        let env = Environment::new();
        // cos²(x) + sin²(x) → 1
        let sin_x = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
        let cos_x = Node::Function("cos".to_string(), vec![Node::Variable("x".to_string())]);
        let expr = Node::Add(
            Box::new(Node::Power(
                Box::new(cos_x),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
            Box::new(Node::Power(
                Box::new(sin_x),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::integer(1)));
    }

    #[test]
    fn test_pythagorean_different_args() {
        let env = Environment::new();
        // sin²(x) + cos²(y) should NOT simplify
        let sin_x = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
        let cos_y = Node::Function("cos".to_string(), vec![Node::Variable("y".to_string())]);
        let expr = Node::Add(
            Box::new(Node::Power(
                Box::new(sin_x),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
            Box::new(Node::Power(
                Box::new(cos_y),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_ne!(simplified, Node::Num(ExactNum::integer(1)));
    }

    #[test]
    fn test_pythagorean_with_coefficient() {
        let env = Environment::new();
        // 3·sin²(x) + 3·cos²(x) → 3
        let sin_x = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
        let cos_x = Node::Function("cos".to_string(), vec![Node::Variable("x".to_string())]);
        let expr = Node::Add(
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::integer(3))),
                Box::new(Node::Power(
                    Box::new(sin_x),
                    Box::new(Node::Num(ExactNum::integer(2))),
                )),
            )),
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::integer(3))),
                Box::new(Node::Power(
                    Box::new(cos_x),
                    Box::new(Node::Num(ExactNum::integer(2))),
                )),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::integer(3)));
    }

    #[test]
    fn test_pythagorean_one_minus_sin_sq() {
        let env = Environment::new();
        // 1 - sin²(x) → cos²(x)
        let sin_x = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
        let expr = Node::Subtract(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Power(
                Box::new(sin_x),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(format!("{}", simplified), "\\cos(x)^{2}");
    }

    #[test]
    fn test_pythagorean_one_minus_cos_sq() {
        let env = Environment::new();
        // 1 - cos²(x) → sin²(x)
        let cos_x = Node::Function("cos".to_string(), vec![Node::Variable("x".to_string())]);
        let expr = Node::Subtract(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Power(
                Box::new(cos_x),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(format!("{}", simplified), "\\sin(x)^{2}");
    }

    #[test]
    fn test_pythagorean_sin_sq_minus_one() {
        let env = Environment::new();
        // sin²(x) - 1 → -cos²(x)
        let sin_x = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
        let expr = Node::Subtract(
            Box::new(Node::Power(
                Box::new(sin_x),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
            Box::new(Node::Num(ExactNum::integer(1))),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(format!("{}", simplified), "-\\cos(x)^{2}");
    }

    #[test]
    fn test_function_latex_display() {
        let sin_x = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
        assert_eq!(format!("{}", sin_x), "\\sin(x)");

        let cos_x = Node::Function("cos".to_string(), vec![Node::Variable("x".to_string())]);
        assert_eq!(format!("{}", cos_x), "\\cos(x)");

        let ln_x = Node::Function("ln".to_string(), vec![Node::Variable("x".to_string())]);
        assert_eq!(format!("{}", ln_x), "\\ln(x)");
    }

    #[test]
    fn test_trig_constant_folding() {
        let env = Environment::new();
        // sin(0) → 0
        let expr = Node::Function("sin".to_string(), vec![Node::Num(ExactNum::zero())]);
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::zero()));

        // cos(0) → 1
        let expr = Node::Function("cos".to_string(), vec![Node::Num(ExactNum::zero())]);
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::one()));
    }

    #[test]
    fn test_ln_constant_folding() {
        let env = Environment::new();
        // ln(1) → 0
        let expr = Node::Function("ln".to_string(), vec![Node::Num(ExactNum::one())]);
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::zero()));
    }

    #[test]
    fn test_divide_same_expr() {
        let env = Environment::new();
        // x / x → 1
        let expr = Node::Divide(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Variable("x".to_string())),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::integer(1)));
    }

    #[test]
    fn test_divide_power_subtraction() {
        let env = Environment::new();
        // x^5 / x^3 → x^2
        let expr = Node::Divide(
            Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(5))),
            )),
            Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(3))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(format!("{}", simplified), "x^{2}");
    }

    #[test]
    fn test_divide_power_to_one() {
        let env = Environment::new();
        // x^3 / x^2 → x
        let expr = Node::Divide(
            Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(3))),
            )),
            Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(2))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(format!("{}", simplified), "x");
    }

    #[test]
    fn test_divide_power_by_base() {
        let env = Environment::new();
        // x^3 / x → x^2
        let expr = Node::Divide(
            Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(3))),
            )),
            Box::new(Node::Variable("x".to_string())),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(format!("{}", simplified), "x^{2}");
    }

    #[test]
    fn test_divide_base_by_power() {
        let env = Environment::new();
        // x / x^3 → x^{-2}
        let expr = Node::Divide(
            Box::new(Node::Variable("x".to_string())),
            Box::new(Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(3))),
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(format!("{}", simplified), "x^{-2}");
    }
}

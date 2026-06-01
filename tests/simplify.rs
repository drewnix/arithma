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
    fn test_ln_of_e_to_x() {
        let env = Environment::new();
        // ln(e^x) → x
        let e = Node::Num(ExactNum::from_f64(std::f64::consts::E));
        let expr = Node::Function(
            "ln".to_string(),
            vec![Node::Power(
                Box::new(e),
                Box::new(Node::Variable("x".to_string())),
            )],
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Variable("x".to_string()));
    }

    #[test]
    fn test_exp_of_ln_x() {
        let env = Environment::new();
        // exp(ln(x)) → x
        let expr = Node::Function(
            "exp".to_string(),
            vec![Node::Function(
                "ln".to_string(),
                vec![Node::Variable("x".to_string())],
            )],
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Variable("x".to_string()));
    }

    #[test]
    fn test_sqrt_of_x_squared() {
        let env = Environment::new();
        // sqrt(x²) → |x|
        let expr = Node::Function(
            "sqrt".to_string(),
            vec![Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(2))),
            )],
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(
            simplified,
            Node::Abs(Box::new(Node::Variable("x".to_string())))
        );
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
    fn test_ln_of_power() {
        let env = Environment::new();
        // ln(x^3) → 3·ln(x)
        let expr = Node::Function(
            "ln".to_string(),
            vec![Node::Power(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Num(ExactNum::integer(3))),
            )],
        );
        let simplified = expr.simplify(&env).unwrap();
        let mut test_env = Environment::new();
        test_env.set("x", 2.0);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        assert!((val - 3.0 * 2.0_f64.ln()).abs() < 1e-10);
    }

    #[test]
    fn test_ln_of_product() {
        let env = Environment::new();
        // ln(x * y) → ln(x) + ln(y)
        let expr = Node::Function(
            "ln".to_string(),
            vec![Node::Multiply(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Variable("y".to_string())),
            )],
        );
        let simplified = expr.simplify(&env).unwrap();
        assert!(matches!(simplified, Node::Add(_, _)));
    }

    #[test]
    fn test_ln_of_quotient() {
        let env = Environment::new();
        // ln(x / y) → ln(x) - ln(y)
        let expr = Node::Function(
            "ln".to_string(),
            vec![Node::Divide(
                Box::new(Node::Variable("x".to_string())),
                Box::new(Node::Variable("y".to_string())),
            )],
        );
        let simplified = expr.simplify(&env).unwrap();
        assert!(matches!(simplified, Node::Subtract(_, _)));
    }

    #[test]
    fn test_sin_div_cos() {
        let env = Environment::new();
        // sin(x) / cos(x) → tan(x)
        let sin_x = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
        let cos_x = Node::Function("cos".to_string(), vec![Node::Variable("x".to_string())]);
        let expr = Node::Divide(Box::new(sin_x), Box::new(cos_x));
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(
            simplified,
            Node::Function("tan".to_string(), vec![Node::Variable("x".to_string())])
        );
    }

    #[test]
    fn test_cos_div_sin() {
        let env = Environment::new();
        // cos(x) / sin(x) → cot(x)
        let sin_x = Node::Function("sin".to_string(), vec![Node::Variable("x".to_string())]);
        let cos_x = Node::Function("cos".to_string(), vec![Node::Variable("x".to_string())]);
        let expr = Node::Divide(Box::new(cos_x), Box::new(sin_x));
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(
            simplified,
            Node::Function("cot".to_string(), vec![Node::Variable("x".to_string())])
        );
    }

    #[test]
    fn test_sin_odd_function() {
        let env = Environment::new();
        // sin(-x) → -sin(x)
        let expr = Node::Function(
            "sin".to_string(),
            vec![Node::Negate(Box::new(Node::Variable("x".to_string())))],
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(
            simplified,
            Node::Negate(Box::new(Node::Function(
                "sin".to_string(),
                vec![Node::Variable("x".to_string())]
            )))
        );
    }

    #[test]
    fn test_cos_even_function() {
        let env = Environment::new();
        // cos(-x) → cos(x)
        let expr = Node::Function(
            "cos".to_string(),
            vec![Node::Negate(Box::new(Node::Variable("x".to_string())))],
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(
            simplified,
            Node::Function("cos".to_string(), vec![Node::Variable("x".to_string())])
        );
    }

    #[test]
    fn test_abs_numeric() {
        let env = Environment::new();
        // |-5| → 5
        let expr = Node::Abs(Box::new(Node::Num(ExactNum::integer(-5))));
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::integer(5)));
    }

    #[test]
    fn test_abs_negate() {
        let env = Environment::new();
        // |-x| → |x|
        let expr = Node::Abs(Box::new(Node::Negate(Box::new(Node::Variable(
            "x".to_string(),
        )))));
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(
            simplified,
            Node::Abs(Box::new(Node::Variable("x".to_string())))
        );
    }

    #[test]
    fn test_abs_idempotent() {
        let env = Environment::new();
        // ||x|| → |x|
        let expr = Node::Abs(Box::new(Node::Abs(Box::new(Node::Variable(
            "x".to_string(),
        )))));
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(
            simplified,
            Node::Abs(Box::new(Node::Variable("x".to_string())))
        );
    }

    #[test]
    fn test_one_over_sin() {
        let env = Environment::new();
        // 1 / sin(x) → csc(x)
        let expr = Node::Divide(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Function(
                "sin".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(
            simplified,
            Node::Function("csc".to_string(), vec![Node::Variable("x".to_string())])
        );
    }

    #[test]
    fn test_one_over_cos() {
        let env = Environment::new();
        // 1 / cos(x) → sec(x)
        let expr = Node::Divide(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Function(
                "cos".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(
            simplified,
            Node::Function("sec".to_string(), vec![Node::Variable("x".to_string())])
        );
    }

    #[test]
    fn test_one_over_tan() {
        let env = Environment::new();
        // 1 / tan(x) → cot(x)
        let expr = Node::Divide(
            Box::new(Node::Num(ExactNum::integer(1))),
            Box::new(Node::Function(
                "tan".to_string(),
                vec![Node::Variable("x".to_string())],
            )),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(
            simplified,
            Node::Function("cot".to_string(), vec![Node::Variable("x".to_string())])
        );
    }

    #[test]
    fn test_zero_to_power() {
        let env = Environment::new();
        // 0^5 → 0
        let expr = Node::Power(
            Box::new(Node::Num(ExactNum::zero())),
            Box::new(Node::Num(ExactNum::integer(5))),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::zero()));
    }

    #[test]
    fn test_one_to_power() {
        let env = Environment::new();
        // 1^x → 1
        let expr = Node::Power(
            Box::new(Node::Num(ExactNum::one())),
            Box::new(Node::Variable("x".to_string())),
        );
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Num(ExactNum::one()));
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

    // --- Multivariate simplification tests ---

    #[test]
    fn test_multivariate_polynomial_normalize() {
        let env = Environment::new();
        // (x + y) + (x + y) → 2x + 2y
        let x = Node::Variable("x".to_string());
        let y = Node::Variable("y".to_string());
        let xy = Node::Add(Box::new(x.clone()), Box::new(y.clone()));
        let expr = Node::Add(Box::new(xy.clone()), Box::new(xy.clone()));
        let simplified = expr.simplify(&env).unwrap();
        let val = Evaluator::evaluate_exact(&simplified, &{
            let mut e = Environment::new();
            e.set_exact("x", ExactNum::integer(3));
            e.set_exact("y", ExactNum::integer(7));
            e
        });
        // 2*3 + 2*7 = 20
        assert_eq!(val.unwrap(), ExactNum::integer(20));
    }

    #[test]
    fn test_multivariate_gcd_cancellation() {
        let env = Environment::new();
        // (x*y + x) / (y + 1) → x since x*y + x = x(y+1)
        let x = Node::Variable("x".to_string());
        let y = Node::Variable("y".to_string());
        let numer = Node::Add(
            Box::new(Node::Multiply(Box::new(x.clone()), Box::new(y.clone()))),
            Box::new(x.clone()),
        ); // xy + x
        let denom = Node::Add(
            Box::new(y.clone()),
            Box::new(Node::Num(ExactNum::integer(1))),
        ); // y + 1
        let expr = Node::Divide(Box::new(numer), Box::new(denom));
        let simplified = expr.simplify(&env).unwrap();
        assert_eq!(simplified, Node::Variable("x".to_string()));
    }

    #[test]
    fn test_multivariate_partial_cancellation() {
        let env = Environment::new();
        // (x^2 - y^2) / (x + y) → x - y since x^2-y^2 = (x+y)(x-y)
        let x = Node::Variable("x".to_string());
        let y = Node::Variable("y".to_string());
        let x2 = Node::Power(
            Box::new(x.clone()),
            Box::new(Node::Num(ExactNum::integer(2))),
        );
        let y2 = Node::Power(
            Box::new(y.clone()),
            Box::new(Node::Num(ExactNum::integer(2))),
        );
        let numer = Node::Subtract(Box::new(x2), Box::new(y2));
        let denom = Node::Add(Box::new(x.clone()), Box::new(y.clone()));
        let expr = Node::Divide(Box::new(numer), Box::new(denom));
        let simplified = expr.simplify(&env).unwrap();
        // Should be x - y. Verify by evaluation.
        let val = Evaluator::evaluate_exact(&simplified, &{
            let mut e = Environment::new();
            e.set_exact("x", ExactNum::integer(7));
            e.set_exact("y", ExactNum::integer(3));
            e
        });
        assert_eq!(val.unwrap(), ExactNum::integer(4)); // 7 - 3 = 4
    }

    #[test]
    fn test_combine_fractions_same_denominator_subtract() {
        let env = Environment::new();
        // (2x+4)/(x^2+4x+5) - 1/(x^2+4x+5) → (2x+3)/(x^2+4x+5)
        let expr =
            arithma::parse_latex("\\frac{2x+4}{x^2+4x+5} - \\frac{1}{x^2+4x+5}", &env).unwrap();
        let result = expr.simplify(&env).unwrap();
        let expected = arithma::parse_latex("\\frac{2x+3}{x^2+4x+5}", &env).unwrap();
        let expected_simplified = expected.simplify(&env).unwrap();
        assert_eq!(
            format!("{}", result),
            format!("{}", expected_simplified),
            "Expected (2x+3)/(x^2+4x+5), got {}",
            result
        );
    }

    #[test]
    fn test_combine_fractions_same_denominator_add() {
        let env = Environment::new();
        // 1/(x+1) + x/(x+1) → (1+x)/(x+1) → 1
        let expr = arithma::parse_latex("\\frac{1}{x+1} + \\frac{x}{x+1}", &env).unwrap();
        let result = expr.simplify(&env).unwrap();
        // (1+x)/(x+1) should simplify to 1
        let mut test_env = Environment::new();
        test_env.set("x", 5.0);
        let val = Evaluator::evaluate(&result, &test_env).unwrap();
        assert_eq!(val, 1.0, "1/(x+1) + x/(x+1) at x=5 should be 1");
    }

    // --- Content GCD simplification tests ---

    #[test]
    fn test_content_gcd_simplification() {
        let env = Environment::new();
        // (-32a + 32) / (16a + 8) should reduce coefficients via content GCD
        // GCD of coefficients: numerator has gcd(32,32)=32, denominator has gcd(16,8)=8
        // Overall content GCD = 8, so it should reduce to (-4a + 4) / (2a + 1)
        let expr = arithma::parse_latex("\\frac{-32a + 32}{16a + 8}", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        let display = format!("{}", simplified);
        assert!(
            !display.contains("32") && !display.contains("16"),
            "Expected reduced coefficients (no 32 or 16), got: {}",
            display
        );
        // Verify numerical correctness at a = 0.435
        let mut test_env = Environment::new();
        test_env.set("a", 0.435);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        let expected = (-32.0 * 0.435 + 32.0) / (16.0 * 0.435 + 8.0);
        assert!(
            (val - expected).abs() < 1e-10,
            "Numerical mismatch at a=0.435: got {}, expected {}",
            val,
            expected
        );
    }

    #[test]
    fn test_content_gcd_constant_numerator() {
        let env = Environment::new();
        // 48 / (16a^3 + 24a^2 + 12a + 2) should reduce via content GCD
        // Denominator coefficients gcd(16,24,12,2) = 2, gcd with 48 = 2
        // Reduces to 24 / (8a^3 + 12a^2 + 6a + 1)
        let expr = arithma::parse_latex("\\frac{48}{16a^3 + 24a^2 + 12a + 2}", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        let display = format!("{}", simplified);
        assert!(
            !display.contains("48"),
            "Expected reduced numerator (no 48), got: {}",
            display
        );
        // Verify numerically at a = 0.5
        let mut test_env = Environment::new();
        test_env.set("a", 0.5);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        let expected = 48.0 / (16.0 * 0.125 + 24.0 * 0.25 + 12.0 * 0.5 + 2.0);
        assert!(
            (val - expected).abs() < 1e-10,
            "Numerical mismatch at a=0.5: got {}, expected {}",
            val,
            expected
        );
    }

    #[test]
    fn test_content_gcd_constant_denominator() {
        let env = Environment::new();
        // (6a + 3) / 3 should reduce to 2a + 1 (a polynomial, no fraction)
        let expr = arithma::parse_latex("\\frac{6a + 3}{3}", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        let display = format!("{}", simplified);
        assert!(
            !display.contains("\\frac"),
            "Expected polynomial result (no \\frac), got: {}",
            display
        );
        // Verify numerically at a = 2: (6*2 + 3) / 3 = 15 / 3 = 5
        let mut test_env = Environment::new();
        test_env.set("a", 2.0);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        assert_eq!(val, 5.0, "(6a+3)/3 at a=2 should be 5, got {}", val);
    }

    #[test]
    fn test_content_gcd_both_reduce() {
        let env = Environment::new();
        // (6a^2 - 6) / (4a + 4) should reduce via both polynomial GCD and content GCD
        // 6a^2 - 6 = 6(a^2 - 1) = 6(a+1)(a-1)
        // 4a + 4 = 4(a + 1)
        // After polynomial GCD cancellation of (a+1) and content GCD: (3a - 3) / 2
        let expr = arithma::parse_latex("\\frac{6a^2 - 6}{4a + 4}", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        let display = format!("{}", simplified);
        // The result should be a single clean fraction \frac{3a - 3}{2},
        // not scattered rational coefficients like \frac{3}{2} * a - \frac{3}{2}
        assert!(
            !display.contains('6') && !display.contains('4'),
            "Expected fully reduced coefficients (no 6 or 4), got: {}",
            display
        );
        // The display should show the result as a single fraction with integer coefficients
        // (i.e., content GCD should keep the denominator unified, not distribute it)
        let frac_count = display.matches("\\frac").count();
        assert!(
            frac_count <= 1,
            "Expected at most one \\frac (clean form), got {} in: {}",
            frac_count,
            display
        );
        // Verify numerically at a = 3: (6*9 - 6) / (4*3 + 4) = 48 / 16 = 3.0
        let mut test_env = Environment::new();
        test_env.set("a", 3.0);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        assert_eq!(
            val, 3.0,
            "(6a^2-6)/(4a+4) at a=3 should be 3.0, got {}",
            val
        );
    }

    // --- Factored display tests ---

    #[test]
    fn test_factored_display_cubed() {
        let env = Environment::new();
        // 48 / (16a^3 + 24a^2 + 12a + 2)
        // Content GCD reduces to 24 / (8a^3 + 12a^2 + 6a + 1)
        // The denominator 8a^3 + 12a^2 + 6a + 1 = (2a + 1)^3
        // Factored display should show (2a + 1) in the denominator
        let expr = arithma::parse_latex("\\frac{48}{16a^3 + 24a^2 + 12a + 2}", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        let display = format!("{}", simplified);
        assert!(
            display.contains("(2a + 1)") || display.contains("(1 + 2a)"),
            "Expected factored denominator containing (2a + 1), got: {}",
            display
        );
        // Verify numerically at a = 0.5: 48 / (16*0.125 + 24*0.25 + 12*0.5 + 2) = 48/16 = 3
        let mut test_env = Environment::new();
        test_env.set("a", 0.5);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        let expected = 48.0 / (16.0 * 0.125 + 24.0 * 0.25 + 12.0 * 0.5 + 2.0);
        assert!(
            (val - expected).abs() < 1e-10,
            "Numerical mismatch at a=0.5: got {}, expected {}",
            val,
            expected
        );
    }

    #[test]
    fn test_factored_display_squared() {
        let env = Environment::new();
        // 1 / (a^2 + 2a + 1)
        // The denominator a^2 + 2a + 1 = (a + 1)^2
        // Factored display should show (a + 1) in the denominator
        let expr = arithma::parse_latex("\\frac{1}{a^2 + 2a + 1}", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        let display = format!("{}", simplified);
        assert!(
            display.contains("(a + 1)") || display.contains("(1 + a)"),
            "Expected factored denominator containing (a + 1), got: {}",
            display
        );
        // Verify numerically at a = 2: 1 / (4 + 4 + 1) = 1/9
        let mut test_env = Environment::new();
        test_env.set("a", 2.0);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        let expected = 1.0 / 9.0;
        assert!(
            (val - expected).abs() < 1e-10,
            "Numerical mismatch at a=2: got {}, expected {}",
            val,
            expected
        );
    }

    #[test]
    fn test_factored_display_two_distinct_factors() {
        let env = Environment::new();
        // 1 / (a^2 - 1)
        // The denominator a^2 - 1 = (a - 1)(a + 1)
        // Factored display should show both factors
        let expr = arithma::parse_latex("\\frac{1}{a^2 - 1}", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        let display = format!("{}", simplified);
        assert!(
            (display.contains("(a + 1)") || display.contains("(1 + a)"))
                && (display.contains("(a - 1)") || display.contains("(-1 + a)")),
            "Expected factored denominator with (a - 1)(a + 1), got: {}",
            display
        );
        // Verify numerically at a = 3: 1 / (9 - 1) = 1/8 = 0.125
        let mut test_env = Environment::new();
        test_env.set("a", 3.0);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        assert!(
            (val - 0.125).abs() < 1e-10,
            "Numerical mismatch at a=3: got {}, expected 0.125",
            val
        );
    }

    #[test]
    fn test_factored_display_irreducible_unchanged() {
        let env = Environment::new();
        // 1 / (a^2 + a + 1) is irreducible over Q
        // The denominator should remain as a polynomial (no factoring possible)
        let expr = arithma::parse_latex("\\frac{1}{a^2 + a + 1}", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        // Just verify numerical correctness at a = 2: 1 / (4 + 2 + 1) = 1/7
        let mut test_env = Environment::new();
        test_env.set("a", 2.0);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        let expected = 1.0 / 7.0;
        assert!(
            (val - expected).abs() < 1e-10,
            "Numerical mismatch at a=2: got {}, expected {}",
            val,
            expected
        );
    }

    #[test]
    fn test_multivariate_content_simplification() {
        let env = Environment::new();
        // (6xy + 6x) / (3y + 3) → 2x
        // 6xy + 6x = 6x(y+1), 3y + 3 = 3(y+1)
        // After poly GCD (y+1) and content GCD: 6x/3 = 2x
        let x = Node::Variable("x".to_string());
        let y = Node::Variable("y".to_string());
        let numer = Node::Add(
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::integer(6))),
                Box::new(Node::Multiply(Box::new(x.clone()), Box::new(y.clone()))),
            )),
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::integer(6))),
                Box::new(x.clone()),
            )),
        );
        let denom = Node::Add(
            Box::new(Node::Multiply(
                Box::new(Node::Num(ExactNum::integer(3))),
                Box::new(y.clone()),
            )),
            Box::new(Node::Num(ExactNum::integer(3))),
        );
        let expr = Node::Divide(Box::new(numer), Box::new(denom));
        let simplified = expr.simplify(&env).unwrap();
        let mut test_env = Environment::new();
        test_env.set("x", 5.0);
        test_env.set("y", 7.0);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        assert_eq!(val, 10.0, "6*5*7+6*5=210+30=240, 3*7+3=24, 240/24=10=2*5");
    }

    // --- E2E tests through LaTeX interface ---

    #[test]
    fn test_e2e_alex_eigenvalue_ratio() {
        // R = 4(1-α)/(2α+1) from the companion paper
        // Parse \frac{-32α+32}{16α+8} and verify coefficients are reduced
        let env = Environment::new();
        let expr = arithma::parse_latex("\\frac{-32\\alpha+32}{16\\alpha+8}", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        let display = format!("{}", simplified);
        assert!(
            !display.contains("32") && !display.contains("16"),
            "Expected reduced coefficients (no 32 or 16), got: {}",
            display
        );
        // Verify numerically at α=0.435
        let mut test_env = Environment::new();
        test_env.set("α", 0.435);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        let expected = (-32.0 * 0.435 + 32.0) / (16.0 * 0.435 + 8.0);
        assert!(
            (val - expected).abs() < 1e-10,
            "Numerical mismatch at α=0.435: got {}, expected {}",
            val,
            expected
        );
    }

    #[test]
    fn test_e2e_factored_cubic_denominator() {
        // Parse \frac{48}{16α^3+24α^2+12α+2} with Greek alpha
        // Content GCD reduces to 24/(8α^3+12α^2+6α+1), denominator factors as (2α+1)^3
        let env = Environment::new();
        let expr =
            arithma::parse_latex("\\frac{48}{16\\alpha^3+24\\alpha^2+12\\alpha+2}", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        let display = format!("{}", simplified);
        assert!(
            !display.contains("48"),
            "Expected reduced numerator (no 48), got: {}",
            display
        );
        // Denominator should be factored — output should contain parenthesized factor
        assert!(
            display.contains("(2") && display.contains(")"),
            "Expected factored denominator with (2...+1) form, got: {}",
            display
        );
        // Verify numerically at α=0.5: 48 / (16*0.125 + 24*0.25 + 12*0.5 + 2) = 48/16 = 3
        let mut test_env = Environment::new();
        test_env.set("α", 0.5);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        let expected = 48.0 / (16.0 * 0.125 + 24.0 * 0.25 + 12.0 * 0.5 + 2.0);
        assert!(
            (val - expected).abs() < 1e-10,
            "Numerical mismatch at α=0.5: got {}, expected {}",
            val,
            expected
        );
    }

    #[test]
    fn test_already_simplified_unchanged() {
        // (a+1)/(a+2) — coprime polynomials with content 1
        // Should remain unchanged and evaluate correctly
        let env = Environment::new();
        let expr = arithma::parse_latex("\\frac{a+1}{a+2}", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        // Verify numerically at a=5: (5+1)/(5+2) = 6/7
        let mut test_env = Environment::new();
        test_env.set("a", 5.0);
        let val = Evaluator::evaluate(&simplified, &test_env).unwrap();
        let expected = 6.0 / 7.0;
        assert!(
            (val - expected).abs() < 1e-10,
            "Expected 6/7 at a=5, got {}",
            val
        );
    }

    #[test]
    fn test_no_nested_negations_in_determinant() {
        let env = Environment::new();
        // det([[1,a,b],[c,1,a],[b,a,1]]) should have no nested negations
        let expr = arithma::parse_latex(
            "\\frac{(b - 1) \\cdot a^{2} + (c \\cdot b - c) \\cdot a - -(-b^{2} + 1)}{1}",
            &env,
        )
        .unwrap();
        let simplified = expr.simplify(&env).unwrap();
        let display = format!("{}", simplified);
        assert!(
            !display.contains("- -"),
            "Should not have nested negations '- -', got: {}",
            display
        );
    }

    #[test]
    fn test_subtract_negate_simplifies() {
        let env = Environment::new();
        // x - (-y) → x + y
        let x = Node::Variable("x".to_string());
        let y = Node::Variable("y".to_string());
        let expr = Node::Subtract(Box::new(x), Box::new(Node::Negate(Box::new(y))));
        let simplified = expr.simplify(&env).unwrap();
        let display = format!("{}", simplified);
        assert!(
            display.contains("x + y") || display.contains("y + x"),
            "x - (-y) should simplify to x + y, got: {}",
            display
        );
    }

    #[test]
    fn test_e2e_taylor_symbolic_center() {
        let result =
            arithma::series::taylor_series_latex_symbolic("\\frac{3}{1+2x}", "x", "\\alpha", 1)
                .unwrap();
        assert!(result.contains("\\alpha"), "Should contain α: {}", result);
        let env = Environment::new();
        let expr = arithma::parse_latex(&result, &env).unwrap();
        let mut test_env = Environment::new();
        test_env.set("x", 0.6);
        test_env.set("α", 0.5);
        let val = Evaluator::evaluate(&expr, &test_env).unwrap();
        let exact = 3.0 / (1.0 + 2.0 * 0.6);
        assert!(
            (val - exact).abs() < 0.1,
            "Taylor approx should be close: {} vs {}",
            val,
            exact
        );
    }

    #[test]
    fn test_e2e_taylor_numeric_center_unchanged() {
        let result = arithma::series::taylor_series_latex("\\sin(x)", "x", 0.0, 3).unwrap();
        assert!(!result.is_empty());
        assert!(!result.contains("NaN"));
    }

    #[test]
    fn test_rational_distribution_ada_identity() {
        let env = Environment::new();
        let expr = arithma::parse_latex("2 \\cdot (\\frac{3}{2x+1} - 1)", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        let result = format!("{}", simplified);
        assert!(
            result.contains("-4x + 4") || result.contains("4 - 4x"),
            "Should simplify to (-4x+4)/(2x+1), got: {}",
            result
        );
    }

    #[test]
    fn test_fraction_minus_constant() {
        let env = Environment::new();
        let expr = arithma::parse_latex("\\frac{3}{2x+1} - 1", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        let result = format!("{}", simplified);
        assert!(
            result.contains("\\frac"),
            "Should combine into single fraction, got: {}",
            result
        );
    }

    #[test]
    fn test_different_denominator_combination() {
        let env = Environment::new();
        let expr = arithma::parse_latex("\\frac{1}{x+1} + \\frac{1}{x-1}", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        let result = format!("{}", simplified);
        assert!(
            result.contains("2x"),
            "Should combine to 2x/(...), got: {}",
            result
        );
    }

    #[test]
    fn test_scalar_times_fraction() {
        let env = Environment::new();
        let expr = arithma::parse_latex("x \\cdot \\frac{1}{x+1}", &env).unwrap();
        let simplified = expr.simplify(&env).unwrap();
        let result = format!("{}", simplified);
        assert_eq!(result, "\\frac{x}{x + 1}");
    }

    #[test]
    fn test_simplify_sqrt_12() {
        let env = Environment::new();
        let expr = arithma::parse_latex("\\sqrt{12}", &env).unwrap();
        let result = Evaluator::simplify(&expr, &env).unwrap();
        let s = format!("{}", result);
        assert!(!s.contains('.'), "Should NOT fall back to float: {}", s);
        assert!(s.contains("\\sqrt"), "Should preserve symbolic sqrt: {}", s);
        assert_eq!(s, "2\\sqrt{3}");
    }

    #[test]
    fn test_simplify_sqrt_8() {
        let env = Environment::new();
        let expr = arithma::parse_latex("\\sqrt{8}", &env).unwrap();
        let result = Evaluator::simplify(&expr, &env).unwrap();
        let s = format!("{}", result);
        assert!(!s.contains('.'), "Should NOT fall back to float: {}", s);
        assert_eq!(s, "2\\sqrt{2}");
    }

    #[test]
    fn test_simplify_sqrt_perfect_square() {
        let env = Environment::new();
        let expr = arithma::parse_latex("\\sqrt{4}", &env).unwrap();
        let result = Evaluator::simplify(&expr, &env).unwrap();
        let s = format!("{}", result);
        assert_eq!(s, "2");
    }

    #[test]
    fn test_simplify_sqrt_prime() {
        let env = Environment::new();
        let expr = arithma::parse_latex("\\sqrt{7}", &env).unwrap();
        let result = Evaluator::simplify(&expr, &env).unwrap();
        let s = format!("{}", result);
        assert!(s.contains("\\sqrt"), "sqrt(7) should stay symbolic: {}", s);
        assert!(!s.contains('.'), "Should NOT fall back to float: {}", s);
        assert_eq!(s, "\\sqrt{7}");
    }

    #[test]
    fn test_simplify_sqrt_72() {
        let env = Environment::new();
        let expr = arithma::parse_latex("\\sqrt{72}", &env).unwrap();
        let result = Evaluator::simplify(&expr, &env).unwrap();
        let s = format!("{}", result);
        assert!(!s.contains('.'), "Should NOT be float: {}", s);
        assert!(s.contains("\\sqrt"), "Should have radical: {}", s);
        assert_eq!(s, "6\\sqrt{2}");
    }

    #[test]
    fn test_simplify_sqrt_50() {
        let env = Environment::new();
        let expr = arithma::parse_latex("\\sqrt{50}", &env).unwrap();
        let result = Evaluator::simplify(&expr, &env).unwrap();
        let s = format!("{}", result);
        assert!(!s.contains('.'), "Should NOT be float: {}", s);
        assert_eq!(s, "5\\sqrt{2}");
    }

    #[test]
    fn test_combine_like_radicals_add() {
        let env = Environment::new();
        // √8 + √2 = 2√2 + √2 = 3√2
        let expr = arithma::parse_latex("\\sqrt{8} + \\sqrt{2}", &env).unwrap();
        let result = Evaluator::simplify(&expr, &env).unwrap();
        let s = format!("{}", result);
        assert!(s.contains("3"), "Should have coefficient 3: {}", s);
        assert!(s.contains("\\sqrt{2}"), "Should have √2: {}", s);
        assert!(!s.contains('+'), "Should be combined, no +: {}", s);
    }

    #[test]
    fn test_combine_like_radicals_subtract() {
        let env = Environment::new();
        // √8 - √2 = 2√2 - √2 = √2
        let expr = arithma::parse_latex("\\sqrt{8} - \\sqrt{2}", &env).unwrap();
        let result = Evaluator::simplify(&expr, &env).unwrap();
        let s = format!("{}", result);
        assert_eq!(s, "\\sqrt{2}", "√8 - √2 should be √2: {}", s);
    }

    #[test]
    fn test_combine_like_radicals_cancel() {
        let env = Environment::new();
        // √2 - √2 = 0
        let expr = arithma::parse_latex("\\sqrt{2} - \\sqrt{2}", &env).unwrap();
        let result = Evaluator::simplify(&expr, &env).unwrap();
        let s = format!("{}", result);
        assert_eq!(s, "0", "√2 - √2 should be 0: {}", s);
    }

    #[test]
    fn test_combine_like_radicals_different() {
        let env = Environment::new();
        // √2 + √3 — different radicals, should NOT combine
        let expr = arithma::parse_latex("\\sqrt{2} + \\sqrt{3}", &env).unwrap();
        let result = Evaluator::simplify(&expr, &env).unwrap();
        let s = format!("{}", result);
        assert!(
            s.contains('+'),
            "Different radicals should not combine: {}",
            s
        );
    }

    #[test]
    fn test_negate_in_product_display() {
        use arithma::parse_latex;
        let env = Environment::new();
        let expr = parse_latex("\\sin(x) \\cdot -\\sin(x)", &env).unwrap();
        let result = expr.simplify(&env).unwrap();
        let s = format!("{}", result);
        assert!(
            !s.contains("\\cdot -"),
            "Should not have ·- in output: {}",
            s
        );
        assert!(
            s.contains("-\\sin") || s.contains("-1"),
            "Negation should be extracted: {}",
            s
        );
    }

    #[test]
    fn test_simplify_sqrt_4a_squared() {
        let env = Environment::new();
        let expr = arithma::parse_latex("\\sqrt{4 a^2}", &env).unwrap();
        let result = expr.simplify(&env).unwrap();
        let s = format!("{}", result);
        // Should be 2|a|, not √(4a²)
        assert!(
            !s.contains("\\sqrt"),
            "Should extract all square factors: {}",
            s
        );
        assert!(s.contains("2"), "Should have factor 2: {}", s);
    }

    #[test]
    fn test_simplify_sqrt_4a() {
        let env = Environment::new();
        // √(4a) = 2√a — numeric square factor with symbolic remainder
        let expr = arithma::parse_latex("\\sqrt{4 a}", &env).unwrap();
        let result = expr.simplify(&env).unwrap();
        let s = format!("{}", result);
        assert!(s.contains("2"), "Should have factor 2: {}", s);
        assert!(s.contains("\\sqrt"), "Should have √a remaining: {}", s);
    }

    #[test]
    fn test_simplify_sqrt_9x_squared() {
        let env = Environment::new();
        let expr = arithma::parse_latex("\\sqrt{9 x^2}", &env).unwrap();
        let result = expr.simplify(&env).unwrap();
        let s = format!("{}", result);
        // Should be 3|x|
        assert!(!s.contains("\\sqrt"), "Should fully simplify: {}", s);
        assert!(s.contains("3"), "Should have factor 3: {}", s);
    }

    #[test]
    fn test_simplify_sqrt_mixed_nonneg() {
        use arithma::assumptions::{Assumption, Assumptions};
        // With assumption a >= 0: √(4a²) → 2a
        let mut assumptions = Assumptions::new();
        assumptions.assume("a", Assumption::NonNegative);
        let env_pos = Environment::with_assumptions(assumptions);
        let expr = arithma::parse_latex("\\sqrt{4 a^2}", &env_pos).unwrap();
        let result = expr.simplify(&env_pos).unwrap();
        let s = format!("{}", result);
        assert!(!s.contains("|"), "With a>=0, should be 2a not 2|a|: {}", s);
    }

    #[test]
    fn test_general_fraction_coeff_cancel_sqrt() {
        let env = Environment::new();
        // 2x / (2√a) → x / √a
        let expr = arithma::parse_latex("\\frac{2x}{2\\sqrt{a}}", &env).unwrap();
        let result = expr.simplify(&env).unwrap();
        let s = format!("{}", result);
        assert!(!s.contains("2"), "2/2 should cancel: {}", s);
        assert!(s.contains("x"), "x should remain: {}", s);
        assert!(s.contains("\\sqrt"), "√a should remain: {}", s);
    }

    #[test]
    fn test_general_fraction_coeff_cancel_trig() {
        let env = Environment::new();
        // 6sin(x) / (3cos(x)) → 2tan(x) or 2sin(x)/cos(x)
        let expr = arithma::parse_latex("\\frac{6\\sin(x)}{3\\cos(x)}", &env).unwrap();
        let result = expr.simplify(&env).unwrap();
        let s = format!("{}", result);
        assert!(s.contains("2"), "6/3 should reduce to factor 2: {}", s);
        assert!(!s.contains("6"), "6 should not remain: {}", s);
        assert!(!s.contains("3"), "3 should not remain: {}", s);
    }

    #[test]
    fn test_general_fraction_coeff_cancel_partial() {
        let env = Environment::new();
        // 6·exp(x) / (4·ln(x)) → 3·exp(x) / (2·ln(x))
        let expr = arithma::parse_latex("\\frac{6\\exp(x)}{4\\ln(x)}", &env).unwrap();
        let result = expr.simplify(&env).unwrap();
        let s = format!("{}", result);
        assert!(s.contains("3"), "6/4 should reduce, 3 in numerator: {}", s);
        assert!(!s.contains("6"), "6 should not remain: {}", s);
        assert!(!s.contains("4"), "4 should not remain: {}", s);
    }

    #[test]
    fn test_general_fraction_coeff_cancel_full() {
        let env = Environment::new();
        // 5·exp(x) / (5·x) → exp(x) / x
        let expr = arithma::parse_latex("\\frac{5\\exp(x)}{5x}", &env).unwrap();
        let result = expr.simplify(&env).unwrap();
        let s = format!("{}", result);
        assert!(!s.contains("5"), "5/5 should cancel completely: {}", s);
    }
}

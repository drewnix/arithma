use lazy_static::lazy_static;
use std::collections::HashMap;
type FunctionHandler = Box<dyn Fn(Vec<f64>) -> f64 + Send + Sync + 'static>;

lazy_static! {
    pub static ref LATEX_FUNCTIONS: HashMap<&'static str, (FunctionHandler, usize)> = {
        let mut map = HashMap::new();

        // Unary functions
        map.insert("\\sin", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\sin requires exactly one argument");
            }
            args[0].sin()
        }) as FunctionHandler, 1));  // Explicitly cast to FunctionHandler

        map.insert("\\cos", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\cos requires exactly one argument");
            }
            args[0].cos()
        }) as FunctionHandler, 1));  // Explicitly cast to FunctionHandler

                map.insert("\\tan", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\tan requires exactly one argument");
            }
            args[0].tan()
        }) as FunctionHandler, 1));

        // Hyperbolic functions
        map.insert("\\sinh", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\sinh requires exactly one argument");
            }
            args[0].sinh()
        }) as FunctionHandler, 1));

        map.insert("\\cosh", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\cosh requires exactly one argument");
            }
            args[0].cosh()
        }) as FunctionHandler, 1));

        map.insert("\\tanh", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\tanh requires exactly one argument");
            }
            args[0].tanh()
        }) as FunctionHandler, 1));

        // Inverse trigonometric functions
        map.insert("\\arcsin", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\arcsin requires exactly one argument");
            }
            args[0].asin()
        }) as FunctionHandler, 1));

        map.insert("\\arccos", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\arccos requires exactly one argument");
            }
            args[0].acos()
        }) as FunctionHandler, 1));

        map.insert("\\arctan", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\arctan requires exactly one argument");
            }
            args[0].atan()
        }) as FunctionHandler, 1));

        // Secant: sec(x) = 1 / cos(x)
        map.insert("\\sec", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\sec requires exactly one argument");
            }
            if args[0].cos() == 0.0 {
                return f64::NAN;  // Return NaN for undefined result (cos(x) = 0)
            }
            1.0 / args[0].cos()
        }) as FunctionHandler, 1));

        // Cosecant: csc(x) = 1 / sin(x)
        map.insert("\\csc", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\csc requires exactly one argument");
            }
            if args[0].sin() == 0.0 {
                return f64::NAN;  // Return NaN for undefined result (sin(x) = 0)
            }
            1.0 / args[0].sin()
        }) as FunctionHandler, 1));

        // Hyperbolic Cotangent: coth(x) = 1 / tanh(x)
        map.insert("\\coth", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\coth requires exactly one argument");
            }
            let tanh_val = args[0].tanh();
            if tanh_val == 0.0 {
                return f64::NAN;  // Return NaN for undefined result
            }
            1.0 / tanh_val
        }), 1));

        // Binary functions like \frac
        map.insert("\\frac", (Box::new(|args: Vec<f64>| {
            if args.len() != 2 {
                panic!("\\frac requires exactly two arguments");
            }
            if args[1] == 0.0 {
                panic!("Division by zero in \\frac");
            }
            args[0] / args[1]
        }) as FunctionHandler, 2));  // Explicitly cast to FunctionHandler

        // More functions like \log, \ln, etc.
        map.insert("\\log", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\log requires exactly one argument");
            }
            args[0].log10()
        }) as FunctionHandler, 1));

        // More functions like \log, \ln, etc.
        map.insert("\\ln", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\log requires exactly one argument");
            }
            args[0].ln()
        }) as FunctionHandler, 1));

        map.insert("\\lg", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\log requires exactly one argument");
            }
            args[0].log2()
        }) as FunctionHandler, 1));

        map.insert("\\sqrt", (Box::new(|args: Vec<f64>| {
            if args.len() != 1 {
                panic!("\\sqrt requires exactly one argument");
            }
            args[0].sqrt()
        }) as FunctionHandler, 1));

        // Min: min(x1, x2, ..., xn)
        map.insert("\\min", (Box::new(|args: Vec<f64>| {
            if args.is_empty() {
                panic!("\\min requires at least one argument");
            }
            args.into_iter().fold(f64::INFINITY, |a, b| a.min(b))
        }) as FunctionHandler, 0));  // 0 means variable number of arguments

        // Max: max(x1, x2, ..., xn)
        map.insert("\\max", (Box::new(|args: Vec<f64>| {
            if args.is_empty() {
                panic!("\\max requires at least one argument");
            }
            args.into_iter().fold(f64::NEG_INFINITY, |a, b| a.max(b))
        }) as FunctionHandler, 0));  // 0 means variable number of arguments

        // Determinant: det(x1, x2, ..., xn) (currently treated as product)
        map.insert("\\det", (Box::new(|args: Vec<f64>| {
            if args.is_empty() {
                panic!("\\det requires at least one argument");
            }
            args.into_iter().product()
        }) as FunctionHandler, 0));  // 0 means variable number of arguments

        map
    };
}
// fn handle_sin(args: Vec<Node>, env: &Environment) -> Result<f64, String> {
//     if args.len() != 1 {
//         return Err("sin function requires exactly one argument.".to_string());
//     }
//     let arg_value = Evaluator::evaluate(&args[0], env)?;  // Evaluate the argument
//     Ok(arg_value.sin())
// }
//
// fn handle_cos(args: Vec<Node>, env: &Environment) -> Result<f64, String> {
//     if args.len() != 1 {
//         return Err("cos function requires exactly one argument.".to_string());
//     }
//     let arg_value = Evaluator::evaluate(&args[0], env)?;  // Evaluate the argument
//     Ok(arg_value.cos())
// }
//
// fn handle_solve(args: Vec<Node>, env: &Environment) -> Result<f64, String> {
//     if args.len() != 2 {
//         return Err("solve function requires exactly two arguments.".to_string());
//     }
//
//     let variable_node = &args[0];  // The variable to solve for
//     let equation = &args[1];  // The equation to solve
//
//     // Evaluate the right-hand side of the equation to get the 'right_val'
//     let right_val = Evaluator::evaluate(equation, env)?;  // This evaluates the equation (e.g., 10)
//
//     // Check that the variable is indeed a Variable node
//     let variable_name = if let Node::Variable(var_name) = variable_node {
//         var_name
//     } else {
//         return Err("First argument to solve must be a variable.".to_string());
//     };
//
//     // Now call the solve_for_variable function with all three arguments
//     let solution = solve_for_variable(equation, right_val, variable_name)?;
//
//     Ok(solution)
// }
//
// fn handle_log(args: Vec<Node>, env: &Environment) -> Result<f64, String> {
//     if args.len() != 1 {
//         return Err("log function requires exactly one argument.".to_string());
//     }
//     let arg_value = Evaluator::evaluate(&args[0], env)?;  // Evaluate the argument
//     if arg_value <= 0.0 {
//         return Err("logarithm is undefined for non-positive numbers.".to_string());
//     }
//     Ok(arg_value.log10())  // Base 10 logarithm
// }
//
// fn handle_ln(args: Vec<Node>, env: &Environment) -> Result<f64, String> {
//     if args.len() != 1 {
//         return Err("ln function requires exactly one argument.".to_string());
//     }
//     let arg_value = Evaluator::evaluate(&args[0], env)?;  // Evaluate the argument
//     if arg_value <= 0.0 {
//         return Err("natural logarithm is undefined for non-positive numbers.".to_string());
//     }
//     Ok(arg_value.ln())  // Natural logarithm (base e)
// }
//
// fn handle_sqrt(args: Vec<Node>, env: &Environment) -> Result<f64, String> {
//     if args.len() != 1 {
//         return Err("sqrt function requires exactly one argument.".to_string());
//     }
//     let arg_value = Evaluator::evaluate(&args[0], env)?;  // Evaluate the argument
//     if arg_value < 0.0 {
//         return Err("square root is undefined for negative numbers.".to_string());
//     }
//     Ok(arg_value.sqrt())  // Square root
// }
//
// fn handle_lg(args: Vec<Node>, env: &Environment) -> Result<f64, String> {
//     if args.len() != 1 {
//         return Err("lg function requires exactly one argument.".to_string());
//     }
//     let arg_value = Evaluator::evaluate(&args[0], env)?;  // Evaluate the argument
//     if arg_value <= 0.0 {
//         return Err("binary logarithm is undefined for non-positive numbers.".to_string());
//     }
//     Ok(arg_value.log2())  // Binary logarithm (base 2)
// }

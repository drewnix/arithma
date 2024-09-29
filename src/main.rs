use arithma::simplify::Simplifiable;
use arithma::{build_expression_tree, tokenize, Environment, Evaluator};
use std::io::{self, Write};

fn main() {
    env_logger::init();

    println!("Arithma - Type 'exit' to quit.");
    let env = Environment::new(); // Create an environment for variables

    loop {
        // Prompt
        print!(">> ");
        io::stdout().flush().unwrap();

        // Read input
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        // Exit condition
        if input == "exit" {
            break;
        }

        // Tokenize and parse the input
        let tokens = tokenize(input);
        let parsed_expr_result = build_expression_tree(tokens);

        // Handle parsing error
        let parsed_expr = match parsed_expr_result {
            Ok(expr) => expr,
            Err(e) => {
                println!("Error parsing LaTeX: {}", e);
                continue;
            }
        };

        // Simplify the expression
        let simplified_expr = match parsed_expr.simplify(&env) {
            Ok(expr) => expr,
            Err(e) => {
                println!("Error simplifying expression: {}", e);
                continue;
            }
        };

        // Evaluate the simplified expression
        match Evaluator::evaluate(&simplified_expr, &env) {
            Ok(result) => println!("{}", result.to_string()),
            Err(_) => println!("{}", simplified_expr.to_string()),
        }
    }
}

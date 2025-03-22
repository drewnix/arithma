use arithma::simplify::Simplifiable;
use arithma::{build_expression_tree, Environment, Evaluator, Tokenizer};
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

        // Skip empty input
        if input.trim().is_empty() {
            continue;
        }

        // Special case for matrix multiplication with \cdot
        if input.contains("\\begin{pmatrix}")
            && input.contains("\\cdot")
            && input.contains("\\end{pmatrix}")
        {
            // Try to split the expression by \cdot
            let parts: Vec<&str> = input.split("\\cdot").collect();
            if parts.len() == 2 {
                // Handle matrix multiplication specially
                let matrix_a = parts[0].trim();
                let matrix_b = parts[1].trim();

                // Skip direct parsing of matrices since we're using parse_latex_matrix
                // Instead, directly use the specialized matrix parser function

                // Parse the matrices using the specialized matrix parser
                match (
                    arithma::matrix::parse_latex_matrix(matrix_a, &env),
                    arithma::matrix::parse_latex_matrix(matrix_b, &env),
                ) {
                    (Ok(matrix_a), Ok(matrix_b)) => {
                        // Multiply the matrices
                        match matrix_a.multiply(&matrix_b, &env) {
                            Ok(result) => {
                                println!("{}", result.to_latex());
                            }
                            Err(e) => {
                                println!("Error multiplying matrices: {}", e);
                            }
                        }
                    }
                    (Err(e), _) => {
                        println!("Error parsing first matrix: {}", e);
                    }
                    (_, Err(e)) => {
                        println!("Error parsing second matrix: {}", e);
                    }
                }
                continue;
            }
        }

        // Create an instance of the Tokenizer
        let mut tokenizer = Tokenizer::new(input); // Pass input as a reference

        // Tokenize and parse the input
        let tokens = tokenizer.tokenize(); // Call the instance method on tokenizer
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
            Ok(result) => println!("{}", result),
            Err(_) => println!("{}", simplified_expr),
        }
    }
}

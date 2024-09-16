use cassy::{build_expression_tree, solve_for_variable, tokenize, Environment};
use std::io::{self, Write};

fn main() {
    println!("Cassy - Type 'exit' to quit.");
    let mut env = Environment::new(); // Create an environment for variables

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

        // Handle equations (e.g., "x + 2 = 5")
        if input.contains('=') {
            let parts: Vec<&str> = input.split('=').collect();
            if parts.len() == 2 {
                let left_tokens = tokenize(parts[0].trim());
                let right_tokens = tokenize(parts[1].trim());

                // Build expression trees
                match (
                    build_expression_tree(left_tokens),
                    build_expression_tree(right_tokens),
                ) {
                    (Ok(left_tree), Ok(right_tree)) => {
                        // Evaluate the right-hand side expression tree
                        match right_tree.evaluate(&env) {
                            Ok(right_val) => {
                                // Extract the variable and solve for it
                                if let Some(var_name) = cassy::extract_variable(parts[0].trim()) {
                                    match solve_for_variable(&left_tree, right_val, &var_name) {
                                        Ok(result) => {
                                            env.set(&var_name, result);
                                            println!("{} = {}", var_name, result);
                                        }
                                        Err(e) => println!("Error: {}", e),
                                    }
                                } else {
                                    println!("Could not find a variable to solve for.");
                                }
                            }
                            Err(e) => println!("Error evaluating right side: {}", e),
                        }
                    }
                    _ => println!("Error parsing equation."),
                }
            } else {
                println!("Invalid equation. Use format: expression = expression");
            }
        } else {
            // Handle regular expressions (without equals sign)
            let tokens = tokenize(input); // Tokenize the input before passing
            match build_expression_tree(tokens) {
                Ok(tree) => match tree.evaluate(&env) {
                    Ok(result) => println!("{}", result),
                    Err(e) => println!("Error evaluating expression: {}", e),
                },
                Err(e) => println!("Error parsing expression: {}", e),
            }
        }
    }
}

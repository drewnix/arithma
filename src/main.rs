use std::io::{self, Write};

use cassy::{evaluate_rpn, tokenize, shunting_yard, extract_variable, solve_for_variable, Environment};

fn main() {
    println!("Cassy - Type 'exit' to quit.");
    let mut env = Environment::new();  // Create an environment for variables

    loop {
        // Prompt
        print!("> ");
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
                let left_expr = parts[0].trim();
                let right_expr = parts[1].trim();

                // Parse and evaluate the right-hand side of the equation
                let right_tokens = tokenize(right_expr);
                match shunting_yard(right_tokens) {
                    Ok(right_rpn) => match evaluate_rpn(right_rpn, &env) {
                        Ok(right_val) => {
                            // Extract the variable from the left-hand side
                            if let Some(var_name) = extract_variable(left_expr) {
                                // Solve for the variable
                                match solve_for_variable(left_expr, right_val, &env) {
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
                        Err(e) => println!("Error: {}", e),
                    },
                    Err(e) => println!("Error: {}", e),
                }
            } else {
                println!("Invalid equation. Use format: expression = expression");
            }
        } else {
            // Handle standard variable assignment or expression evaluation
            let tokens = tokenize(input);
            match shunting_yard(tokens) {
                Ok(rpn) => {
                    match evaluate_rpn(rpn, &env) {
                        Ok(result) => println!("{}", result),
                        Err(e) => println!("Error: {}", e),
                    }
                }
                Err(e) => println!("Error: {}", e),
            }
        }
    }
}

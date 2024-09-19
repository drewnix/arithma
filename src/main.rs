use arithma::{build_expression_tree, tokenize, Environment, Evaluator};
use std::io::{self, Write};

fn main() {
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

        let tokens = tokenize(input); // Tokenize the input before passing
        match build_expression_tree(tokens) {
            Ok(tree) => match Evaluator::evaluate(&tree, &env) {
                Ok(result) => println!("{}", result),
                Err(e) => println!("Error evaluating expression: {}", e),
            },
            Err(e) => println!("Error parsing expression: {}", e),
        }
    }
}

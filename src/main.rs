use arithma::simplify::Simplifiable;
use arithma::tokenizer::normalize_var;
use arithma::{build_expression_tree, Environment, Evaluator, Tokenizer};
use std::io::{self, Write};

fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        repl();
        return;
    }

    let cmd = args[1].as_str();

    if cmd == "--help" || cmd == "-h" || cmd == "help" {
        print_help();
        return;
    }

    match cmd {
        "simplify" => cmd_simplify(&args[2..]),
        "differentiate" | "diff" => cmd_differentiate(&args[2..]),
        "integrate" => cmd_integrate(&args[2..]),
        "solve" => cmd_solve(&args[2..]),
        "factor" => cmd_factor(&args[2..]),
        "partial-fractions" | "pf" => cmd_partial_fractions(&args[2..]),
        "evaluate" | "eval" => cmd_evaluate(&args[2..]),
        "limit" => cmd_limit(&args[2..]),
        "taylor" => cmd_taylor(&args[2..]),
        "substitute" | "sub" => cmd_substitute(&args[2..]),
        "ode" => cmd_ode(&args[2..]),
        _ => {
            eprintln!("Unknown command: {}", cmd);
            eprintln!("Run 'arithma --help' for usage.");
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!(
        "\
Arithma — exact computer algebra from the command line

Usage: arithma [command] [arguments]
       arithma              (launches interactive REPL)

Commands:
  simplify <expr>                    Simplify an expression
  differentiate <expr> [var]         Differentiate (alias: diff)
  integrate <expr> [var]             Indefinite integral
  solve <equation> [var]             Solve an equation
  factor <expr> [var]                Factor a polynomial over Q
  partial-fractions <n> <d> [var]    Partial fraction decomposition (alias: pf)
  evaluate <expr> [var=val ...]      Evaluate numerically (alias: eval)
  limit <expr> [var] [point]         Compute a limit
  taylor <expr> [var] [center] [n]   Taylor series expansion
  substitute <expr> <var> <value>    Substitute a value for a variable (alias: sub)
  ode <rhs> [indep] [dep]            Solve first-order ODE: dy/dx = rhs
  ode --cc <a> <b> <c> [indep]       Solve ay''+by'+cy=0

All expressions use LaTeX notation. Variable defaults to x where applicable.

Examples:
  arithma simplify \"x^2 + 2x + 1\"
  arithma diff \"\\\\sin(x^2)\" x
  arithma integrate \"3x^2\" x
  arithma solve \"x^2 - 4 = 0\"
  arithma factor \"x^4 - 1\"
  arithma eval \"x^2 + 1\" x=3
  arithma limit \"\\\\frac{{\\\\sin(x)}}{{x}}\" x 0
  arithma taylor \"\\\\sin(x)\" x 0 5
  arithma ode \"x \\\\cdot y\" x y
  arithma ode --cc 1 0 1"
    );
}

fn parse_and_simplify(expr: &str, env: &Environment) -> Result<String, String> {
    let mut tokenizer = Tokenizer::new(expr);
    let tokens = tokenizer.tokenize();
    if let Some(err) = tokenizer.errors.into_iter().next() {
        return Err(err);
    }
    let parsed = build_expression_tree(tokens)?;
    let simplified = parsed.simplify(env).unwrap_or(parsed);
    Ok(format!("{}", simplified))
}

fn cmd_simplify(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: arithma simplify <expr>");
        std::process::exit(1);
    }
    let expr = &args[0];
    let env = Environment::new();
    match parse_and_simplify(expr, &env) {
        Ok(result) => println!("{}", result),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_differentiate(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: arithma differentiate <expr> [var]");
        std::process::exit(1);
    }
    let expr = &args[0];
    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".to_string());
    match arithma::derivative::differentiate_latex(expr, &var) {
        Ok(result) => println!("{}", result),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_integrate(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: arithma integrate <expr> [var]");
        std::process::exit(1);
    }
    let expr = &args[0];
    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".to_string());
    match arithma::integration::integrate_latex(expr, &var) {
        Ok(result) => println!("{}", result),
        Err(e) if e.starts_with("NON_ELEMENTARY:") => {
            println!("{}", e.replacen("NON_ELEMENTARY: ", "", 1));
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_solve(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: arithma solve <equation> [var]");
        std::process::exit(1);
    }
    let equation = &args[0];
    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".to_string());

    let mut tokenizer = Tokenizer::new(equation);
    let tokens = tokenizer.tokenize();
    let expr = match build_expression_tree(tokens) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    match arithma::expression::solve_full(&expr, &var) {
        Ok(result) => {
            if result.solutions.is_empty() && result.complex_omitted > 0 {
                println!(
                    "No real solutions ({} complex root{} omitted)",
                    result.complex_omitted,
                    if result.complex_omitted == 1 { "" } else { "s" }
                );
            } else if result.solutions.is_empty() {
                println!("No solutions found");
            } else {
                for s in &result.solutions {
                    println!("{} = {}", var, s);
                }
                if result.complex_omitted > 0 {
                    println!(
                        "({} complex root{} omitted)",
                        result.complex_omitted,
                        if result.complex_omitted == 1 { "" } else { "s" }
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_factor(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: arithma factor <expr> [var]");
        std::process::exit(1);
    }
    let expr = &args[0];
    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".to_string());

    let mut tokenizer = Tokenizer::new(expr);
    let tokens = tokenizer.tokenize();
    let node = match build_expression_tree(tokens) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let poly = match arithma::polynomial::Polynomial::from_node(&node, &var) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Not a polynomial: {}", e);
            std::process::exit(1);
        }
    };

    let (content, factors) = arithma::mod_poly::factor_over_q(&poly);

    let mut parts: Vec<String> = Vec::new();

    let content_node = arithma::Node::Num(arithma::ExactNum::rational(
        content.numer().try_into().unwrap_or(1),
        content.denom().try_into().unwrap_or(1),
    ));
    let content_str = format!("{}", content_node);
    if content_str != "1" {
        parts.push(content_str);
    }

    let mut grouped: Vec<(String, usize)> = Vec::new();
    for f in &factors {
        let s = format!("{}", f);
        if let Some(entry) = grouped.iter_mut().find(|(fs, _)| *fs == s) {
            entry.1 += 1;
        } else {
            grouped.push((s, 1));
        }
    }

    for (f_str, m) in &grouped {
        if *m == 1 {
            parts.push(format!("({})", f_str));
        } else {
            parts.push(format!("({})^{}", f_str, m));
        }
    }

    if parts.is_empty() {
        println!("1");
    } else {
        println!("{}", parts.join(" * "));
    }
}

fn cmd_partial_fractions(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: arithma partial-fractions <numerator> <denominator> [var]");
        std::process::exit(1);
    }
    let num = &args[0];
    let den = &args[1];
    let var = args
        .get(2)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".to_string());
    match arithma::partial_fractions::partial_fractions_latex(num, den, &var) {
        Ok(result) => println!("{}", result),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_evaluate(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: arithma evaluate <expr> [var=val ...]");
        std::process::exit(1);
    }
    let expr_str = &args[0];

    let mut tokenizer = Tokenizer::new(expr_str);
    let tokens = tokenizer.tokenize();
    let expr = match build_expression_tree(tokens) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let env_s = Environment::new();
    let simplified = expr.simplify(&env_s).unwrap_or_else(|_| expr.clone());

    let mut env = Environment::new();
    for arg in &args[1..] {
        if let Some((var, val_str)) = arg.split_once('=') {
            if let Ok(val) = val_str.parse::<f64>() {
                if val == val.floor() && val.abs() < 1e15 {
                    env.set_exact(var, arithma::ExactNum::integer(val as i64));
                } else {
                    env.set(var, val);
                }
            } else {
                eprintln!("Invalid value for {}: {}", var, val_str);
                std::process::exit(1);
            }
        }
    }

    match Evaluator::evaluate_exact(&simplified, &env) {
        Ok(val) => println!("{}", arithma::Node::Num(val)),
        Err(_) => match Evaluator::evaluate(&simplified, &env) {
            Ok(val) => println!("{}", val),
            Err(_) => println!("{}", simplified),
        },
    }
}

fn cmd_limit(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: arithma limit <expr> [var] [point]");
        std::process::exit(1);
    }
    let expr = &args[0];
    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".to_string());
    let point = args
        .get(2)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    match arithma::limits::limit_latex(expr, &var, point) {
        Ok(result) => println!("{}", result),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_taylor(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: arithma taylor <expr> [var] [center] [order]");
        std::process::exit(1);
    }
    let expr = &args[0];
    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".to_string());
    let order = args
        .get(3)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(5);

    let center_str = args.get(2).map(|s| s.as_str()).unwrap_or("0");

    if let Ok(center_f64) = center_str.parse::<f64>() {
        match arithma::series::taylor_series_latex(expr, &var, center_f64, order) {
            Ok(result) => println!("{}", result),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let center_normalized = normalize_var(center_str);
        match arithma::series::taylor_series_latex_symbolic(expr, &var, &center_normalized, order) {
            Ok(result) => println!("{}", result),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn cmd_substitute(args: &[String]) {
    if args.len() < 3 {
        eprintln!("Usage: arithma substitute <expr> <var> <value>");
        std::process::exit(1);
    }
    let expr = &args[0];
    let var = normalize_var(&args[1]);
    let value = &args[2];
    let subs = vec![(var, value.to_string())];
    match arithma::substitute::substitute_latex(expr, &subs) {
        Ok(result) => {
            let env = Environment::new();
            match parse_and_simplify(&result, &env) {
                Ok(simplified) => println!("{}", simplified),
                Err(_) => println!("{}", result),
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_ode(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: arithma ode <rhs> [indep] [dep]");
        eprintln!("       arithma ode --cc <a> <b> <c> [indep]");
        std::process::exit(1);
    }

    if args[0] == "--cc" {
        if args.len() < 4 {
            eprintln!("Usage: arithma ode --cc <a> <b> <c> [indep]");
            std::process::exit(1);
        }
        let a: f64 = args[1].parse().unwrap_or_else(|_| {
            eprintln!("Invalid coefficient a: {}", args[1]);
            std::process::exit(1);
        });
        let b: f64 = args[2].parse().unwrap_or_else(|_| {
            eprintln!("Invalid coefficient b: {}", args[2]);
            std::process::exit(1);
        });
        let c: f64 = args[3].parse().unwrap_or_else(|_| {
            eprintln!("Invalid coefficient c: {}", args[3]);
            std::process::exit(1);
        });
        let indep = args
            .get(4)
            .map(|s| normalize_var(s))
            .unwrap_or_else(|| "x".to_string());
        match arithma::ode::solve_constant_coeff_latex(a, b, c, &indep) {
            Ok(result) => println!("{}", result),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let rhs = &args[0];
        let indep = args
            .get(1)
            .map(|s| normalize_var(s))
            .unwrap_or_else(|| "x".to_string());
        let dep = args
            .get(2)
            .map(|s| normalize_var(s))
            .unwrap_or_else(|| "y".to_string());
        match arithma::ode::solve_ode_latex(rhs, &indep, &dep) {
            Ok(result) => println!("{}", result),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn repl() {
    println!("Arithma — type 'exit' to quit, '--help' for commands.");
    let env = Environment::new();

    loop {
        print!(">> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input == "exit" || input == "quit" {
            break;
        }
        if input.is_empty() {
            continue;
        }
        if input == "--help" || input == "help" {
            print_help();
            continue;
        }

        if input.contains("\\begin{pmatrix}")
            && input.contains("\\cdot")
            && input.contains("\\end{pmatrix}")
        {
            let parts: Vec<&str> = input.split("\\cdot").collect();
            if parts.len() == 2 {
                let matrix_a = parts[0].trim();
                let matrix_b = parts[1].trim();
                match (
                    arithma::matrix::parse_latex_matrix(matrix_a, &env),
                    arithma::matrix::parse_latex_matrix(matrix_b, &env),
                ) {
                    (Ok(a), Ok(b)) => match a.multiply(&b, &env) {
                        Ok(result) => println!("{}", result.to_latex()),
                        Err(e) => println!("Error: {}", e),
                    },
                    (Err(e), _) | (_, Err(e)) => println!("Error: {}", e),
                }
                continue;
            }
        }

        let mut tokenizer = Tokenizer::new(input);
        let tokens = tokenizer.tokenize();
        let parsed = match build_expression_tree(tokens) {
            Ok(expr) => expr,
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        };

        let simplified = match parsed.simplify(&env) {
            Ok(expr) => expr,
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        };

        match Evaluator::evaluate(&simplified, &env) {
            Ok(result) => println!("{}", result),
            Err(_) => println!("{}", simplified),
        }
    }
}

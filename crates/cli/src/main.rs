use arithma::simplify::Simplifiable;
use arithma::status::{ProofCertificate, StatusReport};
use arithma::tokenizer::normalize_var;
use arithma::{
    build_expression_tree, parse_latex, parse_latex_raw, Environment, Evaluator, Node, Tokenizer,
};
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

    if cmd == "--version" || cmd == "-V" {
        println!("arithma {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    match cmd {
        "format" => cmd_format(cmd, &args[2..]),
        "simplify" => cmd_simplify(cmd, &args[2..]),
        "differentiate" | "diff" => cmd_differentiate(cmd, &args[2..]),
        "integrate" => cmd_integrate(cmd, &args[2..]),
        "solve" => cmd_solve(cmd, &args[2..]),
        "factor" => cmd_factor(cmd, &args[2..]),
        "prime-factorize" | "factorint" => cmd_prime_factorize(cmd, &args[2..]),
        "partial-fractions" | "pf" => cmd_partial_fractions(cmd, &args[2..]),
        "evaluate" | "eval" => cmd_evaluate(cmd, &args[2..]),
        "limit" => cmd_limit(cmd, &args[2..]),
        "taylor" => cmd_taylor(cmd, &args[2..]),
        "substitute" | "sub" => cmd_substitute(cmd, &args[2..]),
        "ode" => cmd_ode(cmd, &args[2..]),
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
  format <expr>                      Parse and print canonical LaTeX (no simplify)
  simplify <expr>                    Simplify an expression
  differentiate <expr> [var]         Differentiate (alias: diff)
  integrate <expr> [var] [lo hi]      Integral (definite with bounds)
  solve <equation> [var]             Solve an equation
  solve \"eq1, eq2\" \"x, y\"           Solve a system of linear equations
  factor <expr> [var]                Factor a polynomial over Q
  prime-factorize <n>                Prime-factorize a positive integer (alias: factorint)
  partial-fractions <n> <d> [var]    Partial fraction decomposition (alias: pf)
  evaluate <expr> [var=val ...]      Evaluate numerically (alias: eval)
  limit <expr> [var] [point]         Compute a limit
  taylor <expr> [var] [center] [n]   Taylor series expansion
  substitute <expr> <var> <value>    Substitute a value for a variable (alias: sub)
  ode <rhs> [indep] [dep]            Solve first-order ODE: dy/dx = rhs
  ode --cc <a> <b> <c> [indep]       Solve ay''+by'+cy=0

All expressions use LaTeX notation. Variable defaults to x where applicable.

Examples:
  arithma format \"\\frac{{2}}{{2+{{\\pi}}}}+.5{{\\pi}}\"
  arithma simplify \"x^2 + 2x + 1\"
  arithma diff \"\\\\sin(x^2)\" x
  arithma integrate \"3x^2\" x
  arithma solve \"x^2 - 4 = 0\"
  arithma factor \"x^4 - 1\"
  arithma prime-factorize 720
  arithma eval \"x^2 + 1\" x=3
  arithma limit \"\\\\frac{{\\\\sin(x)}}{{x}}\" x 0
  arithma taylor \"\\\\sin(x)\" x 0 5
  arithma ode \"x \\\\cdot y\" x y
  arithma ode --cc 1 0 1"
    );
}

const NONE: &[&str] = &[];

/// Print CLI usage and exit.
///
/// - `syntax` — primary fragment after `arithma {cmd}`
/// - `alternates` — other valid invocations (each prefixed with `arithma {cmd}`); pass `NONE` if none
/// - `hints` — free-form notes (indented, no command prefix); pass `NONE` if none
fn usage(cmd: &str, syntax: &str, alternates: &[&str], hints: &[&str]) -> ! {
    eprintln!("Usage: arithma {cmd} {syntax}");
    for alt in alternates {
        eprintln!("       arithma {cmd} {alt}");
    }
    for hint in hints {
        eprintln!("  {hint}");
    }
    std::process::exit(1);
}

fn cmd_format(cmd: &str, args: &[String]) {
    if args.is_empty() {
        usage(cmd, "<expr>", NONE, NONE);
    }
    let expr = &args[0];
    match parse_latex_raw(expr).map(|node| format!("{node}")) {
        Ok(result) => println!("{result}"),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_simplify(cmd: &str, args: &[String]) {
    if args.is_empty() {
        usage(cmd, "<expr>", NONE, NONE);
    }
    let expr = &args[0];
    let env = Environment::new();
    match parse_latex(expr, &env).map(|node| format!("{node}")) {
        Ok(result) => println!("{result}"),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_differentiate(cmd: &str, args: &[String]) {
    if args.is_empty() {
        usage(cmd, "<expr> [var]", NONE, NONE);
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

/// Render a NON_ELEMENTARY error as the status marker, so the CLI and the
/// MCP server present impossibility identically (docs/result-status.md).
/// When the antiderivative is a recognized special function (erf, Ei, li),
/// the marker also names the form — strictly more information than the
/// impossibility alone.
fn non_elementary_marker(e: &str, integrand_latex: &str, var: &str) -> String {
    let reason = e.replacen("NON_ELEMENTARY: ", "", 1);
    let proof = ProofCertificate::non_elementary(&reason);
    let status = StatusReport::provably_impossible(proof);
    let status =
        match arithma::special_functions::recognize_special_form_latex(integrand_latex, var) {
            Some((name, form)) => status.with_special_form(&name, &form),
            None => status,
        };
    status
        .marker()
        .expect("provably_impossible always has a marker")
}

fn cmd_integrate(cmd: &str, args: &[String]) {
    if args.is_empty() {
        usage(cmd, "<expr> [var] [lower upper]", NONE, NONE);
    }
    let expr = &args[0];
    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".to_string());

    if args.len() >= 4 {
        let lower = &args[2];
        let upper = &args[3];
        match arithma::integration::definite_integral_exact_latex(expr, &var, lower, upper) {
            Ok(result) => println!("{}", result),
            Err(e) if e.starts_with("NON_ELEMENTARY:") => {
                println!("{}", non_elementary_marker(&e, expr, &var));
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    match arithma::integration::integrate_latex(expr, &var) {
        Ok(result) => println!("{}", result),
        Err(e) if e.starts_with("NON_ELEMENTARY:") => {
            println!("{}", non_elementary_marker(&e, expr, &var));
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_solve(cmd: &str, args: &[String]) {
    if args.is_empty() {
        usage(
            cmd,
            "<equation> [var]",
            &["\"eq1, eq2, ...\" \"x, y, ...\""],
            NONE,
        );
    }
    let equation = &args[0];

    if let Some(vars_arg) = args.get(1) {
        let vars: Vec<String> = vars_arg
            .split(',')
            .map(|s| normalize_var(s.trim()))
            .collect();
        if vars.len() > 1 || equation.contains(',') {
            cmd_solve_system(equation, &vars);
            return;
        }
    }

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

    // Check if it's an inequality
    if matches!(
        expr,
        Node::Greater(_, _) | Node::GreaterEqual(_, _) | Node::Less(_, _) | Node::LessEqual(_, _)
    ) {
        match arithma::solve_inequality(&expr, &var) {
            Ok(result) => println!("{}", result),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

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

fn cmd_solve_system(equations_str: &str, vars: &[String]) {
    let eq_strings: Vec<&str> = equations_str.split(',').collect();
    let mut equations = Vec::new();

    for eq_str in &eq_strings {
        let mut tokenizer = Tokenizer::new(eq_str.trim());
        let tokens = tokenizer.tokenize();
        match build_expression_tree(tokens) {
            Ok(e) => equations.push(e),
            Err(e) => {
                eprintln!("Error parsing '{}': {}", eq_str.trim(), e);
                std::process::exit(1);
            }
        }
    }

    match arithma::solve_system(&equations, vars) {
        Ok(arithma::SystemSolution::Unique(solutions)) => {
            for (var, val) in &solutions {
                println!("{} = {}", var, val);
            }
        }
        Ok(arithma::SystemSolution::Multiple(sets)) => {
            for (i, solutions) in sets.iter().enumerate() {
                if sets.len() > 1 {
                    println!("Solution {}:", i + 1);
                }
                for (var, val) in solutions {
                    let prefix = if sets.len() > 1 { "  " } else { "" };
                    println!("{}{} = {}", prefix, var, val);
                }
            }
        }
        Ok(arithma::SystemSolution::Parametric {
            solutions,
            free_vars,
        }) => {
            println!(
                "Parametric solution (free variable{}: {}):",
                if free_vars.len() > 1 { "s" } else { "" },
                free_vars.join(", ")
            );
            for (var, val) in &solutions {
                println!("  {} = {}", var, val);
            }
        }
        Ok(arithma::SystemSolution::NoSolution) => {
            println!("No solution (inconsistent system)");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_factor(cmd: &str, args: &[String]) {
    if args.is_empty() {
        usage(cmd, "<expr> [var]", NONE, NONE);
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
        let result = parts.join(" * ");
        println!("{}", result);
        if factors.len() == 1 && factors[0].degree().unwrap_or(0) > 1 {
            println!("(irreducible over \\mathbb{{Q}})");
        }
    }
}

fn cmd_prime_factorize(cmd: &str, args: &[String]) {
    if args.is_empty() {
        usage(cmd, "<n>", NONE, NONE);
    }
    let n: u64 = match args[0].parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("Error: expected a non-negative integer");
            std::process::exit(1);
        }
    };
    println!("{}", arithma::prime_factorize_latex(n));
}

fn cmd_partial_fractions(cmd: &str, args: &[String]) {
    if args.len() < 2 {
        usage(cmd, "<numerator> <denominator> [var]", NONE, NONE);
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

fn cmd_evaluate(cmd: &str, args: &[String]) {
    if args.is_empty() {
        usage(cmd, "<expr> [var=val ...]", NONE, NONE);
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

fn cmd_limit(cmd: &str, args: &[String]) {
    if args.is_empty() {
        usage(
            cmd,
            "<expr> [var] [point]",
            NONE,
            &["point: number, inf, -inf, or one-sided (0+, 0-, 3+, 3-)"],
        );
    }
    let expr = &args[0];
    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".to_string());
    let point_str = args.get(2).map(|s| s.as_str()).unwrap_or("0");
    match arithma::limits::limit_latex_str(expr, &var, point_str) {
        Ok(result) => println!("{}", result),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_taylor(cmd: &str, args: &[String]) {
    if args.is_empty() {
        usage(cmd, "<expr> [var] [center] [order]", NONE, NONE);
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

fn cmd_substitute(cmd: &str, args: &[String]) {
    if args.len() < 3 {
        usage(cmd, "<expr> <var> <value>", NONE, NONE);
    }
    let expr = &args[0];
    let var = normalize_var(&args[1]);
    let value = &args[2];
    let subs = vec![(var, value.to_string())];
    match arithma::substitute::substitute_latex(expr, &subs) {
        Ok(result) => {
            let env = Environment::new();
            match parse_latex(&result, &env).map(|node| format!("{node}")) {
                Ok(simplified) => println!("{simplified}"),
                Err(_) => println!("{result}"),
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_ode(cmd: &str, args: &[String]) {
    if args.is_empty() {
        usage(
            cmd,
            "<rhs> [indep] [dep]",
            &["--cc <a> <b> <c> [indep]"],
            NONE,
        );
    }

    if args[0] == "--cc" {
        if args.len() < 4 {
            usage(cmd, "--cc <a> <b> <c> [indep]", NONE, NONE);
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
    println!("Arithma — type 'exit' to quit, '--help' for commands, 'format <expr>' for canonical LaTeX.");
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

        if let Some(expr) = input.strip_prefix("format ") {
            match parse_latex_raw(expr.trim()).map(|node| format!("{node}")) {
                Ok(result) => println!("{result}"),
                Err(e) => println!("Error: {e}"),
            }
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

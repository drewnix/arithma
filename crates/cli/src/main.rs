mod unicode;

use arithma::simplify::Simplifiable;
use arithma::status::{ProofCertificate, StatusReport};
use arithma::tokenizer::normalize_var;
use arithma::{
    build_expression_tree, parse_latex, parse_latex_raw, Environment, Evaluator, Node, Tokenizer,
};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

static LATEX_OUTPUT: AtomicBool = AtomicBool::new(false);
static USE_COLOR: AtomicBool = AtomicBool::new(false);

mod ansi {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const RED: &str = "\x1b[31m";
    pub const CYAN: &str = "\x1b[36m";
}

fn color_enabled() -> bool {
    USE_COLOR.load(Ordering::Relaxed)
}

fn output(s: &str) {
    if LATEX_OUTPUT.load(Ordering::Relaxed) {
        println!("{s}");
    } else {
        println!("{}", unicode::latex_to_unicode(s));
    }
}

fn print_error(msg: &str) {
    if color_enabled() {
        println!("{}{}{}{}", ansi::RED, ansi::BOLD, msg, ansi::RESET);
    } else {
        println!("{msg}");
    }
}

fn print_note(msg: &str) {
    if color_enabled() {
        println!("{}{}{}", ansi::DIM, msg, ansi::RESET);
    } else {
        println!("{msg}");
    }
}

fn main() {
    env_logger::init();

    let raw_args: Vec<String> = std::env::args().collect();
    let is_tty = std::io::stdout().is_terminal();
    let force_unicode = raw_args.iter().any(|a| a == "--unicode");
    if !force_unicode && (raw_args.iter().any(|a| a == "--latex") || !is_tty) {
        LATEX_OUTPUT.store(true, Ordering::Relaxed);
    }
    if is_tty && std::env::var_os("NO_COLOR").is_none() {
        USE_COLOR.store(true, Ordering::Relaxed);
    }
    let args: Vec<String> = raw_args
        .into_iter()
        .filter(|a| a != "--latex" && a != "--unicode")
        .enumerate()
        .map(|(i, a)| if i >= 2 { preprocess_input(&a) } else { a })
        .collect();

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

Options:
  --latex                          Output raw LaTeX (default when piped)
  --unicode                        Output Unicode (default in terminal)

All expressions accept LaTeX or natural notation (pi, inf, sqrt, sin, etc.).

Examples:
  arithma simplify 'x^2 + 2x + 1'
  arithma diff 'sin(x^2)' x
  arithma integrate '3x^2' x
  arithma integrate '1/(x^2+1)' x 0 1
  arithma solve 'x^2 - 4 = 0'
  arithma factor 'x^4 - 1'
  arithma prime-factorize 720
  arithma eval 'x^2 + 1' x=3
  arithma limit 'sin(x)/x' x 0
  arithma taylor 'sin(x)' x 0 5
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
        Ok(result) => output(&result),
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
        Ok(result) => output(&result),
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
        Ok(result) => output(&result),
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
            Ok(result) => output(&result),
            Err(e) if e.starts_with("NON_ELEMENTARY:") => {
                output(&non_elementary_marker(&e, expr, &var));
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    match arithma::integration::integrate_latex(expr, &var) {
        Ok(result) => output(&result),
        Err(e) if e.starts_with("NON_ELEMENTARY:") => {
            output(&non_elementary_marker(&e, expr, &var));
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
            Ok(result) => output(&result),
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
                print_note("No solutions found");
            } else {
                for s in &result.solutions {
                    output(&format!("{var} = {s}"));
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
                output(&format!("{var} = {val}"));
            }
        }
        Ok(arithma::SystemSolution::Multiple(sets)) => {
            for (i, solutions) in sets.iter().enumerate() {
                if sets.len() > 1 {
                    println!("Solution {}:", i + 1);
                }
                for (var, val) in solutions {
                    let prefix = if sets.len() > 1 { "  " } else { "" };
                    output(&format!("{prefix}{var} = {val}"));
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
                output(&format!("  {var} = {val}"));
            }
        }
        Ok(arithma::SystemSolution::NoSolution) => {
            print_note("No solution (inconsistent system)");
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
        output("1");
    } else {
        let result = parts.join(" * ");
        output(&result);
        if factors.len() == 1 && factors[0].degree().unwrap_or(0) > 1 {
            output("(irreducible over \\mathbb{Q})");
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
    output(&arithma::prime_factorize_latex(n));
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
        Ok(result) => output(&result),
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
        Ok(val) => output(&format!("{}", arithma::Node::Num(val))),
        Err(_) => match Evaluator::evaluate(&simplified, &env) {
            Ok(val) => output(&format!("{val}")),
            Err(_) => output(&format!("{simplified}")),
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
        Ok(result) => output(&result),
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
            Ok(result) => output(&result),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let center_normalized = normalize_var(center_str);
        match arithma::series::taylor_series_latex_symbolic(expr, &var, &center_normalized, order) {
            Ok(result) => output(&result),
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
                Ok(simplified) => output(&simplified),
                Err(_) => output(&result),
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
            Ok(result) => output(&result),
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
            Ok(result) => output(&result),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

/// Shell-like argument splitting that respects quoted groups.
/// `solve "x^2 - 4 = 0" x` → ["x^2 - 4 = 0", "x"]
fn split_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote: Option<char> = None;

    for c in input.chars() {
        match c {
            q @ ('"' | '\'') if in_quote == Some(q) => in_quote = None,
            q @ ('"' | '\'') if in_quote.is_none() => in_quote = Some(q),
            c if c.is_whitespace() && in_quote.is_none() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

/// Replace natural math notation with LaTeX equivalents.
/// Converts standalone `pi` → `\pi`, `inf`/`infinity` → `\infty`.
fn preprocess_input(input: &str) -> String {
    let b = input.as_bytes();
    let len = b.len();
    let mut out = String::with_capacity(len + 16);
    let mut i = 0;

    while i < len {
        let word_start = i == 0 || {
            let prev = b[i - 1];
            !(prev.is_ascii_alphabetic() || prev == b'\\' || prev == b'_')
        };

        if word_start {
            if i + 8 <= len
                && &b[i..i + 8] == b"infinity"
                && (i + 8 >= len || !(b[i + 8].is_ascii_alphanumeric() || b[i + 8] == b'_'))
            {
                out.push_str("\\infty");
                i += 8;
                continue;
            }
            if i + 2 <= len
                && &b[i..i + 2] == b"pi"
                && (i + 2 >= len || !(b[i + 2].is_ascii_alphanumeric() || b[i + 2] == b'_'))
            {
                out.push_str("\\pi");
                i += 2;
                continue;
            }
            if i + 3 <= len
                && &b[i..i + 3] == b"inf"
                && (i + 3 >= len || !(b[i + 3].is_ascii_alphanumeric() || b[i + 3] == b'_'))
            {
                out.push_str("\\infty");
                i += 3;
                continue;
            }
        }

        out.push(b[i] as char);
        i += 1;
    }

    out
}

fn print_repl_help() {
    println!(
        "\
Commands:
  simplify <expr>                  Simplify an expression
  diff <expr> [var]                Differentiate (default var: x)
  integrate <expr> [var] [lo hi]   Integrate (definite with bounds)
  solve <equation> [var]           Solve equation or inequality
  solve \"eq1,eq2\" \"x,y\"           Solve a system
  factor <expr> [var]              Factor over Q
  limit <expr> [var] [point]       Limit (point: number, inf, 0+, 0-)
  taylor <expr> [var] [center] [n] Taylor series (default order 5)
  eval <expr> [var=val ...]        Evaluate numerically
  sub <expr> <var> <value>         Substitute a value
  ode <rhs> [indep] [dep]          Solve dy/dx = rhs
  ode --cc <a> <b> <c>             Solve ay'' + by' + cy = 0
  factorint <n>                    Prime factorization
  pf <num> <den> [var]             Partial fractions
  format <expr>                    Show canonical LaTeX

Or type any expression to simplify and evaluate.
Constants: pi (= π), inf (= ∞). LaTeX notation also accepted.
Toggle output: 'latex' for raw LaTeX, 'unicode' for readable output."
    );
}

fn repl_format(rest: &str) {
    match parse_latex_raw(rest).map(|n| format!("{n}")) {
        Ok(r) => output(&r),
        Err(e) => print_error(&format!("Error: {e}")),
    }
}

fn repl_simplify(rest: &str, env: &Environment) {
    match parse_latex(rest, env).map(|n| format!("{n}")) {
        Ok(r) => output(&r),
        Err(e) => print_error(&format!("Error: {e}")),
    }
}

fn repl_diff(rest: &str) {
    let args_owned = split_args(rest);
    let args: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".into());
    match arithma::derivative::differentiate_latex(args[0], &var) {
        Ok(r) => output(&r),
        Err(e) => print_error(&format!("Error: {e}")),
    }
}

fn repl_integrate(rest: &str) {
    let args_owned = split_args(rest);
    let args: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
    let expr = args[0];
    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".into());
    if args.len() >= 4 {
        match arithma::integration::definite_integral_exact_latex(expr, &var, args[2], args[3]) {
            Ok(r) => output(&r),
            Err(e) if e.starts_with("NON_ELEMENTARY:") => {
                output(&non_elementary_marker(&e, expr, &var));
            }
            Err(e) => print_error(&format!("Error: {e}")),
        }
    } else {
        match arithma::integration::integrate_latex(expr, &var) {
            Ok(r) => output(&r),
            Err(e) if e.starts_with("NON_ELEMENTARY:") => {
                output(&non_elementary_marker(&e, expr, &var));
            }
            Err(e) => print_error(&format!("Error: {e}")),
        }
    }
}

fn repl_solve(rest: &str) {
    let args_owned = split_args(rest);
    let args: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
    let equation = args[0];

    if let Some(vars_str) = args.get(1) {
        let vars: Vec<String> = vars_str
            .split(',')
            .map(|s| normalize_var(s.trim()))
            .collect();
        if vars.len() > 1 || equation.contains(',') {
            repl_solve_system(equation, &vars);
            return;
        }
    }

    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".into());

    let mut tokenizer = Tokenizer::new(equation);
    let tokens = tokenizer.tokenize();
    let expr = match build_expression_tree(tokens) {
        Ok(e) => e,
        Err(e) => {
            print_error(&format!("Error: {e}"));
            return;
        }
    };

    if matches!(
        expr,
        Node::Greater(_, _) | Node::GreaterEqual(_, _) | Node::Less(_, _) | Node::LessEqual(_, _)
    ) {
        match arithma::solve_inequality(&expr, &var) {
            Ok(r) => output(&r),
            Err(e) => print_error(&format!("Error: {e}")),
        }
        return;
    }

    match arithma::expression::solve_full(&expr, &var) {
        Ok(result) => {
            if result.solutions.is_empty() && result.complex_omitted > 0 {
                print_note(&format!(
                    "No real solutions ({} complex root{} omitted)",
                    result.complex_omitted,
                    if result.complex_omitted == 1 { "" } else { "s" }
                ));
            } else if result.solutions.is_empty() {
                print_note("No solutions found");
            } else {
                for s in &result.solutions {
                    output(&format!("{var} = {s}"));
                }
                if result.complex_omitted > 0 {
                    print_note(&format!(
                        "({} complex root{} omitted)",
                        result.complex_omitted,
                        if result.complex_omitted == 1 { "" } else { "s" }
                    ));
                }
            }
        }
        Err(e) => print_error(&format!("Error: {e}")),
    }
}

fn repl_solve_system(equations_str: &str, vars: &[String]) {
    let eq_strings: Vec<&str> = equations_str.split(',').collect();
    let mut equations = Vec::new();
    for eq_str in &eq_strings {
        let mut tokenizer = Tokenizer::new(eq_str.trim());
        let tokens = tokenizer.tokenize();
        match build_expression_tree(tokens) {
            Ok(e) => equations.push(e),
            Err(e) => {
                print_error(&format!("Error parsing '{}': {e}", eq_str.trim()));
                return;
            }
        }
    }
    match arithma::solve_system(&equations, vars) {
        Ok(arithma::SystemSolution::Unique(solutions)) => {
            for (var, val) in &solutions {
                output(&format!("{var} = {val}"));
            }
        }
        Ok(arithma::SystemSolution::Multiple(sets)) => {
            for (i, solutions) in sets.iter().enumerate() {
                if sets.len() > 1 {
                    println!("Solution {}:", i + 1);
                }
                for (var, val) in solutions {
                    let pre = if sets.len() > 1 { "  " } else { "" };
                    output(&format!("{pre}{var} = {val}"));
                }
            }
        }
        Ok(arithma::SystemSolution::Parametric {
            solutions,
            free_vars,
        }) => {
            println!("Parametric solution (free: {}):", free_vars.join(", "));
            for (var, val) in &solutions {
                output(&format!("  {var} = {val}"));
            }
        }
        Ok(arithma::SystemSolution::NoSolution) => {
            print_note("No solution (inconsistent system)");
        }
        Err(e) => print_error(&format!("Error: {e}")),
    }
}

fn repl_factor(rest: &str) {
    let args_owned = split_args(rest);
    let args: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".into());

    let mut tokenizer = Tokenizer::new(args[0]);
    let tokens = tokenizer.tokenize();
    let node = match build_expression_tree(tokens) {
        Ok(n) => n,
        Err(e) => {
            print_error(&format!("Error: {e}"));
            return;
        }
    };

    let poly = match arithma::polynomial::Polynomial::from_node(&node, &var) {
        Ok(p) => p,
        Err(e) => {
            print_error(&format!("Not a polynomial: {e}"));
            return;
        }
    };

    let (content, factors) = arithma::mod_poly::factor_over_q(&poly);

    let mut parts: Vec<String> = Vec::new();
    let content_node = arithma::Node::Num(arithma::ExactNum::rational(
        content.numer().try_into().unwrap_or(1),
        content.denom().try_into().unwrap_or(1),
    ));
    let cs = format!("{content_node}");
    if cs != "1" {
        parts.push(cs);
    }

    let mut grouped: Vec<(String, usize)> = Vec::new();
    for f in &factors {
        let s = format!("{f}");
        if let Some(entry) = grouped.iter_mut().find(|(fs, _)| *fs == s) {
            entry.1 += 1;
        } else {
            grouped.push((s, 1));
        }
    }
    for (f, m) in &grouped {
        if *m == 1 {
            parts.push(format!("({f})"));
        } else {
            parts.push(format!("({f})^{m}"));
        }
    }

    if parts.is_empty() {
        output("1");
    } else {
        output(&parts.join(" * "));
        if factors.len() == 1 && factors[0].degree().unwrap_or(0) > 1 {
            output("(irreducible over \\mathbb{Q})");
        }
    }
}

fn repl_limit(rest: &str) {
    let args_owned = split_args(rest);
    let args: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".into());
    let point = args.get(2).copied().unwrap_or("0");
    match arithma::limits::limit_latex_str(args[0], &var, point) {
        Ok(r) => output(&r),
        Err(e) => print_error(&format!("Error: {e}")),
    }
}

fn repl_taylor(rest: &str) {
    let args_owned = split_args(rest);
    let args: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
    let var = args
        .get(1)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".into());
    let order = args
        .get(3)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(5);
    let center = args.get(2).copied().unwrap_or("0");

    if let Ok(center_f64) = center.parse::<f64>() {
        match arithma::series::taylor_series_latex(args[0], &var, center_f64, order) {
            Ok(r) => output(&r),
            Err(e) => print_error(&format!("Error: {e}")),
        }
    } else {
        let center_norm = normalize_var(center);
        match arithma::series::taylor_series_latex_symbolic(args[0], &var, &center_norm, order) {
            Ok(r) => output(&r),
            Err(e) => print_error(&format!("Error: {e}")),
        }
    }
}

fn repl_eval(rest: &str) {
    let args_owned = split_args(rest);
    let args: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();

    let mut tokenizer = Tokenizer::new(args[0]);
    let tokens = tokenizer.tokenize();
    let expr = match build_expression_tree(tokens) {
        Ok(e) => e,
        Err(e) => {
            print_error(&format!("Error: {e}"));
            return;
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
                print_error(&format!("Invalid value for {var}: {val_str}"));
                return;
            }
        }
    }

    match Evaluator::evaluate_exact(&simplified, &env) {
        Ok(val) => output(&format!("{}", Node::Num(val))),
        Err(_) => match Evaluator::evaluate(&simplified, &env) {
            Ok(val) => output(&format!("{val}")),
            Err(_) => output(&format!("{simplified}")),
        },
    }
}

fn repl_sub(rest: &str, env: &Environment) {
    let args_owned = split_args(rest);
    let args: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
    if args.len() < 3 {
        print_note("Usage: sub <expr> <var> <value>");
        return;
    }
    let var = normalize_var(args[1]);
    let subs = vec![(var, args[2].to_string())];
    match arithma::substitute::substitute_latex(args[0], &subs) {
        Ok(result) => match parse_latex(&result, env).map(|n| format!("{n}")) {
            Ok(simplified) => output(&simplified),
            Err(_) => output(&result),
        },
        Err(e) => print_error(&format!("Error: {e}")),
    }
}

fn repl_ode(rest: &str) {
    let args_owned = split_args(rest);
    let args: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
    if args[0] == "--cc" {
        if args.len() < 4 {
            print_note("Usage: ode --cc <a> <b> <c> [indep]");
            return;
        }
        let a: f64 = match args[1].parse() {
            Ok(v) => v,
            Err(_) => {
                print_error(&format!("Invalid coefficient: {}", args[1]));
                return;
            }
        };
        let b: f64 = match args[2].parse() {
            Ok(v) => v,
            Err(_) => {
                print_error(&format!("Invalid coefficient: {}", args[2]));
                return;
            }
        };
        let c: f64 = match args[3].parse() {
            Ok(v) => v,
            Err(_) => {
                print_error(&format!("Invalid coefficient: {}", args[3]));
                return;
            }
        };
        let indep = args
            .get(4)
            .map(|s| normalize_var(s))
            .unwrap_or_else(|| "x".into());
        match arithma::ode::solve_constant_coeff_latex(a, b, c, &indep) {
            Ok(r) => output(&r),
            Err(e) => print_error(&format!("Error: {e}")),
        }
    } else {
        let indep = args
            .get(1)
            .map(|s| normalize_var(s))
            .unwrap_or_else(|| "x".into());
        let dep = args
            .get(2)
            .map(|s| normalize_var(s))
            .unwrap_or_else(|| "y".into());
        match arithma::ode::solve_ode_latex(args[0], &indep, &dep) {
            Ok(r) => output(&r),
            Err(e) => print_error(&format!("Error: {e}")),
        }
    }
}

fn repl_prime_factorize(rest: &str) {
    match rest.trim().parse::<u64>() {
        Ok(n) => output(&arithma::prime_factorize_latex(n)),
        Err(_) => print_error("Error: expected a non-negative integer"),
    }
}

fn repl_pf(rest: &str) {
    let args_owned = split_args(rest);
    let args: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
    if args.len() < 2 {
        print_note("Usage: pf <numerator> <denominator> [var]");
        return;
    }
    let var = args
        .get(2)
        .map(|s| normalize_var(s))
        .unwrap_or_else(|| "x".into());
    match arithma::partial_fractions::partial_fractions_latex(args[0], args[1], &var) {
        Ok(r) => output(&r),
        Err(e) => print_error(&format!("Error: {e}")),
    }
}

fn repl_expr(input: &str, env: &Environment) {
    if input.contains("\\begin{pmatrix}")
        && input.contains("\\cdot")
        && input.contains("\\end{pmatrix}")
    {
        let parts: Vec<&str> = input.split("\\cdot").collect();
        if parts.len() == 2 {
            match (
                arithma::matrix::parse_latex_matrix(parts[0].trim(), env),
                arithma::matrix::parse_latex_matrix(parts[1].trim(), env),
            ) {
                (Ok(a), Ok(b)) => match a.multiply(&b, env) {
                    Ok(result) => {
                        output(&result.to_latex());
                        return;
                    }
                    Err(e) => {
                        print_error(&format!("Error: {e}"));
                        return;
                    }
                },
                (Err(e), _) | (_, Err(e)) => {
                    print_error(&format!("Error: {e}"));
                    return;
                }
            }
        }
    }

    let simplified = match parse_latex(input, env) {
        Ok(node) => node,
        Err(e) => {
            print_error(&format!("Error: {e}"));
            return;
        }
    };

    // Try exact rational evaluation (e.g., 1/3+1/4 → 7/12)
    if let Ok(arithma::ExactNum::Rational(ref r)) = Evaluator::evaluate_exact(&simplified, env) {
        let val = arithma::ExactNum::Rational(r.clone());
        output(&format!("{}", Node::Num(val)));
        return;
    }

    let simplified_str = format!("{simplified}");

    // If simplification produced a fully-reduced form (no unevaluated
    // trig/log/etc.), prefer it over a float approximation.
    // e.g., sin(pi/4) → √2/2 rather than 0.7071...
    if !has_unevaluated_functions(&simplified_str) {
        output(&simplified_str);
        return;
    }

    // Fall back to float for expressions with unevaluated functions
    // e.g., sin(1) → 0.8414...
    match Evaluator::evaluate(&simplified, env) {
        Ok(val) => output(&format!("{val}")),
        Err(_) => output(&simplified_str),
    }
}

fn has_unevaluated_functions(s: &str) -> bool {
    [
        "\\sin", "\\cos", "\\tan", "\\sec", "\\csc", "\\cot", "\\ln", "\\log", "\\exp", "\\arctan",
        "\\arcsin", "\\arccos", "\\sinh", "\\cosh", "\\tanh", "\\erf", "\\Ei", "\\li", "\\lim",
    ]
    .iter()
    .any(|f| s.contains(f))
}

fn history_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join(".arithma_history"))
}

fn repl() {
    let ver = env!("CARGO_PKG_VERSION");
    if color_enabled() {
        println!(
            "{}{}Arithma v{ver}{} — interactive mode",
            ansi::BOLD,
            ansi::CYAN,
            ansi::RESET
        );
        println!(
            "{}Commands: simplify, diff, integrate, solve, factor, limit, taylor, eval{}",
            ansi::DIM,
            ansi::RESET
        );
        println!(
            "{}Type 'help' for details, 'exit' to quit.{}\n",
            ansi::DIM,
            ansi::RESET
        );
    } else {
        println!("Arithma v{ver} — interactive mode");
        println!("Commands: simplify, diff, integrate, solve, factor, limit, taylor, eval");
        println!("Type 'help' for details, 'exit' to quit.\n");
    }

    let mut rl = DefaultEditor::new().unwrap();
    if let Some(ref path) = history_path() {
        let _ = rl.load_history(path);
    }

    let env = Environment::new();
    let prompt = if color_enabled() {
        format!(
            "\x01{}{}\x02>>\x01{}\x02 ",
            ansi::BOLD,
            ansi::CYAN,
            ansi::RESET
        )
    } else {
        ">> ".to_string()
    };

    loop {
        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                if input == "exit" || input == "quit" {
                    break;
                }

                let _ = rl.add_history_entry(input);

                if input == "latex" {
                    LATEX_OUTPUT.store(true, Ordering::Relaxed);
                    print_note("Output: LaTeX");
                    continue;
                }
                if input == "unicode" {
                    LATEX_OUTPUT.store(false, Ordering::Relaxed);
                    print_note("Output: Unicode");
                    continue;
                }

                if input == "help" || input == "--help" {
                    print_repl_help();
                    continue;
                }

                let input = preprocess_input(input);

                let (cmd, rest) = match input.find(char::is_whitespace) {
                    Some(pos) => (&input[..pos], input[pos..].trim_start()),
                    None => (input.as_str(), ""),
                };

                match cmd {
                    "format" if !rest.is_empty() => repl_format(rest),
                    "simplify" if !rest.is_empty() => repl_simplify(rest, &env),
                    "diff" | "differentiate" if !rest.is_empty() => repl_diff(rest),
                    "integrate" if !rest.is_empty() => repl_integrate(rest),
                    "solve" if !rest.is_empty() => repl_solve(rest),
                    "factor" if !rest.is_empty() => repl_factor(rest),
                    "limit" if !rest.is_empty() => repl_limit(rest),
                    "taylor" if !rest.is_empty() => repl_taylor(rest),
                    "eval" | "evaluate" if !rest.is_empty() => repl_eval(rest),
                    "sub" | "substitute" if !rest.is_empty() => repl_sub(rest, &env),
                    "ode" if !rest.is_empty() => repl_ode(rest),
                    "prime-factorize" | "factorint" if !rest.is_empty() => {
                        repl_prime_factorize(rest)
                    }
                    "pf" | "partial-fractions" if !rest.is_empty() => repl_pf(rest),
                    "format" | "simplify" | "diff" | "differentiate" | "integrate" | "solve"
                    | "factor" | "limit" | "taylor" | "eval" | "evaluate" | "sub"
                    | "substitute" | "ode" | "prime-factorize" | "factorint" | "pf"
                    | "partial-fractions" => {
                        print_note(&format!(
                            "Usage: {cmd} <expr> [args...] — type 'help' for details"
                        ));
                    }
                    _ => repl_expr(&input, &env),
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => break,
            Err(e) => {
                print_error(&format!("Error: {e}"));
                break;
            }
        }
    }

    if let Some(ref path) = history_path() {
        let _ = rl.save_history(path);
    }
}

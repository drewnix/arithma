use num_rational::BigRational;
use num_traits::{One, Zero};

use crate::derivative::differentiate;
use crate::environment::Environment;
use crate::exact::ExactNum;
use crate::expression::solve_full;
use crate::node::Node;
use crate::simplify::Simplifiable;
use crate::substitute::substitute_variable;

#[derive(Debug, Clone)]
pub enum SystemSolution {
    Unique(Vec<(String, Node)>),
    Multiple(Vec<Vec<(String, Node)>>),
    Parametric {
        solutions: Vec<(String, Node)>,
        free_vars: Vec<String>,
    },
    NoSolution,
}

pub fn solve_linear_system(equations: &[Node], vars: &[String]) -> Result<SystemSolution, String> {
    let n_var = vars.len();
    if vars.is_empty() {
        return Err("No variables specified".to_string());
    }
    if equations.is_empty() {
        return Err("No equations provided".to_string());
    }

    let env = Environment::new();
    let mut augmented: Vec<Vec<BigRational>> = Vec::new();

    for eq_node in equations {
        let expr = match eq_node {
            Node::Equation(lhs, rhs) => Node::Subtract(lhs.clone(), rhs.clone())
                .simplify(&env)
                .unwrap_or_else(|_| Node::Subtract(lhs.clone(), rhs.clone())),
            _ => return Err("Expected an equation (contains '=')".to_string()),
        };

        let mut row: Vec<BigRational> = Vec::with_capacity(n_var + 1);

        for var in vars {
            let deriv = differentiate(&expr, var)
                .map_err(|e| format!("Cannot differentiate with respect to {}: {}", var, e))?;
            let deriv = deriv.simplify(&env).unwrap_or(deriv);

            for v in vars {
                if contains_var(&deriv, v) {
                    return Err(format!(
                        "System is not linear: equation involves {} in a nonlinear way",
                        v
                    ));
                }
            }

            let coeff = node_to_rational(&deriv).ok_or_else(|| {
                format!("Coefficient of {} is not a rational number: {}", var, deriv)
            })?;
            row.push(coeff);
        }

        let zero = Node::Num(ExactNum::integer(0));
        let mut const_expr = expr.clone();
        for var in vars {
            const_expr = substitute_variable(&const_expr, var, &zero).unwrap_or(const_expr);
        }
        let const_expr = const_expr.simplify(&env).unwrap_or(const_expr);

        let c = node_to_rational(&const_expr)
            .ok_or_else(|| format!("Constant term is not a rational number: {}", const_expr))?;

        row.push(-c);
        augmented.push(row);
    }

    gaussian_eliminate(&mut augmented, n_var, vars)
}

pub fn solve_system(equations: &[Node], vars: &[String]) -> Result<SystemSolution, String> {
    if vars.is_empty() {
        return Err("No variables specified".to_string());
    }
    if equations.is_empty() {
        return Err("No equations provided".to_string());
    }

    if let Ok(solution) = solve_linear_system(equations, vars) {
        return Ok(solution);
    }

    solve_by_substitution(equations, vars)
}

fn solve_by_substitution(equations: &[Node], vars: &[String]) -> Result<SystemSolution, String> {
    if equations.len() == 1 && vars.len() == 1 {
        let result = solve_full(&equations[0], &vars[0])?;
        if result.solutions.is_empty() {
            return Ok(SystemSolution::NoSolution);
        }
        if result.solutions.len() == 1 {
            return Ok(SystemSolution::Unique(vec![(
                vars[0].clone(),
                result.solutions.into_iter().next().unwrap(),
            )]));
        }
        let solution_sets: Vec<Vec<(String, Node)>> = result
            .solutions
            .into_iter()
            .map(|s| vec![(vars[0].clone(), s)])
            .collect();
        return Ok(SystemSolution::Multiple(solution_sets));
    }

    let env = Environment::new();

    for (eq_idx, eq) in equations.iter().enumerate() {
        for var in vars {
            if is_linear_in(eq, var, vars) {
                let expr = match eq {
                    Node::Equation(lhs, rhs) => Node::Subtract(lhs.clone(), rhs.clone()),
                    _ => continue,
                };

                let solved = match solve_linear_for_var(&expr, var, vars, &env) {
                    Some(s) => s,
                    None => continue,
                };

                let remaining_vars: Vec<String> =
                    vars.iter().filter(|v| *v != var).cloned().collect();

                let mut reduced_eqs: Vec<Node> = Vec::new();
                for (i, other_eq) in equations.iter().enumerate() {
                    if i == eq_idx {
                        continue;
                    }
                    let substituted = match other_eq {
                        Node::Equation(lhs, rhs) => {
                            let new_lhs =
                                substitute_variable(lhs, var, &solved).unwrap_or(*lhs.clone());
                            let new_rhs =
                                substitute_variable(rhs, var, &solved).unwrap_or(*rhs.clone());
                            let new_lhs = new_lhs.simplify(&env).unwrap_or(new_lhs);
                            let new_rhs = new_rhs.simplify(&env).unwrap_or(new_rhs);
                            Node::Equation(Box::new(new_lhs), Box::new(new_rhs))
                        }
                        _ => continue,
                    };
                    reduced_eqs.push(substituted);
                }

                if remaining_vars.is_empty() {
                    let back_sub = solved.simplify(&env).unwrap_or(solved);
                    return Ok(SystemSolution::Unique(vec![(var.clone(), back_sub)]));
                }

                let sub_result = solve_system(&reduced_eqs, &remaining_vars)?;

                let sub_solution_sets = match sub_result {
                    SystemSolution::Unique(s) => vec![s],
                    SystemSolution::Multiple(sets) => sets,
                    SystemSolution::NoSolution => {
                        return Ok(SystemSolution::NoSolution);
                    }
                    SystemSolution::Parametric { .. } => {
                        return Ok(sub_result);
                    }
                };

                let mut all_sets: Vec<Vec<(String, Node)>> = Vec::new();

                for sub_solutions in &sub_solution_sets {
                    let mut val = solved.clone();
                    for (sv, sval) in sub_solutions {
                        val = substitute_variable(&val, sv, sval).unwrap_or(val);
                    }
                    let val = val.simplify(&env).unwrap_or(val);

                    let mut solution_set = vec![(var.clone(), val)];
                    solution_set.extend(sub_solutions.clone());

                    solution_set.sort_by(|a, b| {
                        let a_idx = vars.iter().position(|v| v == &a.0).unwrap_or(usize::MAX);
                        let b_idx = vars.iter().position(|v| v == &b.0).unwrap_or(usize::MAX);
                        a_idx.cmp(&b_idx)
                    });

                    all_sets.push(solution_set);
                }

                if all_sets.is_empty() {
                    return Ok(SystemSolution::NoSolution);
                }
                if all_sets.len() == 1 {
                    return Ok(SystemSolution::Unique(all_sets.into_iter().next().unwrap()));
                }
                return Ok(SystemSolution::Multiple(all_sets));
            }
        }
    }

    Err("Cannot solve: no equation is linear in any variable (try a system where at least one equation is linear)".to_string())
}

fn is_linear_in(eq: &Node, var: &str, all_vars: &[String]) -> bool {
    let expr = match eq {
        Node::Equation(lhs, rhs) => Node::Subtract(lhs.clone(), rhs.clone()),
        _ => return false,
    };

    let env = Environment::new();

    let deriv = match differentiate(&expr, var) {
        Ok(d) => d.simplify(&env).unwrap_or(d),
        Err(_) => return false,
    };

    if contains_var(&deriv, var) {
        return false;
    }

    for v in all_vars {
        if v == var {
            continue;
        }
        if contains_var(&deriv, v) {
            return false;
        }
    }

    let zero_check = node_to_rational(&deriv);
    matches!(zero_check, Some(r) if !r.is_zero())
}

fn solve_linear_for_var(
    expr: &Node,
    var: &str,
    _all_vars: &[String],
    env: &Environment,
) -> Option<Node> {
    let deriv = differentiate(expr, var).ok()?;
    let deriv = deriv.simplify(env).unwrap_or(deriv);
    let coeff = node_to_rational(&deriv)?;
    if coeff.is_zero() {
        return None;
    }

    let zero = Node::Num(ExactNum::integer(0));
    let rest = substitute_variable(expr, var, &zero).unwrap_or_else(|_| expr.clone());
    let rest = rest.simplify(env).unwrap_or(rest);

    let neg_rest = Node::Negate(Box::new(rest));
    let coeff_node = rational_to_node(&coeff);
    let result = Node::Divide(Box::new(neg_rest), Box::new(coeff_node));
    let result = result.simplify(env).unwrap_or(result);
    Some(result)
}

fn node_to_rational(node: &Node) -> Option<BigRational> {
    match node {
        Node::Num(e) => e.to_rational(),
        Node::Negate(inner) => node_to_rational(inner).map(|r| -r),
        Node::Divide(a, b) => {
            let ra = node_to_rational(a)?;
            let rb = node_to_rational(b)?;
            if rb.is_zero() {
                None
            } else {
                Some(ra / rb)
            }
        }
        Node::Multiply(a, b) => {
            let ra = node_to_rational(a)?;
            let rb = node_to_rational(b)?;
            Some(ra * rb)
        }
        Node::Add(a, b) => {
            let ra = node_to_rational(a)?;
            let rb = node_to_rational(b)?;
            Some(ra + rb)
        }
        Node::Subtract(a, b) => {
            let ra = node_to_rational(a)?;
            let rb = node_to_rational(b)?;
            Some(ra - rb)
        }
        _ => None,
    }
}

fn contains_var(node: &Node, var: &str) -> bool {
    match node {
        Node::Variable(v) => v == var,
        Node::Num(_) => false,
        Node::Add(a, b)
        | Node::Subtract(a, b)
        | Node::Multiply(a, b)
        | Node::Divide(a, b)
        | Node::Power(a, b) => contains_var(a, var) || contains_var(b, var),
        Node::Negate(inner) | Node::Sqrt(inner) | Node::Abs(inner) | Node::Factorial(inner) => {
            contains_var(inner, var)
        }
        Node::Function(_, args) => args.iter().any(|a| contains_var(a, var)),
        Node::Equation(a, b)
        | Node::Greater(a, b)
        | Node::Less(a, b)
        | Node::GreaterEqual(a, b)
        | Node::LessEqual(a, b)
        | Node::Equal(a, b) => contains_var(a, var) || contains_var(b, var),
        Node::Summation(_, start, end, body) => {
            contains_var(start, var) || contains_var(end, var) || contains_var(body, var)
        }
        Node::Product(_, start, end, body) => {
            contains_var(start, var) || contains_var(end, var) || contains_var(body, var)
        }
        Node::Piecewise(cases) => cases
            .iter()
            .any(|(val, cond)| contains_var(val, var) || contains_var(cond, var)),
    }
}

fn rational_to_node(r: &BigRational) -> Node {
    if r.denom().is_one() {
        let n: i64 = r.numer().try_into().unwrap_or(0);
        Node::Num(ExactNum::integer(n))
    } else {
        let n: i64 = r.numer().try_into().unwrap_or(0);
        let d: i64 = r.denom().try_into().unwrap_or(1);
        Node::Num(ExactNum::rational(n, d))
    }
}

fn gaussian_eliminate(
    aug: &mut [Vec<BigRational>],
    n_var: usize,
    vars: &[String],
) -> Result<SystemSolution, String> {
    let n_eq = aug.len();

    let mut pivot_col = vec![None; n_eq];
    let mut pivot_row = 0;

    for col in 0..n_var {
        let found = aug[pivot_row..]
            .iter()
            .position(|row| !row[col].is_zero())
            .map(|offset| offset + pivot_row);

        let Some(swap_row) = found else {
            continue;
        };

        if swap_row != pivot_row {
            aug.swap(swap_row, pivot_row);
        }

        pivot_col[pivot_row] = Some(col);

        let pivot_val = aug[pivot_row][col].clone();
        for entry in aug[pivot_row].iter_mut() {
            *entry = &*entry / &pivot_val;
        }

        for i in 0..n_eq {
            if i == pivot_row {
                continue;
            }
            let factor = aug[i][col].clone();
            if !factor.is_zero() {
                let pivot_row_copy: Vec<BigRational> = aug[pivot_row].clone();
                for (entry, pval) in aug[i].iter_mut().zip(pivot_row_copy.iter()) {
                    let sub = &factor * pval;
                    *entry = &*entry - &sub;
                }
            }
        }

        pivot_row += 1;
    }

    let rank = pivot_row;

    for row in aug.iter().take(n_eq).skip(rank) {
        if !row[n_var].is_zero() {
            return Ok(SystemSolution::NoSolution);
        }
    }

    if rank == n_var {
        let solutions: Vec<(String, Node)> = (0..n_var)
            .map(|i| {
                let val = rational_to_node(&aug[i][n_var]);
                (vars[i].clone(), val)
            })
            .collect();
        return Ok(SystemSolution::Unique(solutions));
    }

    let pivot_cols: Vec<Option<usize>> = pivot_col[..rank].to_vec();
    let bound_vars: Vec<usize> = pivot_cols.iter().filter_map(|c| *c).collect();
    let free_var_indices: Vec<usize> = (0..n_var).filter(|c| !bound_vars.contains(c)).collect();

    let free_vars: Vec<String> = free_var_indices.iter().map(|&i| vars[i].clone()).collect();

    let env = Environment::new();
    let mut solutions: Vec<(String, Node)> = Vec::new();

    for &free_idx in &free_var_indices {
        solutions.push((
            vars[free_idx].clone(),
            Node::Variable(vars[free_idx].clone()),
        ));
    }

    for i in 0..rank {
        if let Some(col) = pivot_cols[i] {
            let mut expr = rational_to_node(&aug[i][n_var]);

            for &free_idx in &free_var_indices {
                let coeff = &aug[i][free_idx];
                if !coeff.is_zero() {
                    let neg_coeff = -coeff;
                    let term = if neg_coeff.is_one() {
                        Node::Variable(vars[free_idx].clone())
                    } else {
                        Node::Multiply(
                            Box::new(rational_to_node(&neg_coeff)),
                            Box::new(Node::Variable(vars[free_idx].clone())),
                        )
                    };
                    expr = Node::Add(Box::new(expr), Box::new(term));
                }
            }

            let expr = expr.simplify(&env).unwrap_or(expr);
            solutions.push((vars[col].clone(), expr));
        }
    }

    solutions.sort_by(|a, b| {
        let a_idx = vars.iter().position(|v| v == &a.0).unwrap_or(usize::MAX);
        let b_idx = vars.iter().position(|v| v == &b.0).unwrap_or(usize::MAX);
        a_idx.cmp(&b_idx)
    });

    Ok(SystemSolution::Parametric {
        solutions,
        free_vars,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_expression_tree, Tokenizer};

    fn parse(s: &str) -> Node {
        let mut t = Tokenizer::new(s);
        let tokens = t.tokenize();
        build_expression_tree(tokens).unwrap()
    }

    fn solve_sys(eqs: &[&str], vars: &[&str]) -> Result<SystemSolution, String> {
        let equations: Vec<Node> = eqs.iter().map(|e| parse(e)).collect();
        let var_strings: Vec<String> = vars.iter().map(|v| v.to_string()).collect();
        solve_linear_system(&equations, &var_strings)
    }

    fn assert_unique(result: &SystemSolution, expected: &[(&str, &str)]) {
        match result {
            SystemSolution::Unique(solutions) => {
                assert_eq!(
                    solutions.len(),
                    expected.len(),
                    "Expected {} solutions, got {}",
                    expected.len(),
                    solutions.len()
                );
                for (var, expected_val) in expected {
                    let found = solutions
                        .iter()
                        .find(|(v, _)| v == var)
                        .unwrap_or_else(|| panic!("Variable {} not in solution", var));
                    let expected_node = parse(expected_val);
                    assert_eq!(
                        format!("{}", found.1),
                        format!("{}", expected_node),
                        "Variable {} = {} but expected {}",
                        var,
                        found.1,
                        expected_val
                    );
                }
            }
            SystemSolution::NoSolution => panic!("Expected unique solution, got no solution"),
            SystemSolution::Multiple(_) => panic!("Expected unique solution, got multiple"),
            SystemSolution::Parametric { .. } => {
                panic!("Expected unique solution, got parametric")
            }
        }
    }

    #[test]
    fn test_2x2_basic() {
        let result = solve_sys(&["x + y = 3", "x - y = 1"], &["x", "y"]).unwrap();
        assert_unique(&result, &[("x", "2"), ("y", "1")]);
    }

    #[test]
    fn test_2x2_with_coefficients() {
        let result = solve_sys(&["2x + 3y = 8", "x - y = 1"], &["x", "y"]).unwrap();
        assert_unique(&result, &[("x", "\\frac{11}{5}"), ("y", "\\frac{6}{5}")]);
    }

    #[test]
    fn test_3x3_system() {
        let result = solve_sys(
            &["x + y + z = 6", "2x - y + z = 3", "x + 2y - z = 2"],
            &["x", "y", "z"],
        )
        .unwrap();
        assert_unique(&result, &[("x", "1"), ("y", "2"), ("z", "3")]);
    }

    #[test]
    fn test_2x2_fractional_coefficients() {
        let result = solve_sys(&["x/2 + y/3 = 1", "x - y = 1"], &["x", "y"]).unwrap();
        assert_unique(&result, &[("x", "\\frac{8}{5}"), ("y", "\\frac{3}{5}")]);
    }

    #[test]
    fn test_inconsistent_system() {
        let result = solve_sys(&["x + y = 1", "x + y = 2"], &["x", "y"]).unwrap();
        assert!(matches!(result, SystemSolution::NoSolution));
    }

    #[test]
    fn test_underdetermined_system() {
        let result = solve_sys(&["x + y + z = 6"], &["x", "y", "z"]).unwrap();
        match &result {
            SystemSolution::Parametric { free_vars, .. } => {
                assert_eq!(free_vars.len(), 2);
            }
            _ => panic!("Expected parametric solution"),
        }
    }

    #[test]
    fn test_4x4_system() {
        let result = solve_sys(
            &[
                "w + x + y + z = 10",
                "w - x + y - z = 0",
                "2w + x - y + z = 5",
                "w + 2x + 3y - z = 10",
            ],
            &["w", "x", "y", "z"],
        )
        .unwrap();
        match &result {
            SystemSolution::Unique(solutions) => {
                assert_eq!(solutions.len(), 4);
            }
            _ => panic!("Expected unique solution for 4x4 system"),
        }
    }

    #[test]
    fn test_nonlinear_rejected() {
        let result = solve_sys(&["x^2 + y = 1", "x + y = 1"], &["x", "y"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cross_term_rejected() {
        let result = solve_sys(&["x*y = 1", "x + y = 2"], &["x", "y"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_trivial_1x1() {
        let result = solve_sys(&["3x = 9"], &["x"]).unwrap();
        assert_unique(&result, &[("x", "3")]);
    }

    #[test]
    fn test_zero_coefficient() {
        let result = solve_sys(&["x = 3", "y = 5"], &["x", "y"]).unwrap();
        assert_unique(&result, &[("x", "3"), ("y", "5")]);
    }

    #[test]
    fn test_negative_solution() {
        let result = solve_sys(&["x + y + 1 = 0", "x - y = 3"], &["x", "y"]).unwrap();
        assert_unique(&result, &[("x", "1"), ("y", "-2")]);
    }

    #[test]
    fn test_overdetermined_consistent() {
        let result = solve_sys(&["x + y = 3", "x - y = 1", "2x = 4"], &["x", "y"]).unwrap();
        assert_unique(&result, &[("x", "2"), ("y", "1")]);
    }

    #[test]
    fn test_overdetermined_inconsistent() {
        let result = solve_sys(&["x + y = 3", "x - y = 1", "x + y = 5"], &["x", "y"]).unwrap();
        assert!(matches!(result, SystemSolution::NoSolution));
    }

    // --- Nonlinear (polynomial) system tests ---

    fn solve_sys_poly(eqs: &[&str], vars: &[&str]) -> Result<SystemSolution, String> {
        let equations: Vec<Node> = eqs.iter().map(|e| parse(e)).collect();
        let var_strings: Vec<String> = vars.iter().map(|v| v.to_string()).collect();
        solve_system(&equations, &var_strings)
    }

    #[test]
    fn test_nonlinear_2x2_sum_product() {
        let result = solve_sys_poly(&["x + y = 5", "x * y = 6"], &["x", "y"]).unwrap();
        match &result {
            SystemSolution::Multiple(sets) => {
                assert_eq!(sets.len(), 2);
                let mut vals: Vec<(String, String)> = sets
                    .iter()
                    .map(|s| {
                        let x = s.iter().find(|(v, _)| v == "x").unwrap();
                        let y = s.iter().find(|(v, _)| v == "y").unwrap();
                        (format!("{}", x.1), format!("{}", y.1))
                    })
                    .collect();
                vals.sort();
                assert_eq!(vals[0], ("2".to_string(), "3".to_string()));
                assert_eq!(vals[1], ("3".to_string(), "2".to_string()));
            }
            _ => panic!("Expected multiple solutions, got {:?}", result),
        }
    }

    #[test]
    fn test_nonlinear_2x2_circle_line() {
        let result = solve_sys_poly(&["x + y = 1", "x^2 + y^2 = 1"], &["x", "y"]).unwrap();
        match &result {
            SystemSolution::Multiple(sets) => {
                assert_eq!(sets.len(), 2);
            }
            SystemSolution::Unique(_) => {
                panic!("Expected 2 solutions for circle-line intersection")
            }
            _ => panic!("Expected solutions, got {:?}", result),
        }
    }

    #[test]
    fn test_nonlinear_falls_back_from_linear() {
        let result = solve_sys_poly(&["x + y = 3", "x - y = 1"], &["x", "y"]).unwrap();
        assert_unique(&result, &[("x", "2"), ("y", "1")]);
    }

    #[test]
    fn test_nonlinear_3x3_one_quadratic() {
        let result = solve_sys_poly(
            &["x + y + z = 6", "x - y = 0", "x^2 + y^2 + z^2 = 14"],
            &["x", "y", "z"],
        )
        .unwrap();
        match &result {
            SystemSolution::Unique(solutions) => {
                assert_eq!(solutions.len(), 3);
            }
            SystemSolution::Multiple(sets) => {
                assert!(!sets.is_empty());
                for s in sets {
                    assert_eq!(s.len(), 3);
                }
            }
            _ => panic!("Expected solutions, got {:?}", result),
        }
    }
}

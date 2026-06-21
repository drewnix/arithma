use crate::environment::Environment;
use crate::exact::ExactNum;
use crate::node::Node;
use crate::polynomial::Polynomial;
use crate::simplify::Simplifiable;
use num_rational::BigRational;
use num_traits::{Signed, Zero};

#[derive(Debug, Clone, Copy)]
enum IneqType {
    Gt,
    Ge,
    Lt,
    Le,
}

impl IneqType {
    fn includes_zero(self) -> bool {
        matches!(self, IneqType::Ge | IneqType::Le)
    }

}

fn sign_satisfies(val: &BigRational, ineq: IneqType) -> bool {
    match ineq {
        IneqType::Gt => val.is_positive(),
        IneqType::Ge => !val.is_negative(),
        IneqType::Lt => val.is_negative(),
        IneqType::Le => !val.is_positive(),
    }
}

#[derive(Debug, Clone)]
struct CritPoint {
    value: BigRational,
    display: String,
    is_pole: bool,
}

pub fn solve_inequality(expr: &Node, target_var: &str) -> Result<String, String> {
    let (lhs, rhs, ineq_type) = match expr {
        Node::Greater(l, r) => (l.as_ref(), r.as_ref(), IneqType::Gt),
        Node::GreaterEqual(l, r) => (l.as_ref(), r.as_ref(), IneqType::Ge),
        Node::Less(l, r) => (l.as_ref(), r.as_ref(), IneqType::Lt),
        Node::LessEqual(l, r) => (l.as_ref(), r.as_ref(), IneqType::Le),
        _ => return Err("Expected an inequality (>, >=, <, <=)".to_string()),
    };

    let diff = Node::Subtract(Box::new(lhs.clone()), Box::new(rhs.clone()));
    let env = Environment::new();
    let simplified = diff.simplify(&env).unwrap_or(diff);

    if let Ok(poly) = Polynomial::from_node(&simplified, target_var) {
        return solve_poly_inequality(&poly, ineq_type);
    }

    if let Some((num, den)) = crate::expression::to_rational_form(&simplified) {
        let num_s = num.simplify(&env).unwrap_or(num);
        let den_s = den.simplify(&env).unwrap_or(den);

        if let (Ok(num_poly), Ok(den_poly)) = (
            Polynomial::from_node(&num_s, target_var),
            Polynomial::from_node(&den_s, target_var),
        ) {
            if !den_poly.is_constant() {
                return solve_rational_inequality(&num_poly, &den_poly, ineq_type);
            }
            let den_sign = den_poly.coeff(0);
            if den_sign.is_negative() {
                let flipped = match ineq_type {
                    IneqType::Gt => IneqType::Lt,
                    IneqType::Ge => IneqType::Le,
                    IneqType::Lt => IneqType::Gt,
                    IneqType::Le => IneqType::Ge,
                };
                let neg_num = num_poly.scalar_mul(&BigRational::from_integer((-1).into()));
                return solve_poly_inequality(&neg_num, flipped);
            }
            return solve_poly_inequality(&num_poly, ineq_type);
        }
    }

    Err("Cannot solve this inequality (not polynomial or rational in the given variable)".to_string())
}

fn find_rational_and_irrational_roots(poly: &Polynomial) -> Vec<CritPoint> {
    let mut points = Vec::new();

    let rat_roots = poly.rational_roots();
    let mut remaining = poly.clone();

    for root in &rat_roots {
        let display = format!("{}", Node::Num(exact_from_rational(root)));
        points.push(CritPoint {
            value: root.clone(),
            display,
            is_pole: false,
        });
        remaining = remaining.deflate(root);
    }

    match remaining.degree() {
        None | Some(0) => {}
        Some(1) => {
            let a = remaining.coeff(1);
            let b = remaining.coeff(0);
            let root = -b / a;
            let display = format!("{}", Node::Num(exact_from_rational(&root)));
            points.push(CritPoint {
                value: root,
                display,
                is_pole: false,
            });
        }
        Some(2) => {
            let a = remaining.coeff(2);
            let b = remaining.coeff(1);
            let c = remaining.coeff(0);
            let disc = &b * &b - BigRational::from_integer(4.into()) * &a * &c;
            if disc.is_positive() {
                let disc_f64 = to_f64(&disc);
                let sqrt_disc = disc_f64.sqrt();
                let a_f64 = to_f64(&a);
                let b_f64 = to_f64(&b);

                let r1 = (-b_f64 - sqrt_disc) / (2.0 * a_f64);
                let r2 = (-b_f64 + sqrt_disc) / (2.0 * a_f64);

                let r1_exact = ExactNum::from_f64(r1);
                let r2_exact = ExactNum::from_f64(r2);

                if let Some(r1_rat) = r1_exact.to_rational() {
                    points.push(CritPoint {
                        value: r1_rat,
                        display: format!("{}", Node::Num(r1_exact)),
                        is_pole: false,
                    });
                } else {
                    points.push(CritPoint {
                        value: BigRational::from_float(r1).unwrap_or_default(),
                        display: format!("{}", Node::Num(r1_exact)),
                        is_pole: false,
                    });
                }

                if let Some(r2_rat) = r2_exact.to_rational() {
                    points.push(CritPoint {
                        value: r2_rat,
                        display: format!("{}", Node::Num(r2_exact)),
                        is_pole: false,
                    });
                } else {
                    points.push(CritPoint {
                        value: BigRational::from_float(r2).unwrap_or_default(),
                        display: format!("{}", Node::Num(r2_exact)),
                        is_pole: false,
                    });
                }
            } else if disc.is_zero() {
                let root = -b / (BigRational::from_integer(2.into()) * a);
                let display = format!("{}", Node::Num(exact_from_rational(&root)));
                points.push(CritPoint {
                    value: root,
                    display,
                    is_pole: false,
                });
            }
        }
        _ => {
            // For higher degree remainders, use f64 approximation
            // via the existing solve infrastructure
        }
    }

    points.sort_by(|a, b| a.value.cmp(&b.value));
    points.dedup_by(|a, b| a.value == b.value);
    points
}

fn solve_poly_inequality(poly: &Polynomial, ineq: IneqType) -> Result<String, String> {
    let degree = poly.degree();

    if degree.is_none() || degree == Some(0) {
        let c = poly.coeff(0);
        let sat = sign_satisfies(&c, ineq);
        return Ok(if sat {
            "(-∞, ∞)".to_string()
        } else {
            "∅".to_string()
        });
    }

    let points = find_rational_and_irrational_roots(poly);

    if points.is_empty() {
        let val = poly.evaluate(&BigRational::zero());
        return Ok(if sign_satisfies(&val, ineq) {
            "(-∞, ∞)".to_string()
        } else {
            "∅".to_string()
        });
    }

    build_solution_intervals(&points, |x| poly.evaluate(x), ineq)
}

fn solve_rational_inequality(
    num: &Polynomial,
    den: &Polynomial,
    ineq: IneqType,
) -> Result<String, String> {
    let mut points = find_rational_and_irrational_roots(num);
    let mut den_points = find_rational_and_irrational_roots(den);
    for p in &mut den_points {
        p.is_pole = true;
    }

    points.append(&mut den_points);
    points.sort_by(|a, b| a.value.cmp(&b.value));
    points.dedup_by(|a, b| {
        if a.value == b.value {
            b.is_pole = a.is_pole || b.is_pole;
            true
        } else {
            false
        }
    });

    build_solution_intervals(
        &points,
        |x| {
            let d = den.evaluate(x);
            if d.is_zero() {
                return BigRational::zero();
            }
            num.evaluate(x) / d
        },
        ineq,
    )
}

fn build_solution_intervals<F>(
    points: &[CritPoint],
    eval: F,
    ineq: IneqType,
) -> Result<String, String>
where
    F: Fn(&BigRational) -> BigRational,
{
    let includes_eq = ineq.includes_zero();

    let mut intervals: Vec<String> = Vec::new();

    // State for merging adjacent satisfied intervals
    let mut in_interval = false;
    let mut interval_start: Option<(String, bool)> = None; // (display, is_closed)

    // Test region before first root
    let first = &points[0];
    let test = &first.value - BigRational::from_integer(1.into());
    let val = eval(&test);
    let region_sat = sign_satisfies(&val, ineq);

    if region_sat {
        in_interval = true;
        interval_start = Some(("-∞".to_string(), false));
    }

    for (i, pt) in points.iter().enumerate() {
        let point_included = if pt.is_pole {
            false
        } else {
            includes_eq
        };

        if in_interval {
            if point_included {
                // Continue the interval through this point
            } else {
                // Close the interval before this point
                let (start_str, start_closed) = interval_start.take().unwrap();
                let left_br = if start_closed { "[" } else { "(" };
                let right_br = ")";
                intervals.push(format!(
                    "{}{}, {}{}",
                    left_br, start_str, pt.display, right_br
                ));
                in_interval = false;
            }
        } else if point_included {
            // Start a potential new interval at this isolated point
            interval_start = Some((pt.display.clone(), true));
            in_interval = true;
        }

        // Test region after this point (before next point or to +∞)
        let next_val = if i + 1 < points.len() {
            let next = &points[i + 1];
            let mid_num = &pt.value + &next.value;
            let mid = mid_num / BigRational::from_integer(2.into());
            eval(&mid)
        } else {
            let test = &pt.value + BigRational::from_integer(1.into());
            eval(&test)
        };
        let next_region_sat = sign_satisfies(&next_val, ineq);

        if in_interval && !next_region_sat {
            // Close the interval at this point
            let (start_str, start_closed) = interval_start.take().unwrap();
            let left_br = if start_closed { "[" } else { "(" };
            let right_closed = if pt.is_pole { false } else { includes_eq };
            let right_br = if right_closed { "]" } else { ")" };
            intervals.push(format!(
                "{}{}, {}{}",
                left_br, start_str, pt.display, right_br
            ));
            in_interval = false;
        } else if !in_interval && next_region_sat {
            // Start a new interval after this point
            let left_closed = if pt.is_pole { false } else { includes_eq };
            interval_start = Some((pt.display.clone(), left_closed));
            in_interval = true;
        }
    }

    // Close any remaining open interval
    if in_interval {
        let (start_str, start_closed) = interval_start.take().unwrap();
        let left_br = if start_closed { "[" } else { "(" };
        intervals.push(format!("{}{}, ∞)", left_br, start_str));
    }

    if intervals.is_empty() {
        Ok("∅".to_string())
    } else {
        Ok(intervals.join(" ∪ "))
    }
}

fn exact_from_rational(r: &BigRational) -> ExactNum {
    ExactNum::Rational(r.clone())
}

fn to_f64(r: &BigRational) -> f64 {
    use num_traits::ToPrimitive;
    r.numer().to_f64().unwrap_or(0.0) / r.denom().to_f64().unwrap_or(1.0)
}

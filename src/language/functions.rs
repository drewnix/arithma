use lazy_static::lazy_static;
use std::collections::HashMap;

use crate::exact::ExactNum;
use crate::integer::{binom, factorial, gcd, lcm};

// Define a trait for function handlers
pub trait FunctionHandler {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String>;

    // New method to return the number of arguments the function requires
    fn get_arg_count(&self) -> Option<usize>; // None for variable arguments
}

// Define the function registry that holds all functions
pub struct FunctionRegistry {
    functions: HashMap<String, Box<dyn FunctionHandler + Send + Sync>>,
}

impl Default for FunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionRegistry {
    // Create a new function registry (using lazy_static to ensure it's a singleton)
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    pub fn register_function(
        &mut self,
        name: &str,
        function: Box<dyn FunctionHandler + Send + Sync>,
    ) {
        self.functions.insert(name.to_string(), function);
    }

    pub fn get(&self, name: &str) -> Option<&(dyn FunctionHandler + Send + Sync)> {
        self.functions.get(name).map(|v| &**v)
    }
}

// Function to retrieve and call a function from the registry
pub fn call_function(name: &str, args: Vec<ExactNum>) -> Result<ExactNum, String> {
    if let Some(function) = FUNCTION_REGISTRY.get(name) {
        function.call(args)
    } else {
        Err(format!("Unknown function: {}", name))
    }
}

fn arg_f64(args: &[ExactNum], i: usize) -> f64 {
    args[i].to_f64()
}

lazy_static! {
    pub static ref FUNCTION_REGISTRY: FunctionRegistry = {
        let mut registry = FunctionRegistry::new(); // Make sure registry is mutable

        // Register built-in LaTeX Math Commands

        // Absolute value and rounding
        registry.register_function("abs", Box::new(AbsFunction));
        registry.register_function("floor", Box::new(FloorFunction));
        registry.register_function("ceil", Box::new(CeilFunction));
        registry.register_function("round", Box::new(RoundFunction));
        registry.register_function("trunc", Box::new(TruncFunction));

        // Integer arithmetic
        registry.register_function("gcd", Box::new(GcdFunction));
        registry.register_function("lcm", Box::new(LcmFunction));
        registry.register_function("factorial", Box::new(FactorialFunction));
        registry.register_function("binom", Box::new(BinomFunction));

        // Circular trigonometric
        registry.register_function("sin", Box::new(SinFunction));
        registry.register_function("cos", Box::new(CosFunction));
        registry.register_function("tan", Box::new(TanFunction));

        // Reciprocal trigonometric
        registry.register_function("csc", Box::new(CscFunction));
        registry.register_function("sec", Box::new(SecFunction));
        registry.register_function("cot", Box::new(CotFunction));

        // Inverse circular trigonometric
        registry.register_function("arcsin", Box::new(ArcsinFunction));
        registry.register_function("arccos", Box::new(ArccosFunction));
        registry.register_function("arctan", Box::new(ArctanFunction));
        registry.register_function("asin", Box::new(ArcsinFunction));
        registry.register_function("acos", Box::new(ArccosFunction));
        registry.register_function("atan", Box::new(ArctanFunction));

        // Inverse reciprocal trigonometric
        registry.register_function("arccsc", Box::new(ArccscFunction));
        registry.register_function("arcsec", Box::new(ArcsecFunction));
        registry.register_function("arccot", Box::new(ArccotFunction));

        // Hyperbolic
        registry.register_function("sinh", Box::new(SinhFunction));
        registry.register_function("cosh", Box::new(CoshFunction));
        registry.register_function("tanh", Box::new(TanhFunction));

        // Reciprocal hyperbolic
        registry.register_function("csch", Box::new(CschFunction));
        registry.register_function("sech", Box::new(SechFunction));
        registry.register_function("coth", Box::new(CothFunction));

        // Inverse hyperbolic
        registry.register_function("arcsinh", Box::new(ArcsinhFunction));
        registry.register_function("arccosh", Box::new(ArccoshFunction));
        registry.register_function("arctanh", Box::new(ArctanhFunction));
        registry.register_function("asinh", Box::new(ArcsinhFunction));
        registry.register_function("acosh", Box::new(ArccoshFunction));
        registry.register_function("atanh", Box::new(ArctanhFunction));

        // Inverse reciprocal hyperbolic
        registry.register_function("arccsch", Box::new(ArccschFunction));
        registry.register_function("arcsech", Box::new(ArcsechFunction));
        registry.register_function("arccoth", Box::new(ArccothFunction));

        // Logarithmic and exponential
        registry.register_function("log", Box::new(LogFunction));
        registry.register_function("ln", Box::new(LnFunction));
        registry.register_function("lg", Box::new(LgFunction));
        registry.register_function("exp", Box::new(ExpFunction));

        // Special functions (non-elementary antiderivatives): erf, Ei, li.
        // Symbolic-only for now: they parse, print, and differentiate exactly;
        // numeric evaluation is not yet implemented and says so rather than
        // approximating silently.
        registry.register_function("erf", Box::new(ErfFunction));
        registry.register_function("Ei", Box::new(EiFunction));
        registry.register_function("li", Box::new(LiFunction));

        registry.register_function("frac", Box::new(FracFunction));
        registry.register_function("sqrt", Box::new(SqrtFunction));
        registry.register_function("min", Box::new(MinFunction));
        registry.register_function("max", Box::new(MaxFunction));
        registry.register_function("det", Box::new(DetFunction));
        registry.register_function("dim", Box::new(DimFunction)); // TODO: Implement
        registry.register_function("inf", Box::new(InfFunction));
        registry.register_function("ker", Box::new(KerFunction)); // TODO: Implement
        registry.register_function("sup", Box::new(SupFunction));
        registry.register_function("deg", Box::new(DegFunction)); // TODO: Implement
        registry.register_function("liminf", Box::new(LimInfFunction)); // TODO: Implement Fully
        registry.register_function("limsup", Box::new(LimSupFunction)); // TODO: Implement Fully
        registry.register_function("arg", Box::new(ArgFunction)); // TODO: Implement Fully
        registry.register_function("lim", Box::new(LimFunction)); // TODO: Implement Fully

        registry
    };
}

// Absolute value and rounding
pub struct AbsFunction;
impl FunctionHandler for AbsFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\abs requires exactly one argument.".to_string());
        }
        Ok(args[0].abs())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct FloorFunction;
impl FunctionHandler for FloorFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\floor requires exactly one argument.".to_string());
        }
        Ok(args[0].floor())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct CeilFunction;
impl FunctionHandler for CeilFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\ceil requires exactly one argument.".to_string());
        }
        Ok(args[0].ceil())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct RoundFunction;
impl FunctionHandler for RoundFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\round requires exactly one argument.".to_string());
        }
        Ok(args[0].round())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct TruncFunction;
impl FunctionHandler for TruncFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\trunc requires exactly one argument.".to_string());
        }
        Ok(args[0].trunc())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Integer arithmetic
pub struct GcdFunction;
impl FunctionHandler for GcdFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() < 2 {
            return Err("\\gcd requires at least two arguments.".to_string());
        }
        let mut result = args[0].clone();
        for arg in &args[1..] {
            result = gcd(&result, arg)
                .ok_or_else(|| "\\gcd requires non-negative integer arguments.".to_string())?;
        }
        Ok(result)
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

pub struct LcmFunction;
impl FunctionHandler for LcmFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() < 2 {
            return Err("\\lcm requires at least two arguments.".to_string());
        }
        let mut result = args[0].clone();
        for arg in &args[1..] {
            result = lcm(&result, arg)
                .ok_or_else(|| "\\lcm requires non-negative integer arguments.".to_string())?;
        }
        Ok(result)
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

pub struct FactorialFunction;
impl FunctionHandler for FactorialFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\factorial requires exactly one argument.".to_string());
        }
        factorial(&args[0])
            .ok_or_else(|| "\\factorial requires a non-negative integer.".to_string())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct BinomFunction;
impl FunctionHandler for BinomFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 2 {
            return Err("\\binom requires exactly two arguments.".to_string());
        }
        binom(&args[0], &args[1])
            .ok_or_else(|| "\\binom requires non-negative integer arguments.".to_string())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(2)
    }
}

// Circular trigonometric
pub struct SinFunction;
impl FunctionHandler for SinFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("Sin function requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(args[0].to_f64().sin()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct CosFunction;
impl FunctionHandler for CosFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("Cos function requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(args[0].to_f64().cos()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct TanFunction;
impl FunctionHandler for TanFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\tan requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(args[0].to_f64().tan()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Reciprocal trigonometric
pub struct CscFunction;
impl FunctionHandler for CscFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\csc requires exactly one argument.".to_string());
        }
        let x = arg_f64(&args, 0);
        if x.sin() == 0.0 {
            return Ok(ExactNum::Float(f64::NAN));
        }
        Ok(ExactNum::Float(1.0 / x.sin()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct SecFunction;
impl FunctionHandler for SecFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\sec requires exactly one argument.".to_string());
        }
        let cos_val = arg_f64(&args, 0).cos();
        if cos_val.abs() < 1e-15 {
            return Ok(ExactNum::Float(f64::NAN));
        }
        Ok(ExactNum::Float(1.0 / cos_val))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct CotFunction;
impl FunctionHandler for CotFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\cot requires exactly one argument.".to_string());
        }
        let tan_value = arg_f64(&args, 0).tan();
        if tan_value.abs() < 1e-10 {
            return Ok(ExactNum::Float(f64::NAN));
        }
        Ok(ExactNum::Float(1.0 / tan_value))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Inverse circular trigonometric
pub struct ArcsinFunction;
impl FunctionHandler for ArcsinFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\arcsin requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(args[0].to_f64().asin()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct ArccosFunction;
impl FunctionHandler for ArccosFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\arccos requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(args[0].to_f64().acos()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct ArctanFunction;
impl FunctionHandler for ArctanFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\arctan requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(args[0].to_f64().atan()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Inverse reciprocal trigonometric
pub struct ArccscFunction;
impl FunctionHandler for ArccscFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\arccsc requires exactly one argument.".to_string());
        }
        let x = arg_f64(&args, 0);
        if x.abs() < 1.0 {
            return Ok(ExactNum::Float(f64::NAN));
        }
        Ok(ExactNum::Float((1.0 / x).asin()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct ArcsecFunction;
impl FunctionHandler for ArcsecFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\arcsec requires exactly one argument.".to_string());
        }
        let x = arg_f64(&args, 0);
        if x.abs() < 1.0 {
            return Ok(ExactNum::Float(f64::NAN));
        }
        Ok(ExactNum::Float((1.0 / x).acos()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct ArccotFunction;
impl FunctionHandler for ArccotFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\arccot requires exactly one argument.".to_string());
        }
        let x = arg_f64(&args, 0);
        if x == 0.0 {
            return Ok(ExactNum::Float(std::f64::consts::FRAC_PI_2));
        }
        Ok(ExactNum::Float((1.0 / x).atan()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Hyperbolic
pub struct SinhFunction;
impl FunctionHandler for SinhFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\sinh requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(args[0].to_f64().sinh()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct CoshFunction;
impl FunctionHandler for CoshFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\cosh requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(args[0].to_f64().cosh()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct TanhFunction;
impl FunctionHandler for TanhFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\tanh requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(args[0].to_f64().tanh()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Reciprocal hyperbolic
pub struct CschFunction;
impl FunctionHandler for CschFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\csch requires exactly one argument.".to_string());
        }
        let x = arg_f64(&args, 0);
        if x.sinh() == 0.0 {
            return Ok(ExactNum::Float(f64::NAN));
        }
        Ok(ExactNum::Float(1.0 / x.sinh()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct SechFunction;
impl FunctionHandler for SechFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\sech requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(1.0 / arg_f64(&args, 0).cosh()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct CothFunction;
impl FunctionHandler for CothFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\coth requires exactly one argument.".to_string());
        }
        let tanh_val = arg_f64(&args, 0).tanh();
        if tanh_val == 0.0 {
            return Ok(ExactNum::Float(f64::NAN));
        }
        Ok(ExactNum::Float(1.0 / tanh_val))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Inverse hyperbolic
pub struct ArcsinhFunction;
impl FunctionHandler for ArcsinhFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\arcsinh requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(args[0].to_f64().asinh()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct ArccoshFunction;
impl FunctionHandler for ArccoshFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\arccosh requires exactly one argument.".to_string());
        }
        let x = arg_f64(&args, 0);
        if x < 1.0 {
            return Ok(ExactNum::Float(f64::NAN));
        }
        Ok(ExactNum::Float(x.acosh()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct ArctanhFunction;
impl FunctionHandler for ArctanhFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\arctanh requires exactly one argument.".to_string());
        }
        let x = arg_f64(&args, 0);
        if x.abs() >= 1.0 {
            return Ok(ExactNum::Float(f64::NAN));
        }
        Ok(ExactNum::Float(x.atanh()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Inverse reciprocal hyperbolic
pub struct ArccschFunction;
impl FunctionHandler for ArccschFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\arccsch requires exactly one argument.".to_string());
        }
        let x = arg_f64(&args, 0);
        if x == 0.0 {
            return Ok(ExactNum::Float(f64::NAN));
        }
        Ok(ExactNum::Float((1.0 / x).asinh()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct ArcsechFunction;
impl FunctionHandler for ArcsechFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\arcsech requires exactly one argument.".to_string());
        }
        let x = arg_f64(&args, 0);
        if x <= 0.0 || x > 1.0 {
            return Ok(ExactNum::Float(f64::NAN));
        }
        Ok(ExactNum::Float((1.0 / x).acosh()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct ArccothFunction;
impl FunctionHandler for ArccothFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\arccoth requires exactly one argument.".to_string());
        }
        let x = arg_f64(&args, 0);
        if x.abs() <= 1.0 {
            return Ok(ExactNum::Float(f64::NAN));
        }
        Ok(ExactNum::Float((1.0 / x).atanh()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Binary functions (like \frac)
pub struct FracFunction;
impl FunctionHandler for FracFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 2 {
            return Err("\\frac requires exactly two arguments.".to_string());
        }
        if args[1].is_zero() {
            return Ok(ExactNum::Float(f64::NAN));
        }
        Ok(args[0].clone() / args[1].clone())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(2) // \frac requires exactly two arguments
    }
}

// Logarithmic and exponential
pub struct LogFunction;
impl FunctionHandler for LogFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\log requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(args[0].to_f64().log10()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct LnFunction;
impl FunctionHandler for LnFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\ln requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(args[0].to_f64().ln()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct LgFunction;
impl FunctionHandler for LgFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\lg requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(args[0].to_f64().log2()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

/// The error function erf(x) = (2/√π)∫₀ˣ e^{-t²} dt (DLMF 7.2.1).
/// Kept symbolic: no f64 approximation is offered until one with a stated
/// error bound lands; an honest refusal beats a silent approximation.
pub struct ErfFunction;
impl FunctionHandler for ErfFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\erf requires exactly one argument.".to_string());
        }
        Err("Numeric evaluation of erf is not implemented; the value is kept symbolic.".to_string())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

/// The exponential integral Ei(x) = −∫_{−x}^∞ e^{−t}/t dt (DLMF 6.2.5).
pub struct EiFunction;
impl FunctionHandler for EiFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\Ei requires exactly one argument.".to_string());
        }
        Err("Numeric evaluation of Ei is not implemented; the value is kept symbolic.".to_string())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

/// The logarithmic integral li(x) = ∫₀ˣ dt/ln(t) (DLMF 6.2.8).
pub struct LiFunction;
impl FunctionHandler for LiFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\li requires exactly one argument.".to_string());
        }
        Err("Numeric evaluation of li is not implemented; the value is kept symbolic.".to_string())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct ExpFunction;
impl FunctionHandler for ExpFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\exp requires exactly one argument.".to_string());
        }
        Ok(ExactNum::Float(args[0].to_f64().exp())) //exp(x) = e^x
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Square root
pub struct SqrtFunction;
impl FunctionHandler for SqrtFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\sqrt requires exactly one argument.".to_string());
        }
        Ok(args[0].sqrt())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Min and Max
pub struct MinFunction;
impl FunctionHandler for MinFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.is_empty() {
            return Err("\\min requires at least one argument.".to_string());
        }
        Ok(args
            .into_iter()
            .fold(ExactNum::Float(f64::INFINITY), |a, b| {
                if a.partial_cmp(&b) == Some(std::cmp::Ordering::Greater) {
                    b
                } else {
                    a
                }
            }))
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

pub struct MaxFunction;
impl FunctionHandler for MaxFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.is_empty() {
            return Err("\\max requires at least one argument.".to_string());
        }
        Ok(args
            .into_iter()
            .fold(ExactNum::Float(f64::NEG_INFINITY), |a, b| {
                if a.partial_cmp(&b) == Some(std::cmp::Ordering::Less) {
                    b
                } else {
                    a
                }
            }))
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

// Determinant (currently treated as product)
pub struct DetFunction;
impl FunctionHandler for DetFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.is_empty() {
            return Err("\\det requires at least one argument.".to_string());
        }
        Ok(args.into_iter().fold(ExactNum::one(), |a, b| a * b))
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

// TODO: Implement
pub struct DimFunction;
impl FunctionHandler for DimFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if !args.is_empty() {
            return Err("\\dim does not require any arguments.".to_string());
        }

        // Return a default value for now. You can customize this later.
        Ok(ExactNum::integer(1)) // Assuming dim() returns 1 for simplicity
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(0)
    }
}

pub struct InfFunction;
impl FunctionHandler for InfFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.is_empty() {
            return Err("\\inf requires at least one argument.".to_string());
        }
        Ok(args
            .into_iter()
            .fold(ExactNum::Float(f64::INFINITY), |a, b| {
                if a.partial_cmp(&b) == Some(std::cmp::Ordering::Greater) {
                    b
                } else {
                    a
                }
            })) // Find the minimum
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

// TODO: Implement
pub struct KerFunction;
impl FunctionHandler for KerFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if !args.is_empty() {
            return Err("\\ker does not require any arguments.".to_string());
        }

        // Return a default value for now. You can customize this later.
        Ok(ExactNum::integer(0)) // Assuming ker() returns 0 for simplicity
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(0)
    }
}

pub struct SupFunction;
impl FunctionHandler for SupFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.is_empty() {
            return Err("\\sup requires at least one argument.".to_string());
        }
        Ok(args
            .into_iter()
            .fold(ExactNum::Float(f64::NEG_INFINITY), |a, b| {
                if a.partial_cmp(&b) == Some(std::cmp::Ordering::Less) {
                    b
                } else {
                    a
                }
            })) // Find the maximum
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

pub struct DegFunction;
impl FunctionHandler for DegFunction {
    fn call(&self, _args: Vec<ExactNum>) -> Result<ExactNum, String> {
        // Placeholder return, assuming deg() returns a fixed value
        Ok(ExactNum::integer(1))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(0)
    }
}

pub struct LimInfFunction;
impl FunctionHandler for LimInfFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.is_empty() {
            return Err("\\liminf requires at least one argument.".to_string());
        }
        Ok(args
            .into_iter()
            .fold(ExactNum::Float(f64::INFINITY), |a, b| {
                if a.partial_cmp(&b) == Some(std::cmp::Ordering::Greater) {
                    b
                } else {
                    a
                }
            })) // Minimum value approximation
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

pub struct ArgFunction;
impl FunctionHandler for ArgFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 1 {
            return Err("\\arg requires exactly one argument.".to_string());
        }

        Ok(ExactNum::Float(arg_f64(&args, 0).atan()))
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct LimSupFunction;
impl FunctionHandler for LimSupFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.is_empty() {
            return Err("\\limsup requires at least one argument.".to_string());
        }
        Ok(args
            .into_iter()
            .fold(ExactNum::Float(f64::NEG_INFINITY), |a, b| {
                if a.partial_cmp(&b) == Some(std::cmp::Ordering::Less) {
                    b
                } else {
                    a
                }
            })) // Maximum value approximation
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

pub struct LimFunction;
impl FunctionHandler for LimFunction {
    fn call(&self, args: Vec<ExactNum>) -> Result<ExactNum, String> {
        if args.len() != 2 {
            return Err("\\lim requires exactly two arguments: the function value and the point to evaluate at.".to_string());
        }

        Ok(args[0].clone()) // Just return the function value for now (as a placeholder)
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(2) // Requires two arguments: function and the point
    }
}

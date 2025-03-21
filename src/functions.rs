use lazy_static::lazy_static;
use std::collections::HashMap;

// Define a trait for function handlers
pub trait FunctionHandler {
    fn call(&self, args: Vec<f64>) -> Result<f64, String>;

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
pub fn call_function(name: &str, args: Vec<f64>) -> Result<f64, String> {
    if let Some(function) = FUNCTION_REGISTRY.get(name) {
        function.call(args)
    } else {
        Err(format!("Unknown function: {}", name))
    }
}

lazy_static! {
    pub static ref FUNCTION_REGISTRY: FunctionRegistry = {
        let mut registry = FunctionRegistry::new(); // Make sure registry is mutable

        // Register built-in LaTeX Math Commands
        registry.register_function("sin", Box::new(SinFunction));
        registry.register_function("cos", Box::new(CosFunction));
        registry.register_function("tan", Box::new(TanFunction));
        registry.register_function("sinh", Box::new(SinhFunction));
        registry.register_function("cosh", Box::new(CoshFunction));
        registry.register_function("tanh", Box::new(TanhFunction));
        registry.register_function("arcsin", Box::new(ArcsinFunction));
        registry.register_function("arccos", Box::new(ArccosFunction));
        registry.register_function("arctan", Box::new(ArctanFunction));
        registry.register_function("sec", Box::new(SecFunction));
        registry.register_function("csc", Box::new(CscFunction));
        registry.register_function("coth", Box::new(CothFunction));
        registry.register_function("frac", Box::new(FracFunction));
        registry.register_function("log", Box::new(LogFunction));
        registry.register_function("ln", Box::new(LnFunction));
        registry.register_function("lg", Box::new(LgFunction));
        registry.register_function("sqrt", Box::new(SqrtFunction));
        registry.register_function("min", Box::new(MinFunction));
        registry.register_function("max", Box::new(MaxFunction));
        registry.register_function("det", Box::new(DetFunction));
        registry.register_function("cot", Box::new(CotFunction));
        registry.register_function("dim", Box::new(DimFunction)); // TODO: Implement
        registry.register_function("inf", Box::new(InfFunction));
        registry.register_function("exp", Box::new(ExpFunction));
        registry.register_function("ker", Box::new(KerFunction)); // TODO: Implement
        registry.register_function("sup", Box::new(SupFunction));
        registry.register_function("deg", Box::new(DegFunction)); // TODO: Implement
        registry.register_function("liminf", Box::new(LimInfFunction)); // TODO: Implement Fully
        registry.register_function("limsup", Box::new(LimSupFunction)); // TODO: Implement Fully
        registry.register_function("arg", Box::new(ArgFunction)); // TODO: Implement Fully
        registry.register_function("gcd", Box::new(GcdFunction));
        registry.register_function("lim", Box::new(LimFunction)); // TODO: Implement Fully

        registry
    };
}

// Example implementation for a sine function
pub struct SinFunction;
impl FunctionHandler for SinFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("Sin function requires exactly one argument.".to_string());
        }
        Ok(args[0].sin())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Example implementation for a cosine function
pub struct CosFunction;
impl FunctionHandler for CosFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("Cos function requires exactly one argument.".to_string());
        }
        Ok(args[0].cos())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct TanFunction;
impl FunctionHandler for TanFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\tan requires exactly one argument.".to_string());
        }
        Ok(args[0].tan())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Hyperbolic functions
pub struct SinhFunction;
impl FunctionHandler for SinhFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\sinh requires exactly one argument.".to_string());
        }
        Ok(args[0].sinh())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct CoshFunction;
impl FunctionHandler for CoshFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\cosh requires exactly one argument.".to_string());
        }
        Ok(args[0].cosh())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct TanhFunction;
impl FunctionHandler for TanhFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\tanh requires exactly one argument.".to_string());
        }
        Ok(args[0].tanh())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Inverse trigonometric functions
pub struct ArcsinFunction;
impl FunctionHandler for ArcsinFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\arcsin requires exactly one argument.".to_string());
        }
        Ok(args[0].asin())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct ArccosFunction;
impl FunctionHandler for ArccosFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\arccos requires exactly one argument.".to_string());
        }
        Ok(args[0].acos())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct ArctanFunction;
impl FunctionHandler for ArctanFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\arctan requires exactly one argument.".to_string());
        }
        Ok(args[0].atan())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Secant, cosecant, and cotangent functions
pub struct SecFunction;
impl FunctionHandler for SecFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\sec requires exactly one argument.".to_string());
        }
        if args[0].cos() == 0.0 {
            return Ok(f64::NAN); // Return NaN for undefined result
        }
        Ok(1.0 / args[0].cos())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct CscFunction;
impl FunctionHandler for CscFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\csc requires exactly one argument.".to_string());
        }
        if args[0].sin() == 0.0 {
            return Ok(f64::NAN); // Return NaN for undefined result
        }
        Ok(1.0 / args[0].sin())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct CothFunction;
impl FunctionHandler for CothFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\coth requires exactly one argument.".to_string());
        }
        let tanh_val = args[0].tanh();
        if tanh_val == 0.0 {
            return Ok(f64::NAN); // Return NaN for undefined result
        }
        Ok(1.0 / tanh_val)
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Binary functions (like \frac)
pub struct FracFunction;
impl FunctionHandler for FracFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 2 {
            return Err("\\frac requires exactly two arguments.".to_string());
        }
        if args[1] == 0.0 {
            return Ok(f64::NAN); // Return NaN for division by zero
        }
        Ok(args[0] / args[1])
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(2) // \frac requires exactly two arguments
    }
}

// Logarithmic functions
pub struct LogFunction;
impl FunctionHandler for LogFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\log requires exactly one argument.".to_string());
        }
        Ok(args[0].log10())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct LnFunction;
impl FunctionHandler for LnFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\ln requires exactly one argument.".to_string());
        }
        Ok(args[0].ln())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct LgFunction;
impl FunctionHandler for LgFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\lg requires exactly one argument.".to_string());
        }
        Ok(args[0].log2())
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// Square root
pub struct SqrtFunction;
impl FunctionHandler for SqrtFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
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
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.is_empty() {
            return Err("\\min requires at least one argument.".to_string());
        }
        Ok(args.into_iter().fold(f64::INFINITY, |a, b| a.min(b)))
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

pub struct MaxFunction;
impl FunctionHandler for MaxFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.is_empty() {
            return Err("\\max requires at least one argument.".to_string());
        }
        Ok(args.into_iter().fold(f64::NEG_INFINITY, |a, b| a.max(b)))
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

// Determinant (currently treated as product)
pub struct DetFunction;
impl FunctionHandler for DetFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.is_empty() {
            return Err("\\det requires at least one argument.".to_string());
        }
        Ok(args.into_iter().product())
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

pub struct CotFunction;
impl FunctionHandler for CotFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\cot requires exactly one argument.".to_string());
        }
        let tan_value = args[0].tan();

        if tan_value.abs() < 1e-10 {
            // If tan(x) is close to zero, cot(x) is undefined (infinity)
            return Ok(f64::NAN);
        }

        Ok(1.0 / tan_value) // cot(x) = 1 / tan(x)
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// TODO: Implement
pub struct DimFunction;
impl FunctionHandler for DimFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if !args.is_empty() {
            return Err("\\dim does not require any arguments.".to_string());
        }

        // Return a default value for now. You can customize this later.
        Ok(1.0) // Assuming dim() returns 1 for simplicity
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(0)
    }
}

pub struct InfFunction;
impl FunctionHandler for InfFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.is_empty() {
            return Err("\\inf requires at least one argument.".to_string());
        }
        Ok(args.into_iter().fold(f64::INFINITY, |a, b| a.min(b))) // Find the minimum
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

pub struct ExpFunction;
impl FunctionHandler for ExpFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\exp requires exactly one argument.".to_string());
        }
        Ok(args[0].exp()) // exp(x) = e^x
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

// TODO: Implement
pub struct KerFunction;
impl FunctionHandler for KerFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if !args.is_empty() {
            return Err("\\ker does not require any arguments.".to_string());
        }

        // Return a default value for now. You can customize this later.
        Ok(0.0) // Assuming ker() returns 0 for simplicity
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(0)
    }
}

pub struct SupFunction;
impl FunctionHandler for SupFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.is_empty() {
            return Err("\\sup requires at least one argument.".to_string());
        }
        Ok(args.into_iter().fold(f64::NEG_INFINITY, |a, b| a.max(b))) // Find the maximum
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

pub struct DegFunction;
impl FunctionHandler for DegFunction {
    fn call(&self, _args: Vec<f64>) -> Result<f64, String> {
        // Placeholder return, assuming deg() returns a fixed value
        Ok(1.0) // Assuming deg returns 1 for now
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(0)
    }
}

pub struct LimInfFunction;
impl FunctionHandler for LimInfFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.is_empty() {
            return Err("\\liminf requires at least one argument.".to_string());
        }
        Ok(args.into_iter().fold(f64::INFINITY, |a, b| a.min(b))) // Minimum value approximation
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

pub struct ArgFunction;
impl FunctionHandler for ArgFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 1 {
            return Err("\\arg requires exactly one argument.".to_string());
        }

        Ok(args[0].atan()) // For real numbers, we'll return atan(x)
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(1)
    }
}

pub struct LimSupFunction;
impl FunctionHandler for LimSupFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.is_empty() {
            return Err("\\limsup requires at least one argument.".to_string());
        }
        Ok(args.into_iter().fold(f64::NEG_INFINITY, |a, b| a.max(b))) // Maximum value approximation
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

pub struct GcdFunction;
impl FunctionHandler for GcdFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() < 2 {
            return Err("\\gcd requires at least two arguments.".to_string());
        }

        fn gcd(a: u64, b: u64) -> u64 {
            let mut a = a;
            let mut b = b;
            while b != 0 {
                let temp = b;
                b = a % b;
                a = temp;
            }
            a
        }

        // Convert arguments to integers
        let args: Vec<u64> = args
            .into_iter()
            .map(|x| x as u64) // Truncate floating point numbers to integers
            .collect();

        // Compute GCD of all arguments
        let mut result = args[0];
        for &num in &args[1..] {
            result = gcd(result, num);
        }

        Ok(result as f64) // Return the GCD
    }

    fn get_arg_count(&self) -> Option<usize> {
        None // Variable number of arguments
    }
}

pub struct LimFunction;
impl FunctionHandler for LimFunction {
    fn call(&self, args: Vec<f64>) -> Result<f64, String> {
        if args.len() != 2 {
            return Err("\\lim requires exactly two arguments: the function value and the point to evaluate at.".to_string());
        }

        Ok(args[0]) // Just return the function value for now (as a placeholder)
    }

    fn get_arg_count(&self) -> Option<usize> {
        Some(2) // Requires two arguments: function and the point
    }
}

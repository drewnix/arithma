use lazy_static::lazy_static;
use std::collections::HashMap;

// Define a trait for function handlers
pub trait FunctionHandler {
    fn call(&self, args: Vec<f64>) -> Result<f64, String>;

    // New method to return the number of arguments the function requires
    fn get_arg_count(&self) -> usize;
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

    fn get_arg_count(&self) -> usize {
        1 // \sin expects 1 argument
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

    fn get_arg_count(&self) -> usize {
        1 // \cos expects 1 argument
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

    fn get_arg_count(&self) -> usize {
        1
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

    fn get_arg_count(&self) -> usize {
        1
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

    fn get_arg_count(&self) -> usize {
        1
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

    fn get_arg_count(&self) -> usize {
        1
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

    fn get_arg_count(&self) -> usize {
        1
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

    fn get_arg_count(&self) -> usize {
        1
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

    fn get_arg_count(&self) -> usize {
        1
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

    fn get_arg_count(&self) -> usize {
        1
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

    fn get_arg_count(&self) -> usize {
        1
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

    fn get_arg_count(&self) -> usize {
        1
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
            return Ok(f64::NAN); // Return NaN instead of panicking
        }
        Ok(args[0] / args[1])
    }

    fn get_arg_count(&self) -> usize {
        2 // \frac expects 2 arguments
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

    fn get_arg_count(&self) -> usize {
        1
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

    fn get_arg_count(&self) -> usize {
        1
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

    fn get_arg_count(&self) -> usize {
        1
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

    fn get_arg_count(&self) -> usize {
        1
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

    fn get_arg_count(&self) -> usize {
        0 // Variable number of arguments
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

    fn get_arg_count(&self) -> usize {
        0 // Variable number of arguments
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

    fn get_arg_count(&self) -> usize {
        0 // Variable number of arguments
    }
}

// Define the function registry that holds all functions
pub struct FunctionRegistry {
    functions: HashMap<String, Box<dyn FunctionHandler + Send + Sync>>,
}

impl FunctionRegistry {
    // Create a new function registry (using lazy_static to ensure it's a singleton)
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    // Store the function in the registry
    pub fn register_function(
        &mut self,
        name: &str,
        function: Box<dyn FunctionHandler + Send + Sync>,
    ) {
        self.functions.insert(name.to_string(), function);
    } // Retrieve a function from the registry
    pub fn get(&self, name: &str) -> Option<&Box<dyn FunctionHandler + Send + Sync>> {
        self.functions.get(name)
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

        // Register built-in functions
        registry.register_function("\\sin", Box::new(SinFunction));
        registry.register_function("\\cos", Box::new(CosFunction));
        registry.register_function("\\tan", Box::new(TanFunction));
        registry.register_function("\\sinh", Box::new(SinhFunction));
        registry.register_function("\\cosh", Box::new(CoshFunction));
        registry.register_function("\\tanh", Box::new(TanhFunction));
        registry.register_function("\\arcsin", Box::new(ArcsinFunction));
        registry.register_function("\\arccos", Box::new(ArccosFunction));
        registry.register_function("\\arctan", Box::new(ArctanFunction));
        registry.register_function("\\sec", Box::new(SecFunction));
        registry.register_function("\\csc", Box::new(CscFunction));
        registry.register_function("\\coth", Box::new(CothFunction));
        registry.register_function("\\frac", Box::new(FracFunction));
        registry.register_function("\\log", Box::new(LogFunction));
        registry.register_function("\\ln", Box::new(LnFunction));
        registry.register_function("\\lg", Box::new(LgFunction));
        registry.register_function("\\sqrt", Box::new(SqrtFunction));
        registry.register_function("\\min", Box::new(MinFunction));
        registry.register_function("\\max", Box::new(MaxFunction));
        registry.register_function("\\det", Box::new(DetFunction));

        registry
    };
}

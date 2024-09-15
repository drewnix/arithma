import { useState, useEffect } from 'react';
import './App.css';
import init, { solve_for_variable_js } from 'cassy';

function App() {
  const [equation, setEquation] = useState(""); // Equation input by the user
  const [solution, setSolution] = useState(""); // Solution/result from WASM
  const [error, setError] = useState(""); // Error handling

  // Initialize WASM once when the app starts
  useEffect(() => {
    const initializeWasm = async () => {
      try {
        // Ensure WASM module is initialized with the correct object
        await init({ path: '/pkg/cassy_bg.wasm' }); // Pass the correct path to the WASM file
      } catch (err) {
        console.error("WASM initialization failed:", err);
      }
    };

    initializeWasm();
  }, []); // Empty dependency array to ensure it runs once

  // Helper function to parse the equation
  const parseEquation = (equation: string) => {
    // For now, this is very basic parsing, just to get "x + 2 = 10" working
    const parts = equation.split("=");
    if (parts.length !== 2) {
      throw new Error("Equation must be in the format 'x + 2 = 10'");
    }

    const left = parts[0].trim();  // This would be "x + 2"
    const right = parts[1].trim(); // This would be "10"
    
    const rightVal = parseFloat(right); // Convert right side to number

    // For simplicity, assume the left is always in the form "x + Number"
    const [variable, operator, number] = left.split(" ");

    if (operator !== "+" && operator !== "-") {
      throw new Error("Only simple addition/subtraction is supported for now");
    }

    const exprJson = JSON.stringify({
      Add: [
        { Variable: variable },
        { Number: parseFloat(number) }
      ]
    });

    return { exprJson, rightVal, variable };
  };

  // Function to handle solving the equation
  const handleSolveEquation = async () => {
    try {
      // Parse the equation entered by the user
      const { exprJson, rightVal, variable } = parseEquation(equation);

      // Solve the equation using the solve_for_variable_js function from WASM
      const result = await solve_for_variable_js(exprJson, rightVal, variable);

      // Update the solution in the state
      setSolution(`The solution for ${variable} is: ${result}`);
      setError(""); // Clear any previous error
    } catch (err: any) {
      // If an error occurs, set the error message
      setError(`Error: ${err.message || err}`);
      setSolution(""); // Clear previous solution
    }
  };

  return (
    <div className="App">
      <h1>Cassy</h1>

      {/* Input field for entering equation */}
      <div className="card">
        <input
          type="text"
          placeholder="Enter equation (e.g., x + 2 = 10)"
          value={equation}
          onChange={(e) => setEquation(e.target.value)}
        />
        <button onClick={handleSolveEquation}>Solve Equation</button>
      </div>

      {/* Display solution or error */}
      {solution && <p>{solution}</p>}
      {error && <p className="error">{error}</p>}
    </div>
  );
}

export default App;
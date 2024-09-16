import { useState, useEffect } from 'react';
import './App.css';
import init, { evaluate_expression_js } from 'cassy';

function App() {
  const [input, setInput] = useState(""); // User's input
  const [solution, setSolution] = useState(""); // Solution/result from WASM
  const [error, setError] = useState(""); // Error handling
  const [environment, setEnvironment] = useState({ vars: {} }); // Environment state

  // Initialize WASM once when the app starts
  useEffect(() => {
    const initializeWasm = async () => {
      try {
        await init({ path: '/pkg/cassy_bg.wasm' });
      } catch (err) {
        console.error("WASM initialization failed:", err);
      }
    };

    initializeWasm();
  }, []);

  // Function to handle evaluating the input (for both equations and simple expressions)
  const handleEvaluate = async () => {
    try {
      // Pass the environment as a JSON string to the WASM function
      const envJson = JSON.stringify(environment);

      // Call WASM to evaluate the input (equation or expression)
      const result = await evaluate_expression_js(input, envJson);

      // Update the environment with the result (if necessary)
      const updatedEnv = { ...environment }; // Add any necessary variable updates here
      setEnvironment(updatedEnv);

      // Display the solution
      setSolution(`Result: ${result}`);
      setError(""); // Clear any previous errors
    } catch (err: any) {
      // If an error occurs, set the error message
      setError(`Error: ${err.message || err}`);
      setSolution(""); // Clear the previous solution
    }
  };

  return (
    <div className="App">
      <h1>Cassy</h1>

      {/* Input field for entering equations or expressions */}
      <div className="card">
        <input
          type="text"
          placeholder="Enter equation or expression (e.g., x + 2 = 10 or x + 2)"
          value={input}
          onChange={(e) => setInput(e.target.value)}
        />
        <button onClick={handleEvaluate}>Evaluate</button>
      </div>

      {/* Display solution or error */}
      {solution && <p>{solution}</p>}
      {error && <p className="error">{error}</p>}
    </div>
  );
}

export default App;
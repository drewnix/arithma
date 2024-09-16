import { useState, useEffect } from 'react';
import './App.css';
import init, { evaluate_expression_js } from 'cassy';
import {
  ChakraProvider,
  VStack,
} from "@chakra-ui/react";
import ControlPad from './components/ControlPad'; // Import the new component
import ExpressionInput from './components/ExpressionInput'; // Import the new component
import HistorySection from './components/HistorySection'; // Import the new component


interface HistoryItem {
  input: string;
  result: string;
}


function App() {
  const [input, setInput] = useState(""); // User's input
  const [error, setError] = useState(""); // Error handling
  const [environment, setEnvironment] = useState({ vars: {} }); // Environment state
  const [history, setHistory] = useState<HistoryItem[]>([]); // Explicit type for history

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
      setHistory([...history, { input, result }]);
      setError(""); // Clear any previous errors
    } catch (err: any) {
      // If an error occurs, set the error message
      setError(`Error: ${err.message || err}`);
    }
  };

  const handleInput = (value) => {
    setInput((prev) => prev + value);
  };

  return (
    <ChakraProvider>
      <VStack spacing={4} align="center" p={4}>

        <div className="App">
          <h1>Cassy</h1>
          <h2>Prototype CAS Platform</h2>
          <br />

          {/* Math Control Pad */}
          <ControlPad onInput={handleInput} /> {/* Using the new component */}

          {/* Math Expression Input */}
          <ExpressionInput
            input={input}
            setInput={setInput}
            handleEvaluate={handleEvaluate}
          />

          {/* History Section */}
          <HistorySection history={history} />

          {/* Display error */}
          {error && <p className="error"><strong>{error}</strong></p>}
        </div>
      </VStack>
    </ChakraProvider>
  );
}

export default App;
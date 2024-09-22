import { useState, useEffect } from 'react';
import './App.css';
import init, { evaluate_latex_expression_js } from 'arithma';
import { ChakraProvider, Heading, Text, VStack } from "@chakra-ui/react";
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
        await init({ path: '/pkg/arithma_bg.wasm' });
      } catch (err) {
        console.error("WASM initialization failed:", err);
      }
    };

    initializeWasm();
  }, []);

  // Function to handle evaluating the input (for both equations and simple expressions)
  const handleEvaluate = async (latex: string) => {
    try {
      // Pass the environment as a JSON string to the WASM function
      const envJson = JSON.stringify(environment);

      // Pass LaTeX to Rust WASM for evaluation
      const result = await evaluate_latex_expression_js(latex, envJson);

      // Update the environment with the result (if necessary)
      const updatedEnv = { ...environment };
      setEnvironment(updatedEnv);

      // Display the solution
      setHistory([...history, { input: latex, result }]);
      setError(""); // Clear any previous errors
    } catch (err: any) {
      setError(`Error: ${err.message || err}`);
    }
  };

  // Function to populate ExpressionInput when history item is clicked
  const handleHistoryItemClick = (latex: string) => {
    setInput(latex); // Set the input to the clicked LaTeX equation
  };

  return (
    <ChakraProvider>
      <VStack spacing={4} align="center" p={4}>
        <div className="App">
          <Heading marginBottom='5px' as='h1' size='4xl'>Arithma</Heading>
          <Text fontSize='xl'>
            Prototype CAS Platform
          </Text>
          <br />

          {/* Math Expression Input */}
          <ExpressionInput
            input={input}
            setInput={setInput}
            handleEvaluate={handleEvaluate}
          />

          {/* History Section */}
          <HistorySection
            history={history}
            onHistoryItemClick={handleHistoryItemClick} // Pass the click handler to HistorySection
          />

          {/* Display error */}
          {error && <p className="error"><strong>{error}</strong></p>}
        </div>
      </VStack>
    </ChakraProvider>
  );
}

export default App;
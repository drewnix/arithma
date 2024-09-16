import { useState, useEffect } from 'react';
import './App.css';
import init, { evaluate_expression_js } from 'cassy';
import {
  ChakraProvider,
  Box,
  Button,
  Input,
  Grid,
  Textarea,
  VStack,
  Heading,
  Stack,
} from "@chakra-ui/react";

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
          <br/>

          {/* Math Control Pad */}
          <Box w="100%" maxW="600px">
            <Grid templateColumns="repeat(6, 1fr)" gap={2} p={4} bg="gray.100" borderRadius="md">
              {["π", "√", "^", "+", "-", "÷", "*", "(", ")", "x", "y", "z", "sin", "cos", "tan", "log", "ln", "="].map((symbol) => (
                <Button key={symbol} onClick={() => handleInput(symbol)}>
                  {symbol}
                </Button>
              ))}
            </Grid>
          </Box>


        {/* Math Expression Input */}
        <Box w="100%" maxW="600px" display="flex" justifyContent="space-between">
          <Input
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Enter an equation (e.g., x + 2 = 10 or x + 2)"
            size="lg"
            flexGrow={1}
          />
          <Button ml={2} colorScheme="blue" onClick={handleEvaluate}>
            Evaluate
          </Button>
        </Box>

        {/* History Section */}
        <Box w="100%" maxW="600px" p={4} bg="gray.50" borderRadius="md">
          <Heading as="h3" size="md" mb={2}>
            History
          </Heading>
          <Stack spacing={2}>
            {history.map((item, index) => (
              <Box key={index} p={2} bg="white" borderRadius="md" boxShadow="md">
                <strong>{item.input}</strong> = {item.result}
              </Box>
            ))}
          </Stack>
        </Box>
          {/* Display error */}
          {error && <p className="error"><strong>{error}</strong></p>}
        </div>
      </VStack>
    </ChakraProvider>
  );
}

export default App;
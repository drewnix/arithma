import React from "react";
import { Box, Input, Button } from "@chakra-ui/react";

interface ExpressionInputProps {
  input: string;
  setInput: (input: string) => void;
  handleEvaluate: () => void;
}

const ExpressionInput: React.FC<ExpressionInputProps> = ({
  input,
  setInput,
  handleEvaluate,
}) => {
  return (
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
  );
};

export default ExpressionInput;
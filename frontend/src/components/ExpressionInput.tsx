import { MathfieldElement } from "mathlive";
import "//unpkg.com/mathlive";

declare global {
  namespace JSX {
    interface IntrinsicElements {
      'math-field': React.DetailedHTMLProps<React.HTMLAttributes<MathfieldElement>, MathfieldElement>;
    }
  }
}
import React from "react";
import { Box, Button } from "@chakra-ui/react";

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
  const handleKeyDown = (evt: React.KeyboardEvent<MathfieldElement>) => {
    const mathfield = evt.target as MathfieldElement; // Cast target to MathfieldElement

    // Check if the Enter key is pressed
    if (evt.key === "Enter") {
      handleEvaluate(); // Call the evaluate function when Enter is pressed
    } else {
      setInput(mathfield.getValue("latex-expanded")); // Update the input when other keys are pressed
    }
  };

  return (
    <Box w="100%" maxW="600px" display="flex" justifyContent="space-between">
      <math-field
        style={{
          flexGrow: 1,
          padding: "10px",
          border: "1px solid #ccc",
          borderRadius: "5px",
          width: "500px",
          paddingBottom: "10px",
          marginBottom: "20px"
        }}
        onInput={(evt: React.FormEvent<MathfieldElement>) => {
          const target = evt.target as MathfieldElement;
          setInput(target.getValue());
        }}
        onKeyDown={handleKeyDown}
      >
        {input}
      </math-field>
      <Button ml={3} style={{height: "63px"}} colorScheme="teal" onClick={handleEvaluate}>
        Evaluate
      </Button>
    </Box>
  );
};

export default ExpressionInput;
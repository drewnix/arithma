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
  handleEvaluate: (latex: string) => void;
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
      const latex = mathfield.getValue("latex-expanded"); // Get LaTeX value

      handleEvaluate(latex);
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
      <Button ml={3} style={{height: "63px"}} colorScheme="teal"
        onClick={() => {
          const mathfield = document.querySelector("math-field") as MathfieldElement;
          const latex = mathfield.getValue("latex-expanded"); // Get LaTeX value
          handleEvaluate(latex);
        }}
        >
        Evaluate
      </Button>
    </Box>
  );
};

export default ExpressionInput;
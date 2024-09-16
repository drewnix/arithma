import React from "react";
import { Box, Button, Grid } from "@chakra-ui/react";

interface MathControlPadProps {
  onInput: (value: string) => void;
}

const ControlPad: React.FC<MathControlPadProps> = ({ onInput }) => {
  const symbols = [
    "π", "√", "^", "+", "-", "÷", "*", "(", ")", "x", "y", "z", "sin", "cos", "tan", "log", "ln", "="
  ];

  return (
    <Box w="100%" maxW="600px">
      <Grid templateColumns="repeat(6, 1fr)" gap={2} p={4} bg="gray.100" borderRadius="md">
        {symbols.map((symbol) => (
          <Button key={symbol} onClick={() => onInput(symbol)}>
            {symbol}
          </Button>
        ))}
      </Grid>
    </Box>
  );
};

export default ControlPad;
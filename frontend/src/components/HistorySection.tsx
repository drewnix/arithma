import React from 'react';
import { Box, Heading, Stack } from '@chakra-ui/react';

interface HistoryItem {
  input: string;
  result: string;
}

interface HistorySectionProps {
  history: HistoryItem[];
}

const HistorySection: React.FC<HistorySectionProps> = ({ history }) => {
  return (
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
  );
};

export default HistorySection;
import React from 'react';
import { Box, Heading, Stack, Flex, Text } from '@chakra-ui/react';
import katex from 'katex';
import 'katex/dist/katex.min.css';

interface HistoryItem {
  input: string;
  result: string;
}

interface HistorySectionProps {
  history: HistoryItem[];
  onHistoryItemClick: (latex: string) => void;
}

const HistorySection: React.FC<HistorySectionProps> = ({ history, onHistoryItemClick }) => {
  return (
    <Box w="100%" maxW="600px" p={4} bg="gray.50" borderRadius="md">
      <Heading as="h3" size="md" mb={2}>
        History
      </Heading>
      <Stack spacing={4}>
        {/* Reverse the history list to show the most recent at the top */}
        {history.slice(0).reverse().map((item, index) => (
          <Flex
            key={index}
            p={2}
            bg="white"
            borderRadius="md"
            boxShadow="md"
            justifyContent="space-between"
            alignItems="center"
            cursor="pointer"
            onClick={() => onHistoryItemClick(item.input)} // Handle click on history item
          >
            {/* Left aligned LaTeX input */}
            <div
              style={{ flexGrow: 1, textAlign: 'left' }}
              dangerouslySetInnerHTML={{
                __html: katex.renderToString(item.input, {
                  throwOnError: false,
                }),
              }}
            ></div>

            {/* Right aligned result */}
            <Text ml={4} fontWeight="bold" color="#319795">
              {item.result}
            </Text>
          </Flex>
        ))}
      </Stack>
    </Box>
  );
};

export default HistorySection;
import type { Meta, StoryObj } from '@storybook/react';
import { ChakraProvider } from '@chakra-ui/react';
import HistorySection from '../components/HistorySection';

const meta = {
  title: 'Components/HistorySection',
  component: HistorySection,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    history: {
      control: 'object',
      description: 'Array of history items (input and result)',
    },
    onHistoryItemClick: { action: 'history item clicked' }, // Add action for the onHistoryItemClick handler
  },
} satisfies Meta<typeof HistorySection>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    history: [
      { input: 'x + 2 = 4', result: 'x = 2' },
      { input: 'y * 3 = 9', result: 'y = 3' },
    ],
    onHistoryItemClick: () => {}, // Provide a dummy function for onHistoryItemClick
  },
  decorators: [
    (Story) => (
      <ChakraProvider>
        <Story />
      </ChakraProvider>
    ),
  ],
};
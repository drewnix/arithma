import type { Meta, StoryObj } from '@storybook/react';
import { ChakraProvider } from '@chakra-ui/react';
import ControlPad from '../components/ControlPad';

// Define meta information for the story, ensuring it satisfies Meta requirements
const meta = {
  title: 'Components/ControlPad',
  component: ControlPad,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    onInput: { action: 'input' }, // Capture onInput action in Storybook's Actions panel
  },
} satisfies Meta<typeof ControlPad>;

export default meta;
type Story = StoryObj<typeof meta>;

// Define the default story
export const Default: Story = {
  args: {
    onInput: (value: string) => console.log(`Button pressed: ${value}`),
  },
  decorators: [
    (Story) => (
      <ChakraProvider>
        <Story />
      </ChakraProvider>
    ),
  ],
};
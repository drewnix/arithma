import type { Meta, StoryObj } from '@storybook/react';
import ExpressionInput from '../components/ExpressionInput';

// Define meta for the story
const meta = {
  title: 'Components/ExpressionInput',
  component: ExpressionInput,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
  argTypes: {
    handleEvaluate: { action: 'evaluate' }, // Capture evaluate action in Storybook's Actions panel
    setInput: { action: 'input change' }, // Capture input changes
  },
} satisfies Meta<typeof ExpressionInput>;

export default meta;
type Story = StoryObj<typeof meta>;

// Define the default story
export const Default: Story = {
  args: {
    input: '',
    handleEvaluate: () => console.log('Evaluated!'),
    setInput: (value: string) => console.log('Input changed:', value),
  },
  decorators: [
    (Story) => (
        <Story />
    ),
  ],
};
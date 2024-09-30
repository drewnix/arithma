import type { Meta, StoryObj } from '@storybook/react';
import MathChart from '../components/MathChart';

// Define meta for the story
const meta = {
  title: 'Components/MathChart',
  component: MathChart,
  parameters: {
    layout: 'centered',
  },
  tags: ['autodocs'],
} satisfies Meta<typeof MathChart>;

export default meta;
type Story = StoryObj<typeof meta>;

// Define the default story
export const Default: Story = {
  args: {
    width: 600,
    height: 400,
    maxX: 20,
    maxY: 20
  },
  decorators: [
    (Story) => (
        <Story />
    ),
  ],
};
import type {Meta, StoryObj} from '@storybook/react';
import App from '../App';


const meta = {
    title: 'Components/App',
    component: App,
    parameters: {
        layout: 'left',
    },
    tags: ['autodocs'],
    argTypes: {}
} satisfies Meta<typeof App>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
    args: {
        defaultCollapsed: true,
        defaultLayout: [20, 32, 48],
        navCollapsedSize: 4
    },
    decorators: [
        (Story) => (
            <Story/>
        ),
    ],
};
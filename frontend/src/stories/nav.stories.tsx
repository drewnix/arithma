import type {Meta, StoryObj} from '@storybook/react';
import {Nav} from '../components/nav';
import {
    Inbox,
} from "lucide-react"
import {
    TooltipProvider
} from "@/components/ui/tooltip.tsx";
import '../index.css'
import {
    ResizableHandle,
    ResizablePanel,
    ResizablePanelGroup,
} from "@/components/ui/resizable"


const meta = {
    title: 'Components/Nav',
    component: Nav,
    parameters: {
        layout: 'centered',
    },
    tags: ['autodocs'],
    argTypes: {}
} satisfies Meta<typeof Nav>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
    args: {
        isCollapsed: true,
        links: [
            {
                title: 'Inbox',
                label: 'inbox',
                icon: Inbox,
                variant: 'default'
            },
            {
                title: 'foo',
                label: 'foobar',
                icon: Inbox,
                variant: 'ghost'
            },
        ]
    },
    decorators: [
        (Story) => (
            <TooltipProvider>
                <ResizablePanelGroup
                    direction="horizontal"
                    onLayout={(sizes: number[]) => {
                        document.cookie = `react-resizable-panels:layout:mail=${JSON.stringify(
                            sizes
                        )}`
                    }}
                    className="h-full max-h-[800px] items-stretch"
                >
                    <ResizablePanel
                        collapsible={true}
                        minSize={15}
                        maxSize={20}
                    >

                        <Story/>
                    </ResizablePanel>
                    <ResizableHandle withHandle/>

                </ResizablePanelGroup>

            </TooltipProvider>
        ),
    ],
};
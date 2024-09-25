"use client"

import * as React from "react"

import './App.css';
import {Nav} from './components/nav';
import {
    TooltipProvider
} from "@/components/ui/tooltip.tsx";
import './index.css'
import {
    ResizablePanel,
    ResizablePanelGroup,
} from "@/components/ui/resizable"
import {Calculator, Infinity} from "lucide-react";
import {twMerge} from "tailwind-merge"
import {clsx, type ClassValue} from "clsx"
import {useEffect, useState} from "react";
import init, {evaluate_latex_expression_js} from "arithma";
import ExpressionInput from "@/components/ExpressionInput.tsx";
import HistorySection from "@/components/HistorySection.tsx";

function cn(...inputs: ClassValue[]) {
    return twMerge(clsx(inputs))
}


interface HistoryItem {
    input: string;
    result: string;
}


interface NewAppProps {
    defaultLayout: number[] | undefined
    defaultCollapsed?: boolean
    navCollapsedSize: number
}

export default function NewApp({
                                   defaultCollapsed = true,
                                   navCollapsedSize = 4,
                                   defaultLayout = [4, 32, 48]

                               }: NewAppProps) {
    const [isCollapsed, setIsCollapsed] = React.useState(defaultCollapsed);
    const [input, setInput] = useState(""); // User's input
    const [error, setError] = useState(""); // Error handling
    const [environment, setEnvironment] = useState({vars: {}}); // Environment state
    const [history, setHistory] = useState<HistoryItem[]>([]); // Explicit type for history

    // Initialize WASM once when the app starts
    useEffect(() => {
        const initializeWasm = async () => {
            try {
                await init({path: '/pkg/arithma_bg.wasm'});
            } catch (err) {
                console.error("WASM initialization failed:", err);
            }
        };

        initializeWasm();
    }, []);

    // Function to handle evaluating the input (for both equations and simple expressions)
    const handleEvaluate = async (latex: string) => {
        try {
            // Pass the environment as a JSON string to the WASM function
            const envJson = JSON.stringify(environment);

            // Pass LaTeX to Rust WASM for evaluation
            const result = await evaluate_latex_expression_js(latex, envJson);

            // Update the environment with the result (if necessary)
            const updatedEnv = {...environment};
            setEnvironment(updatedEnv);

            // Display the solution
            setHistory([...history, {input: latex, result}]);
            setError(""); // Clear any previous errors
        } catch (err: any) {
            setError(`Error: ${err.message || err}`);
        }
    };

    // Function to populate ExpressionInput when history item is clicked
    const handleHistoryItemClick = (latex: string) => {
        setInput(latex); // Set the input to the clicked LaTeX equation
    };

    return (
        <TooltipProvider delayDuration={0}>
            <div className="hidden h-full flex-col md:flex" style={{marginBottom: "3px"}}>
                <div
                    className="container flex flex-col items-start justify-between sm:flex-row sm:items-center sm:space-y-0 md:h-10 border-accent border-gray-300 shadow-sm p-2">
                    <span style={{marginLeft: "0px"}} className="text-lg font-extrabold flex items-center space-x-1">
                        <h2 className="text-lg">Arithma</h2>
                    </span>
                    <Infinity className="boldsymbol mr-1 size-8" style={{marginRight: "15px"}} />
                </div>
            </div>

            <ResizablePanelGroup
                direction="horizontal"
                className="h-full max-h-[800px] items-stretch"
            >
                <ResizablePanel
                    defaultSize={2}
                    collapsedSize={navCollapsedSize}
                    collapsible={true}
                    minSize={10}
                    maxSize={20}
                    onCollapse={() => {
                        setIsCollapsed(true)
                        document.cookie = `react-resizable-panels:collapsed=${JSON.stringify(
                            true
                        )}`
                    }}
                    onResize={() => {
                        setIsCollapsed(false)
                        document.cookie = `react-resizable-panels:collapsed=${JSON.stringify(
                            false
                        )}`
                    }}
                    className={cn(
                        isCollapsed &&
                        "min-w-[50px] transition-all duration-300 ease-in-out"
                    )}

                >
                    <Nav
                        isCollapsed={true}
                        links={[
                            {
                                title: "Calculator",
                                label: "",
                                icon: Calculator,
                                variant: "default",
                            },
                        ]}
                    />
                </ResizablePanel>
                <ResizablePanel defaultSize={defaultLayout[1]} minSize={30}>
                        {/* Math Expression Input */}
                        <ExpressionInput
                            input={input}
                            setInput={setInput}
                            handleEvaluate={handleEvaluate}
                        />

                        {/* History Section */}
                        <HistorySection
                            history={history}
                            onHistoryItemClick={handleHistoryItemClick} // Pass the click handler to HistorySection
                        />

                        {/* Display error */}
                        {error && <p className="error"><strong>{error}</strong></p>}
                </ResizablePanel>


            </ResizablePanelGroup>

        </TooltipProvider>
    )
        ;
}

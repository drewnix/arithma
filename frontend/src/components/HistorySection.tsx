import React, { useState } from 'react';
import katex from 'katex';
import 'katex/dist/katex.min.css';
import { Card } from "@/components/ui/card";
import { TooltipProvider } from "@radix-ui/react-tooltip";
import { Separator } from "@/components/ui/separator";

interface HistoryItem {
    input: string;
    result: string;
    errorMessage?: string;
}

interface HistorySectionProps {
    history: HistoryItem[];
    onHistoryItemClick: (latex: string) => void;
}

const HistorySection: React.FC<HistorySectionProps> = ({ history }) => {
    const [expandedIndex, setExpandedIndex] = useState<number | null>(null);

    const toggleExpand = (index: number) => {
        setExpandedIndex(expandedIndex === index ? null : index);
    };

    return (
        <TooltipProvider>
            <Card className="w-full max-w-2xl p-4 border-none shadow-none" style={{ paddingBottom: "30px" }}>
                <div className="space-y-4">
                    {/* Reverse the history list to show the most recent at the top */}
                    {history.slice(0).reverse().map((item, index) => (
                        <div key={index}>
                            <div
                                className={`p-2 bg-white rounded-md shadow-md flex flex-col justify-between cursor-pointer transition-all duration-300 ${
                                    expandedIndex === index ? 'expanded' : ''
                                }`}
                                style={{ marginRight: "10px" }}
                            >
                                <div
                                    className="flex justify-between items-center cursor-pointer"
                                    onClick={() => toggleExpand(index)} // Toggle expansion only when clicking the top part
                                >                                    {/* Left aligned LaTeX input */}
                                    <div
                                        className="flex-grow text-left"
                                        dangerouslySetInnerHTML={{
                                            __html: katex.renderToString(item.input, {
                                                throwOnError: false,
                                            }),
                                        }}
                                    ></div>
                                    <div
                                        className={`ml-4 font-bold ${item.result === 'Error' ? 'text-red-500' : 'text-teal-500'}`}
                                        dangerouslySetInnerHTML={{
                                            __html: katex.renderToString(item.result, {
                                                throwOnError: false,
                                            }),
                                        }}
                                    ></div>
                                </div>

                                {/* Error details: Visible when expanded */}
                                {expandedIndex === index && item.result === 'Error' && (
                                    <>
                                        <Separator className="my-2"/> {/* Light separator */}
                                        <div className="text-sm text-gray-600">
                                            <p className="text-red-500 font-bold">Error Details</p>
                                            <p>{item.errorMessage}.</p>
                                            <p><b>LaTeX:</b> {item.input}</p>
                                        </div>
                                    </>
                                )}
                            </div>
                        </div>
                    ))}
                </div>
            </Card>
        </TooltipProvider>
    );
};

export default HistorySection;
import React from 'react';
import katex from 'katex';
import 'katex/dist/katex.min.css';
import {Card} from "@/components/ui/card";
import {Tooltip} from "@/components/ui/tooltip";
import {TooltipProvider} from "@radix-ui/react-tooltip";

interface HistoryItem {
    input: string;
    result: string;
}

interface HistorySectionProps {
    history: HistoryItem[];
    onHistoryItemClick: (latex: string) => void;
}

const HistorySection: React.FC<HistorySectionProps> = ({history, onHistoryItemClick}) => {
    return (
        <TooltipProvider>
            <Card className="w-full max-w-2xl p-4  border-none shadow-none" style={{paddingBottom: "30px"}}>
                <div className="space-y-4">
                    {/* Reverse the history list to show the most recent at the top */}
                    {history.slice(0).reverse().map((item, index) => (
                        <Tooltip key={index} delayDuration={0}>
                            <div
                                className="p-2 bg-white rounded-md shadow-md flex justify-between items-center cursor-pointer"
                                 style={{marginRight: "10px"}}
                                onClick={() => onHistoryItemClick(item.input)} // Handle click on history item
                            >
                                {/* Left aligned LaTeX input */}
                                <div
                                    className="flex-grow text-left"
                                    dangerouslySetInnerHTML={{
                                        __html: katex.renderToString(item.input, {
                                            throwOnError: false,
                                        }),
                                    }}
                                ></div>

                                {/* Right aligned result */}
                                <span className="ml-4 font-bold text-teal-500">
                {item.result}
              </span>

                            </div>
                        </Tooltip>
                    ))}
                </div>
            </Card>
        </TooltipProvider>
    );
};

export default HistorySection;
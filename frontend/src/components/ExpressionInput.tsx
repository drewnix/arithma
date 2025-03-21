import {MathfieldElement} from "mathlive";
import "//unpkg.com/mathlive";

// Augment React's JSX types to include math-field
declare global {
    interface JSX {
        IntrinsicElements: {
            'math-field': React.DetailedHTMLProps<React.HTMLAttributes<MathfieldElement>, MathfieldElement>;
        }
    }
}
import React from "react";
import {Button} from "@/components/ui/button.tsx";
import {Card} from "@/components/ui/card";

interface ExpressionInputProps {
    input: string;
    setInput: (input: string) => void;
    handleEvaluate: (latex: string) => void;
}

const ExpressionInput: React.FC<ExpressionInputProps> = ({
                                                             input,
                                                             setInput,
                                                             handleEvaluate,
                                                         }) => {
    const handleKeyDown = (evt: React.KeyboardEvent<MathfieldElement>) => {
        const mathfield = evt.target as MathfieldElement;

        // Check if the Enter key is pressed
        if (evt.key === "Enter") {
            const latex = mathfield.getValue("latex-expanded"); // Get LaTeX value

            handleEvaluate(latex);
        } else {
            setInput(mathfield.getValue("latex-expanded")); // Update the input when other keys are pressed
        }
    };

    return (
        <Card className="w-full max-w-2xl p-2 rounded-md border-none">
            <math-field
                className="w-full"
                style={{
                    flexGrow: 1,
                    padding: "10px",
                    border: "1px solid #ccc",
                    borderRadius: "5px",
                    width: "500px",
                    background: "white",
                    color: "black",
                    paddingBottom: "10px",
                    marginBottom: "5px",
                    marginRight: "15px"
                }}
                onInput={(evt: React.FormEvent<MathfieldElement>) => {
                    const target = evt.target as MathfieldElement;
                    setInput(target.getValue());
                }}
                onKeyDown={handleKeyDown}
            >
                {input}
            </math-field>
            <Button style={{height: "63px", marginRight: "10px"}}
                    onClick={() => {
                        const mathfield = document.querySelector("math-field") as MathfieldElement;
                        const latex = mathfield.getValue("latex-expanded"); // Get LaTeX value
                        handleEvaluate(latex);
                    }}
            >
                Evaluate
            </Button>
        </Card>
    );
};

export default ExpressionInput;
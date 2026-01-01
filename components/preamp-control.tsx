import { useCallback, useEffect, useState } from "react";
import { Slider } from "@/components/ui/slider";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";

interface PreampControlProps {
    value: number;
    onChange: (value: number) => void;
}

export function PreampControl({ value, onChange }: PreampControlProps) {
    const [inputValue, setInputValue] = useState(value.toFixed(1));

    // Update input text when external value changes
    useEffect(() => {
        setInputValue(value.toFixed(1));
    }, [value]);

    const handleSliderChange = useCallback(
        (values: number[]) => {
            onChange(values[0]);
        },
        [onChange]
    );

    const handleInputChange = useCallback(
        (e: React.ChangeEvent<HTMLInputElement>) => {
            const newValue = e.target.value;
            setInputValue(newValue);

            const parsed = parseFloat(newValue);
            if (!isNaN(parsed) && parsed >= -20 && parsed <= 20) {
                onChange(parsed);
            }
        },
        [onChange]
    );

    const handleBlur = useCallback(() => {
        // Reset to valid number on blur if invalid
        setInputValue(value.toFixed(1));
    }, [value]);

    return (
        <div className="p-6 border border-border rounded-lg bg-card">
            <div className="flex items-center justify-between mb-4">
                <Label className="text-lg font-semibold">Preamp</Label>
                <div className="flex items-center gap-2">
                    <Input
                        value={inputValue}
                        onChange={handleInputChange}
                        onBlur={handleBlur}
                        className="w-20 text-right font-mono"
                        step={0.1}
                        min={-20}
                        max={20}
                        type="number"
                    />
                    <span className="text-sm text-muted-foreground">dB</span>
                </div>
            </div>
            <Slider
                value={[value]}
                onValueChange={handleSliderChange}
                min={-20}
                max={20}
                step={0.1}
                className="w-full"
            />
            <p className="text-xs text-muted-foreground mt-2">
                Adjust overall volume before EQ processing
            </p>
        </div>
    );
}

"use client";

import { useCallback, useState, useEffect } from "react";
import { Slider } from "@/components/ui/slider";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Power } from "lucide-react";
import type { ParametricBand, FilterType } from "@/lib/types";

interface BandEditorProps {
    band: ParametricBand;
    index: number;
    onUpdate: (id: string, updates: Partial<Omit<ParametricBand, "id">>) => void;
    onRemove: (id: string) => void;
    onApply: () => void;
}

import { FILTER_TYPE_SHORT } from "@/lib/constants";

export function BandEditor({
    band,
    index,
    onUpdate,
    onRemove,
    onApply,
}: BandEditorProps) {
    const [freqInput, setFreqInput] = useState(String(band.frequency));
    const [qInput, setQInput] = useState(band.q_factor.toFixed(2));

    useEffect(() => {
        setFreqInput(String(band.frequency));
    }, [band.frequency]);

    useEffect(() => {
        setQInput(band.q_factor.toFixed(2));
    }, [band.q_factor]);

    const handleGainChange = useCallback(
        (value: number[]) => {
            onUpdate(band.id, { gain: value[0] });
            onApply();
        },
        [band.id, onUpdate, onApply]
    );

    const cycleFilterType = useCallback(() => {
        const types: FilterType[] = ["peaking", "lowshelf", "highshelf"];
        const currentIndex = types.indexOf(band.filter_type);
        const nextType = types[(currentIndex + 1) % types.length];
        onUpdate(band.id, { filter_type: nextType });
        onApply();
    }, [band.id, band.filter_type, onUpdate, onApply]);

    const toggleEnabled = useCallback(() => {
        onUpdate(band.id, { enabled: !band.enabled });
        onApply();
    }, [band.id, band.enabled, onUpdate, onApply]);

    const handleFreqBlur = () => {
        const value = parseFloat(freqInput);
        if (!isNaN(value) && value >= 20 && value <= 20000) {
            onUpdate(band.id, { frequency: Math.round(value) });
            onApply();
        } else {
            setFreqInput(String(band.frequency));
        }
    };

    const handleQBlur = () => {
        const value = parseFloat(qInput);
        if (!isNaN(value) && value >= 0.1 && value <= 30) {
            onUpdate(band.id, { q_factor: value });
            onApply();
        } else {
            setQInput(band.q_factor.toFixed(2));
        }
    };

    return (
        <div className={`flex flex-col bg-card border border-border rounded-lg p-4 w-[140px] shrink-0 gap-3 transition-opacity ${!band.enabled ? "opacity-50" : ""}`}>
            {/* Header: Band number + Toggle + Remove */}
            <div className="flex items-center justify-between">
                <span className="text-xs text-muted-foreground font-medium">Band {index + 1}</span>
                <div className="flex items-center gap-1">
                    <button
                        type="button"
                        onClick={toggleEnabled}
                        className={`p-0.5 rounded transition-colors ${
                            band.enabled
                                ? "text-primary hover:text-primary/80"
                                : "text-muted-foreground hover:text-foreground"
                        }`}
                        title={band.enabled ? "Disable band" : "Enable band"}
                        aria-label={band.enabled ? "Disable band" : "Enable band"}
                    >
                        <Power className="h-3.5 w-3.5" />
                    </button>
                    <button
                        type="button"
                        onClick={() => onRemove(band.id)}
                        className="text-muted-foreground hover:text-destructive text-sm leading-none px-1"
                        aria-label="Remove band"
                    >
                        ×
                    </button>
                </div>
            </div>

            {/* Filter Type */}
            <Button
                size="sm"
                variant="outline"
                onClick={cycleFilterType}
                className="h-8 text-sm w-full"
            >
                {FILTER_TYPE_SHORT[band.filter_type]}
            </Button>

            {/* Frequency */}
            <div>
                <div className="text-xs text-muted-foreground mb-1">Frequency</div>
                <div className="flex items-center gap-1">
                    <Input
                        type="text"
                        value={freqInput}
                        onChange={(e) => setFreqInput(e.target.value)}
                        onBlur={handleFreqBlur}
                        onKeyDown={(e) => e.key === "Enter" && handleFreqBlur()}
                        className="h-8 text-sm text-center font-mono flex-1"
                    />
                    <span className="text-xs text-muted-foreground">Hz</span>
                </div>
            </div>

            {/* Gain Slider */}
            <div>
                <div className="flex justify-between items-center mb-2">
                    <span className="text-xs text-muted-foreground">Gain</span>
                    <span className="text-sm font-mono text-primary font-medium">
                        {band.gain > 0 ? "+" : ""}{band.gain.toFixed(1)} dB
                    </span>
                </div>
                <Slider
                    value={[band.gain]}
                    onValueChange={handleGainChange}
                    min={-15}
                    max={15}
                    step={0.1}
                    className="w-full"
                />
            </div>

            {/* Q Factor */}
            <div>
                <div className="text-xs text-muted-foreground mb-1">Q Factor</div>
                <Input
                    type="text"
                    value={qInput}
                    onChange={(e) => setQInput(e.target.value)}
                    onBlur={handleQBlur}
                    onKeyDown={(e) => e.key === "Enter" && handleQBlur()}
                    className="h-8 text-sm text-center font-mono"
                />
            </div>
        </div>
    );
}

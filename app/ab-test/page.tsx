"use client";

import { useCallback } from "react";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { useABTest } from "@/lib/use-ab-test";

export default function ABTestPage() {
    const {
        phase,
        profiles,
        setup,
        sessionState,
        results,
        error,
        isLoading,
        autoTrimDb,
        updateSetup,
        startSession,
        applyOption,
        recordAnswer,
        updateTrim,
        resetSession,
        resetTrimToAuto,
    } = useABTest();

    return (
        <main className="min-h-screen bg-background text-foreground">
            {/* Header */}
            <header className="border-b border-border bg-card/50 backdrop-blur-sm sticky top-0 z-50">
                <div className="container mx-auto px-6 py-4">
                    <div className="flex items-center justify-between">
                        <div className="flex items-center gap-4">
                            <Link href="/" className="text-muted-foreground hover:text-foreground">
                                ← Back
                            </Link>
                            <h1 className="text-2xl font-bold bg-gradient-to-r from-purple-400 to-pink-500 bg-clip-text text-transparent">
                                A/B Test
                            </h1>
                        </div>
                        {phase !== "setup" && (
                            <Button variant="outline" size="sm" onClick={resetSession}>
                                New Test
                            </Button>
                        )}
                    </div>
                </div>
            </header>

            {/* Error Display */}
            {error && (
                <div className="bg-destructive/15 text-destructive border-l-4 border-destructive p-4 container mx-auto mt-4 rounded-r-lg">
                    <p className="font-medium">Error</p>
                    <p className="text-sm opacity-90">{error}</p>
                </div>
            )}

            <div className="container mx-auto px-6 py-8">
                {phase === "setup" && (
                    <SetupScreen
                        profiles={profiles}
                        setup={setup}
                        autoTrimDb={autoTrimDb}
                        onUpdate={updateSetup}
                        onStart={startSession}
                        onResetTrim={resetTrimToAuto}
                        isLoading={isLoading}
                    />
                )}

                {phase === "running" && sessionState && (
                    <TestScreen
                        state={sessionState}
                        onApply={applyOption}
                        onAnswer={recordAnswer}
                        onTrimChange={updateTrim}
                        trimDb={setup.trimDb ?? 0}
                    />
                )}

                {phase === "results" && results && (
                    <ResultsScreen results={results} onReset={resetSession} />
                )}
            </div>
        </main>
    );
}

// ============================================================================
// Setup Screen
// ============================================================================

interface SetupScreenProps {
    profiles: string[];
    setup: ReturnType<typeof useABTest>["setup"];
    autoTrimDb: number;
    onUpdate: ReturnType<typeof useABTest>["updateSetup"];
    onStart: () => void;
    onResetTrim: () => void;
    isLoading: boolean;
}

function SetupScreen({ profiles, setup, autoTrimDb, onUpdate, onStart, onResetTrim, isLoading }: SetupScreenProps) {
    return (
        <div className="max-w-xl mx-auto space-y-8">
            <div className="bg-card border border-border rounded-lg p-6 space-y-6">
                <h2 className="text-xl font-semibold">Test Configuration</h2>

                {/* Preset Selection */}
                <div className="grid grid-cols-2 gap-4">
                    <div className="space-y-2">
                        <Label>Preset A</Label>
                        <select
                            className="w-full h-10 px-3 rounded-md border border-border bg-background"
                            value={setup.presetA}
                            onChange={(e) => onUpdate({ presetA: e.target.value })}
                        >
                            <option value="">Select preset...</option>
                            {profiles.map((p) => (
                                <option key={p} value={p}>{p}</option>
                            ))}
                        </select>
                    </div>
                    <div className="space-y-2">
                        <Label>Preset B</Label>
                        <select
                            className="w-full h-10 px-3 rounded-md border border-border bg-background"
                            value={setup.presetB}
                            onChange={(e) => onUpdate({ presetB: e.target.value })}
                        >
                            <option value="">Select preset...</option>
                            {profiles.map((p) => (
                                <option key={p} value={p}>{p}</option>
                            ))}
                        </select>
                    </div>
                </div>

                {/* Mode Selection */}
                <div className="space-y-2">
                    <Label>Test Mode</Label>
                    <div className="grid grid-cols-3 gap-2">
                        {[
                            { value: "ab", label: "A/B", desc: "Non-blind" },
                            { value: "blindab", label: "Blind A/B", desc: "Option 1/2" },
                            { value: "abx", label: "ABX", desc: "Guess X" },
                        ].map((mode) => (
                            <button
                                key={mode.value}
                                onClick={() => onUpdate({ mode: mode.value as typeof setup.mode })}
                                className={`p-3 rounded-lg border text-center transition-colors ${setup.mode === mode.value
                                    ? "border-primary bg-primary/10 text-primary"
                                    : "border-border hover:border-primary/50"
                                    }`}
                            >
                                <div className="font-medium">{mode.label}</div>
                                <div className="text-xs text-muted-foreground">{mode.desc}</div>
                            </button>
                        ))}
                    </div>
                </div>

                {/* Trials */}
                <div className="space-y-2">
                    <Label>Number of Trials: {setup.totalTrials}</Label>
                    <Slider
                        value={[setup.totalTrials]}
                        onValueChange={([v]) => onUpdate({ totalTrials: v })}
                        min={5}
                        max={30}
                        step={1}
                    />
                </div>

                {/* Loudness Trim */}
                <div className="space-y-2">
                    <div className="flex justify-between items-center">
                        <Label>Loudness Trim (Preset B)</Label>
                        <div className="flex items-center gap-2">
                            <span className="text-sm text-muted-foreground">
                                {setup.trimDb !== null ? `${setup.trimDb.toFixed(1)} dB` : `Auto (${autoTrimDb.toFixed(1)} dB)`}
                            </span>
                            {setup.trimDb !== null && (
                                <button
                                    onClick={onResetTrim}
                                    className="text-xs text-primary hover:underline"
                                >
                                    Reset to Auto
                                </button>
                            )}
                        </div>
                    </div>
                    <Slider
                        value={[setup.trimDb ?? autoTrimDb]}
                        onValueChange={([v]) => onUpdate({ trimDb: v })}
                        min={-15}
                        max={15}
                        step={0.1}
                    />
                    <p className="text-xs text-muted-foreground">
                        Auto-calculated based on EQ curves. Adjust manually if needed.
                    </p>
                </div>

                {/* Start Button */}
                <Button
                    onClick={onStart}
                    disabled={isLoading || !setup.presetA || !setup.presetB}
                    className="w-full"
                    size="lg"
                >
                    {isLoading ? "Starting..." : "Start Test"}
                </Button>
            </div>

            {/* Hotkey Help */}
            <div className="bg-card/50 border border-border rounded-lg p-4 text-sm text-muted-foreground">
                <p className="font-medium mb-2">Hotkeys during test:</p>
                <ul className="space-y-1">
                    <li><kbd className="px-1.5 py-0.5 bg-muted rounded">1</kbd> → Option 1 / A</li>
                    <li><kbd className="px-1.5 py-0.5 bg-muted rounded">2</kbd> → Option 2 / B</li>
                    <li><kbd className="px-1.5 py-0.5 bg-muted rounded">Space</kbd> → Toggle</li>
                    <li><kbd className="px-1.5 py-0.5 bg-muted rounded">X</kbd> → Reference X (ABX only)</li>
                </ul>
            </div>
        </div>
    );
}

// ============================================================================
// Test Screen
// ============================================================================

interface TestScreenProps {
    state: NonNullable<ReturnType<typeof useABTest>["sessionState"]>;
    onApply: (option: string) => void;
    onAnswer: (answer: string) => void;
    onTrimChange: (trim: number) => void;
    trimDb: number;
}

function TestScreen({ state, onApply, onAnswer, onTrimChange, trimDb }: TestScreenProps) {
    const isBlind = state.mode !== "ab";
    const isABX = state.mode === "abx";

    return (
        <div className="max-w-2xl mx-auto space-y-8">
            {/* Progress */}
            <div className="text-center">
                <div className="text-sm text-muted-foreground mb-2">
                    Trial {state.current_trial + 1} of {state.total_trials}
                </div>
                <div className="h-2 bg-muted rounded-full overflow-hidden">
                    <div
                        className="h-full bg-primary transition-all"
                        style={{ width: `${(state.current_trial / state.total_trials) * 100}%` }}
                    />
                </div>
            </div>

            {/* Switching Buttons */}
            <div className="bg-card border border-border rounded-lg p-8">
                <div className="flex justify-center gap-4 mb-8">
                    {isABX ? (
                        <>
                            <SwitchButton
                                label="A"
                                hotkey="1"
                                active={state.active_option === "a"}
                                onClick={() => onApply("A")}
                            />
                            <SwitchButton
                                label="B"
                                hotkey="2"
                                active={state.active_option === "b"}
                                onClick={() => onApply("B")}
                            />
                            <SwitchButton
                                label="X"
                                hotkey="X"
                                active={state.active_option === "x"}
                                onClick={() => onApply("X")}
                                variant="accent"
                            />
                        </>
                    ) : isBlind ? (
                        <>
                            <SwitchButton
                                label="Option 1"
                                hotkey="1"
                                active={state.active_option === "a"}
                                onClick={() => onApply("1")}
                            />
                            <SwitchButton
                                label="Option 2"
                                hotkey="2"
                                active={state.active_option === "b"}
                                onClick={() => onApply("2")}
                            />
                        </>
                    ) : (
                        <>
                            <SwitchButton
                                label="A"
                                hotkey="1"
                                active={state.active_option === "a"}
                                onClick={() => onApply("A")}
                            />
                            <SwitchButton
                                label="B"
                                hotkey="2"
                                active={state.active_option === "b"}
                                onClick={() => onApply("B")}
                            />
                        </>
                    )}
                </div>

                {/* Answer Section */}
                <div className="border-t border-border pt-6">
                    <p className="text-center text-muted-foreground mb-4">
                        {isABX
                            ? "Which preset is X?"
                            : "Which sounds better?"}
                    </p>
                    <div className="flex justify-center gap-4">
                        {isABX ? (
                            <>
                                <Button size="lg" onClick={() => onAnswer("X is A")}>
                                    X is A
                                </Button>
                                <Button size="lg" onClick={() => onAnswer("X is B")}>
                                    X is B
                                </Button>
                            </>
                        ) : isBlind ? (
                            <>
                                <Button size="lg" onClick={() => onAnswer("Option 1")}>
                                    Option 1
                                </Button>
                                <Button size="lg" onClick={() => onAnswer("Option 2")}>
                                    Option 2
                                </Button>
                            </>
                        ) : (
                            <>
                                <Button size="lg" onClick={() => onAnswer("A")}>
                                    Prefer A
                                </Button>
                                <Button size="lg" onClick={() => onAnswer("B")}>
                                    Prefer B
                                </Button>
                            </>
                        )}
                    </div>
                </div>
            </div>

            {/* Trim Control */}
            <div className="bg-card/50 border border-border rounded-lg p-4">
                <div className="flex justify-between mb-2">
                    <Label>Loudness Trim (B)</Label>
                    <span className="text-sm font-mono">{trimDb.toFixed(1)} dB</span>
                </div>
                <Slider
                    value={[trimDb]}
                    onValueChange={([v]) => onTrimChange(v)}
                    min={-15}
                    max={15}
                    step={0.1}
                />
            </div>
        </div>
    );
}

interface SwitchButtonProps {
    label: string;
    hotkey: string;
    active: boolean;
    onClick: () => void;
    variant?: "default" | "accent";
}

function SwitchButton({ label, hotkey, active, onClick, variant = "default" }: SwitchButtonProps) {
    return (
        <button
            onClick={onClick}
            className={`w-24 h-24 rounded-xl border-2 transition-all ${active
                ? variant === "accent"
                    ? "border-yellow-500 bg-yellow-500/20 text-yellow-500"
                    : "border-primary bg-primary/20 text-primary"
                : "border-border hover:border-primary/50"
                }`}
        >
            <div className="text-2xl font-bold">{label}</div>
            <div className="text-xs text-muted-foreground mt-1">
                <kbd className="px-1 py-0.5 bg-muted rounded">{hotkey}</kbd>
            </div>
        </button>
    );
}

// ============================================================================
// Results Screen
// ============================================================================

interface ResultsScreenProps {
    results: NonNullable<ReturnType<typeof useABTest>["results"]>;
    onReset: () => void;
}

function ResultsScreen({ results, onReset }: ResultsScreenProps) {
    const { statistics, mode } = results;

    return (
        <div className="max-w-xl mx-auto space-y-8">
            <div className="bg-card border border-border rounded-lg p-6 space-y-6">
                <h2 className="text-xl font-semibold text-center">Results</h2>

                {/* Verdict */}
                <div className={`p-4 rounded-lg text-center ${statistics.p_value < 0.05
                    ? "bg-green-500/20 border border-green-500"
                    : "bg-muted"
                    }`}>
                    <p className="text-lg font-medium">{statistics.verdict}</p>
                    <p className="text-sm text-muted-foreground">
                        p-value: {statistics.p_value.toFixed(4)}
                    </p>
                </div>

                {/* Statistics */}
                <div className="grid grid-cols-2 gap-4">
                    {mode === "abx" ? (
                        <>
                            <StatCard label="Correct" value={statistics.correct} />
                            <StatCard label="Incorrect" value={statistics.incorrect} />
                        </>
                    ) : (
                        <>
                            <StatCard label={`Preferred ${results.preset_a}`} value={statistics.preference_a} />
                            <StatCard label={`Preferred ${results.preset_b}`} value={statistics.preference_b} />
                        </>
                    )}
                </div>

                {/* Presets Revealed */}
                <div className="text-center text-sm text-muted-foreground">
                    <p>Preset A: <span className="font-medium text-foreground">{results.preset_a}</span></p>
                    <p>Preset B: <span className="font-medium text-foreground">{results.preset_b}</span></p>
                    <p>Trim applied to B: {results.trim_db.toFixed(1)} dB</p>
                </div>

                {/* Export Note */}
                <p className="text-xs text-muted-foreground text-center">
                    Results exported to Documents/EQAPO GUI/ab_results/
                </p>

                <Button onClick={onReset} className="w-full">
                    New Test
                </Button>
            </div>
        </div>
    );
}

function StatCard({ label, value }: { label: string; value: number }) {
    return (
        <div className="bg-muted/50 rounded-lg p-4 text-center">
            <div className="text-3xl font-bold">{value}</div>
            <div className="text-sm text-muted-foreground">{label}</div>
        </div>
    );
}

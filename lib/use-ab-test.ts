"use client";

import { useState, useCallback, useEffect } from "react";
import * as tauri from "@/lib/tauri";
import type { ABTestMode, ABStateForUI, ABSessionResults } from "@/lib/tauri";

export type ABTestPhase = "setup" | "running" | "results";

export interface ABTestSetup {
    presetA: string;
    presetB: string;
    mode: ABTestMode;
    totalTrials: number;
    trimDb: number | null; // null = use auto
}

export function useABTest() {
    const [phase, setPhase] = useState<ABTestPhase>("setup");
    const [profiles, setProfiles] = useState<string[]>([]);
    const [sessionState, setSessionState] = useState<ABStateForUI | null>(null);
    const [results, setResults] = useState<ABSessionResults | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [isLoading, setIsLoading] = useState(false);

    // Setup form state
    const [setup, setSetup] = useState<ABTestSetup>({
        presetA: "",
        presetB: "",
        mode: "blindab",
        totalTrials: 10,
        trimDb: null,
    });

    // Track auto-calculated trim separately
    const [autoTrimDb, setAutoTrimDb] = useState<number>(0);

    // Load profiles on mount
    useEffect(() => {
        tauri.listProfiles().then(setProfiles).catch(console.error);
    }, []);

    // Setup hotkey listeners during running phase
    useEffect(() => {
        if (phase !== "running" || !sessionState) return;

        const handleKeyDown = (e: KeyboardEvent) => {
            // Ignore if typing in input
            if (
                e.target instanceof HTMLInputElement ||
                e.target instanceof HTMLTextAreaElement
            ) {
                return;
            }

            const { mode } = sessionState;

            if (e.key === "1") {
                e.preventDefault();
                if (mode === "ab") {
                    handleApplyOption("A");
                } else {
                    handleApplyOption("1");
                }
            } else if (e.key === "2") {
                e.preventDefault();
                if (mode === "ab") {
                    handleApplyOption("B");
                } else {
                    handleApplyOption("2");
                }
            } else if (e.key === " ") {
                e.preventDefault();
                // Toggle between last two options
                const current = sessionState.active_option;
                if (mode === "abx") {
                    // In ABX, toggle between A and B (not X)
                    handleApplyOption(current === "a" ? "B" : "A");
                } else if (mode === "ab") {
                    handleApplyOption(current === "a" ? "B" : "A");
                } else {
                    handleApplyOption(current === "a" ? "2" : "1");
                }
            } else if (e.key === "x" || e.key === "X") {
                if (mode === "abx") {
                    e.preventDefault();
                    handleApplyOption("X");
                }
            }
        };

        window.addEventListener("keydown", handleKeyDown);
        return () => window.removeEventListener("keydown", handleKeyDown);
    }, [phase, sessionState]);

    const updateSetup = useCallback((updates: Partial<ABTestSetup>) => {
        setSetup((prev) => ({ ...prev, ...updates }));
    }, []);

    const startSession = useCallback(async () => {
        if (!setup.presetA || !setup.presetB) {
            setError("Please select both presets");
            return;
        }
        if (setup.presetA === setup.presetB) {
            setError("Presets must be different");
            return;
        }

        setIsLoading(true);
        setError(null);

        try {
            const state = await tauri.startABSession(
                setup.mode,
                setup.presetA,
                setup.presetB,
                setup.totalTrials,
                setup.trimDb ?? undefined
            );
            setSessionState(state);
            setPhase("running");
            setAutoTrimDb(state.auto_trim_db);

            // If user hadn't set a manual trim, use the auto value for the session
            if (setup.trimDb === null) {
                setSetup((prev) => ({ ...prev, trimDb: state.auto_trim_db }));
            }
        } catch (e) {
            setError(String(e));
        } finally {
            setIsLoading(false);
        }
    }, [setup]);

    const handleApplyOption = useCallback(async (option: string) => {
        try {
            await tauri.applyABOption(option);
            const state = await tauri.getABState();
            if (state) {
                setSessionState(state);
            }
        } catch (e) {
            setError(String(e));
        }
    }, []);

    const recordAnswer = useCallback(async (answer: string) => {
        setIsLoading(true);
        try {
            const state = await tauri.recordABAnswer(answer);
            setSessionState(state);

            // Check if session ended
            if (state.state === "results") {
                const sessionResults = await tauri.finishABSession();
                setResults(sessionResults);
                setPhase("results");
            }
        } catch (e) {
            setError(String(e));
        } finally {
            setIsLoading(false);
        }
    }, []);

    const updateTrim = useCallback(async (trimDb: number) => {
        setSetup((prev) => ({ ...prev, trimDb }));
        if (phase === "running") {
            try {
                await tauri.updateABTrim(trimDb);
            } catch (e) {
                console.error("Failed to update trim:", e);
            }
        }
    }, [phase]);

    const resetSession = useCallback(() => {
        setPhase("setup");
        setSessionState(null);
        setResults(null);
        setError(null);
        setSetup((prev) => ({ ...prev, trimDb: null })); // Reset trim to auto
    }, []);

    const resetTrimToAuto = useCallback(() => {
        setSetup((prev) => ({ ...prev, trimDb: null }));
    }, []);

    return {
        // State
        phase,
        profiles,
        setup,
        sessionState,
        results,
        error,
        isLoading,
        autoTrimDb, // Expose auto value

        // Actions
        updateSetup,
        startSession,
        applyOption: handleApplyOption,
        recordAnswer,
        updateTrim,
        resetSession,
        resetTrimToAuto, // Allow resetting to auto
    };
}

"use client";

import { useState, useCallback, useRef, useEffect } from "react";
import type { ParametricBand } from "@/lib/types";
import * as tauri from "@/lib/tauri";
import { generateId, handleExportProfile, handleImportProfile, handleExportTxt, handleImportTxt } from "./file-io";

function createDefaultBand(): ParametricBand {
    return {
        id: generateId(),
        filter_type: "peaking",
        frequency: 1000,
        gain: 0,
        q_factor: 1.41,
    };
}

export type SyncStatus = "synced" | "syncing" | "pending" | "error";

export function useEqualizer() {
    const [bands, setBands] = useState<ParametricBand[]>([createDefaultBand()]);
    const [preamp, setPreamp] = useState(0);
    const [profiles, setProfiles] = useState<string[]>([]);
    const [currentProfile, setCurrentProfile] = useState<string | null>(null);
    const [isLoading, setIsLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [syncStatus, setSyncStatus] = useState<SyncStatus>("synced");
    const [configPath, setConfigPath] = useState<string | null>(null);

    // Debounce timer ref
    const applyTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

    // Load profiles and persisted state on mount
    useEffect(() => {
        refreshProfiles();
        let path: string | null = null;

        const storedPath = localStorage.getItem("eq_config_path");
        if (storedPath) {
            setConfigPath(storedPath);
            path = storedPath;
        }

        // Load autosave
        const savedState = localStorage.getItem("eq_autosave");
        if (savedState) {
            try {
                const { bands: savedBands, preamp: savedPreamp, currentProfile: savedProfile } = JSON.parse(savedState);
                if (Array.isArray(savedBands) && savedBands.length > 0) setBands(savedBands);
                if (typeof savedPreamp === "number") setPreamp(savedPreamp);
                if (typeof savedProfile === "string" || savedProfile === null) setCurrentProfile(savedProfile);

                // Force sync to ensure live_config.txt matches the restored UI
                if (savedBands && typeof savedPreamp === "number") {
                    // We must use the local variables here, not state, because state updates are async
                    tauri.applyProfile(savedBands, savedPreamp, path).catch(console.error);
                }
            } catch (e) {
                console.error("Failed to parse autosave:", e);
            }
        }
    }, []);

    // Persist state on change
    useEffect(() => {
        // Debounce saving to local storage slightly to avoid rapid writes during slider drag
        const timer = setTimeout(() => {
            const state = { bands, preamp, currentProfile };
            localStorage.setItem("eq_autosave", JSON.stringify(state));
        }, 500);
        return () => clearTimeout(timer);
    }, [bands, preamp, currentProfile]);

    const setCustomConfigPath = useCallback((path: string | null) => {
        setConfigPath(path);
        if (path) localStorage.setItem("eq_config_path", path);
        else localStorage.removeItem("eq_config_path");
    }, []);

    const refreshProfiles = useCallback(async () => {
        try {
            const list = await tauri.listProfiles();
            setProfiles(list);
        } catch (e) {
            console.error("Failed to list profiles:", e);
        }
    }, []);

    const addBand = useCallback(() => {
        setBands((prev) => [...prev, createDefaultBand()]);
    }, []);

    const removeBand = useCallback((id: string) => {
        setBands((prev) => prev.filter((b) => b.id !== id));
    }, []);

    const updateBand = useCallback(
        (id: string, updates: Partial<Omit<ParametricBand, "id">>) => {
            setBands((prev) =>
                prev.map((b) => (b.id === id ? { ...b, ...updates } : b))
            );
        },
        []
    );

    const updatePreamp = useCallback((value: number) => {
        setPreamp(value);
    }, []);

    // Debounced apply - updates live config after 250ms of no changes
    const debouncedApply = useCallback((bandsToApply: ParametricBand[], preampValue: number) => {
        setSyncStatus("pending");
        if (applyTimeoutRef.current) {
            clearTimeout(applyTimeoutRef.current);
        }
        applyTimeoutRef.current = setTimeout(async () => {
            try {
                setSyncStatus("syncing");
                await tauri.applyProfile(bandsToApply, preampValue, configPath);
                setSyncStatus("synced");
                setError(null);
            } catch (e) {
                setSyncStatus("error");
                setError(String(e));
            }
        }, 250);
    }, [configPath]);

    // Force sync immediately
    const forceSync = useCallback(async () => {
        try {
            setSyncStatus("syncing");
            await tauri.applyProfile(bands, preamp, configPath);
            setSyncStatus("synced");
            setError(null);
        } catch (e) {
            setSyncStatus("error");
            setError(String(e));
        }
    }, [bands, preamp, configPath]);

    const saveCurrentProfile = useCallback(
        async (name: string) => {
            try {
                setIsLoading(true);
                await tauri.saveProfile(name, preamp, bands);
                setCurrentProfile(name);
                await refreshProfiles();
                setError(null);
            } catch (e) {
                setError(String(e));
            } finally {
                setIsLoading(false);
            }
        },
        [bands, preamp, refreshProfiles]
    );

    const loadProfileByName = useCallback(
        async (name: string) => {
            try {
                setIsLoading(true);
                const profile = await tauri.loadProfile(name);
                // Add IDs to loaded bands
                const bandsWithIds = profile.bands.map((b) => ({
                    ...b,
                    id: generateId(),
                }));

                const newPreamp = profile.preamp ?? 0;

                setBands(bandsWithIds);
                setPreamp(newPreamp);
                setCurrentProfile(name);

                // Auto-apply when loading
                setSyncStatus("syncing");
                await tauri.applyProfile(bandsWithIds, newPreamp, configPath);
                setSyncStatus("synced");
                setError(null);
            } catch (e) {
                setSyncStatus("error");
                setError(String(e));
            } finally {
                setIsLoading(false);
            }
        },
        [configPath]
    );

    const deleteProfileByName = useCallback(
        async (name: string) => {
            try {
                setIsLoading(true);
                await tauri.deleteProfile(name);
                if (currentProfile === name) {
                    setCurrentProfile(null);
                }
                await refreshProfiles();
                setError(null);
            } catch (e) {
                setError(String(e));
            } finally {
                setIsLoading(false);
            }
        },
        [currentProfile, refreshProfiles]
    );
    // Export current settings
    const exportProfile = useCallback(() => {
        handleExportProfile(currentProfile, preamp, bands);
    }, [bands, preamp, currentProfile]);

    // Import profile
    const importProfile = useCallback(() => {
        handleImportProfile(
            setBands,
            setPreamp,
            setCurrentProfile,
            setSyncStatus,
            setError,
            preamp,
            configPath
        );
    }, [preamp, configPath]);

    // Export to .txt
    const exportTxt = useCallback(() => {
        handleExportTxt(currentProfile, preamp, bands);
    }, [bands, preamp, currentProfile]);

    // Import from .txt
    const importTxt = useCallback(() => {
        handleImportTxt(
            setBands,
            setPreamp,
            setSyncStatus,
            setError
        );
    }, []);

    return {
        bands,
        preamp,
        profiles,
        currentProfile,
        isLoading,
        error,
        syncStatus,
        configPath,
        setCustomConfigPath,
        addBand,
        removeBand,
        updateBand,
        updatePreamp,
        debouncedApply,
        forceSync,
        saveCurrentProfile,
        loadProfileByName,
        deleteProfileByName,
        refreshProfiles,
        exportProfile,
        importProfile,
        exportTxt,
        importTxt,
    };
}

"use client";

import { useState, useCallback, useRef, useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
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

    // Debounce timer ref for saving to backend
    const saveTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
    // Debounce timer ref for applying to EqualizerAPO
    const applyTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

    // Load profiles and persisted state from backend on mount
    useEffect(() => {
        const loadInitialState = async () => {
            try {
                // Load profiles first
                const list = await tauri.listProfiles();
                setProfiles(list);

                // Load settings from backend
                const settings = await tauri.getSettings();

                // Convert bands to include IDs
                const bandsWithIds = settings.bands.map((b) => ({
                    ...b,
                    id: generateId(),
                }));

                setBands(bandsWithIds.length > 0 ? bandsWithIds : [createDefaultBand()]);
                setPreamp(settings.preamp);
                setCurrentProfile(settings.current_profile);
                setConfigPath(settings.config_path);

                // Apply to EqualizerAPO on startup
                await tauri.applyProfile(bandsWithIds.length > 0 ? bandsWithIds : [createDefaultBand()], settings.preamp, settings.config_path);
                setSyncStatus("synced");
            } catch (e) {
                console.error("Failed to load settings from backend:", e);
            }
        };

        loadInitialState();
    }, []);

    // Persist state to backend on change (debounced)
    useEffect(() => {
        // Debounce saving to backend to avoid rapid writes during slider drag
        if (saveTimeoutRef.current) {
            clearTimeout(saveTimeoutRef.current);
        }
        saveTimeoutRef.current = setTimeout(() => {
            tauri.updateSettings(bands, preamp, currentProfile, configPath).catch(console.error);
        }, 500);
        return () => {
            if (saveTimeoutRef.current) {
                clearTimeout(saveTimeoutRef.current);
            }
        };
    }, [bands, preamp, currentProfile, configPath]);

    // Listen for tray-initiated profile changes
    useEffect(() => {
        let unlisten: UnlistenFn | null = null;

        const setupListener = async () => {
            try {
                unlisten = await listen<string>("profile-changed-from-tray", async (event) => {
                    const profileName = event.payload;
                    try {
                        const profile = await tauri.loadProfile(profileName);
                        const bandsWithIds = profile.bands.map((b) => ({
                            ...b,
                            id: generateId(),
                        }));
                        setBands(bandsWithIds);
                        setPreamp(profile.preamp ?? 0);
                        setCurrentProfile(profileName);
                        setSyncStatus("synced");
                    } catch (e) {
                        console.error("Failed to sync profile from tray:", e);
                    }
                });
            } catch (e) {
                console.error("Failed to setup tray listener:", e);
            }
        };

        setupListener();

        return () => {
            if (unlisten) {
                unlisten();
            }
        };
    }, []);

    const setCustomConfigPath = useCallback((path: string | null) => {
        setConfigPath(path);
        // Will be saved to backend via the useEffect above
    }, []);

    const refreshProfiles = useCallback(async () => {
        try {
            const list = await tauri.listProfiles();
            setProfiles(list);
            // Also refresh tray menu
            await tauri.refreshTrayMenu();
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
                // Sync with backend tray state
                await tauri.setCurrentProfile(name);
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

                // Sync with backend tray state
                await tauri.setCurrentProfile(name);

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

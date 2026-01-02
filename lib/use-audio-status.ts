"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import {
    AudioOutputInfo,
    PeakMeterUpdate,
    getAudioOutputInfo,
    startPeakMeter,
    stopPeakMeter,
} from "./tauri";

export interface AudioStatus {
    device: AudioOutputInfo | null;
    peakDb: number;
    peakHold: number;
    isMonitoring: boolean;
    isLoading: boolean;
    error: string | null;
}

export function useAudioStatus() {
    const [device, setDevice] = useState<AudioOutputInfo | null>(null);
    const [peakDb, setPeakDb] = useState<number>(-100);
    const [peakHold, setPeakHold] = useState<number>(-100);
    const [isMonitoring, setIsMonitoring] = useState(false);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    const peakHoldTimeRef = useRef<number>(Date.now());
    const unlistenRef = useRef<UnlistenFn | null>(null);

    // Fetch device info
    const refreshDeviceInfo = useCallback(async () => {
        try {
            const info = await getAudioOutputInfo();
            setDevice(info);
            setError(null);
        } catch (err) {
            console.error("Failed to get audio output info:", err);
            setError(err instanceof Error ? err.message : String(err));
            setDevice(null);
        } finally {
            setIsLoading(false);
        }
    }, []);

    // Start monitoring
    const startMonitoring = useCallback(async () => {
        if (isMonitoring) return;

        try {
            // Subscribe to peak meter events
            const unlisten = await listen<PeakMeterUpdate>("peak_meter_update", (event) => {
                const { peak_db } = event.payload;
                setPeakDb(peak_db);

                // Peak hold logic (1 second hold)
                const now = Date.now();
                if (peak_db > peakHold || now - peakHoldTimeRef.current > 1000) {
                    setPeakHold(peak_db);
                    peakHoldTimeRef.current = now;
                }
            });

            unlistenRef.current = unlisten;

            // Start the peak meter on backend
            await startPeakMeter();
            setIsMonitoring(true);
            setError(null);
        } catch (err) {
            console.error("Failed to start peak monitoring:", err);
            setError(err instanceof Error ? err.message : String(err));
        }
    }, [isMonitoring, peakHold]);

    // Stop monitoring
    const stopMonitoring = useCallback(async () => {
        if (!isMonitoring) return;

        try {
            // Stop backend monitoring
            await stopPeakMeter();

            // Unsubscribe from events
            if (unlistenRef.current) {
                unlistenRef.current();
                unlistenRef.current = null;
            }

            setIsMonitoring(false);
            setPeakDb(-100);
            setPeakHold(-100);
        } catch (err) {
            console.error("Failed to stop peak monitoring:", err);
        }
    }, [isMonitoring]);

    // Initial fetch and auto-start monitoring
    useEffect(() => {
        refreshDeviceInfo();
        startMonitoring();

        // Cleanup on unmount
        return () => {
            stopPeakMeter().catch(console.error);
            if (unlistenRef.current) {
                unlistenRef.current();
            }
        };
    }, []);

    // Periodically refresh device info to catch format changes
    useEffect(() => {
        const intervalId = setInterval(() => {
            refreshDeviceInfo();
        }, 2000); // Poll every 2 seconds

        return () => {
            clearInterval(intervalId);
        };
    }, [refreshDeviceInfo]);

    // Listen for device changes (from backend events)
    useEffect(() => {
        let deviceChangeUnlisten: UnlistenFn | null = null;

        listen<AudioOutputInfo>("audio_device_changed", (event) => {
            setDevice(event.payload);
        }).then((unlisten) => {
            deviceChangeUnlisten = unlisten;
        }).catch(console.error);

        return () => {
            if (deviceChangeUnlisten) {
                deviceChangeUnlisten();
            }
        };
    }, []);

    return {
        device,
        peakDb,
        peakHold,
        isMonitoring,
        isLoading,
        error,
        startMonitoring,
        stopMonitoring,
        refresh: refreshDeviceInfo,
    };
}

// Helper function to format bit depth and sample rate
export function formatAudioFormat(device: AudioOutputInfo | null): string {
    if (!device) return "Format: Unknown";

    const bitDepth = device.format_tag === "IEEE Float"
        ? `${device.bit_depth}-bit float`
        : `${device.bit_depth}-bit`;

    const sampleRate = device.sample_rate >= 1000
        ? `${(device.sample_rate / 1000).toFixed(device.sample_rate % 1000 === 0 ? 0 : 1)} kHz`
        : `${device.sample_rate} Hz`;

    return `${bitDepth} / ${sampleRate}`;
}

// Helper function to get peak meter color class
export function getPeakMeterColorClass(peakDb: number): string {
    if (peakDb > -0.5) return "bg-red-500";      // Clipping
    if (peakDb > -6) return "bg-yellow-500";     // Warning
    if (peakDb > -20) return "bg-green-500";     // Normal
    return "bg-green-600/60";                     // Low/quiet
}

// Helper function to format dBFS display
export function formatPeakDb(peakDb: number): string {
    if (peakDb <= -100) return "-∞";
    if (peakDb > 0) return `+${peakDb.toFixed(1)}`;
    return peakDb.toFixed(1);
}

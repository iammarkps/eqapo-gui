"use client";

import { useAudioStatus, formatAudioFormat, formatPeakDb, getPeakMeterColorClass } from "@/lib/use-audio-status";
import { cn } from "@/lib/utils";
import { Volume2, VolumeX, AlertTriangle } from "lucide-react";

export function AudioStatusPanel() {
    const {
        device,
        peakDb,
        isMonitoring,
        isLoading,
        error,
    } = useAudioStatus();

    // Calculate meter width (0-100%)
    // Map from -60dB to 0dB range
    const meterWidth = Math.max(0, Math.min(100, ((peakDb + 60) / 60) * 100));

    // Determine status
    const isClipping = peakDb > -0.5;
    const isWarning = peakDb > -6;
    const hasDevice = device !== null;
    const hasError = error !== null;

    if (isLoading) {
        return (
            <div className="border-t border-border bg-card/50 backdrop-blur-sm">
                <div className="container mx-auto px-6 py-3">
                    <div className="flex items-center justify-center text-muted-foreground text-sm">
                        <div className="animate-pulse">Initializing audio monitoring...</div>
                    </div>
                </div>
            </div>
        );
    }

    return (
        <div className={cn(
            "border-t border-border bg-card/50 backdrop-blur-sm transition-colors",
            isClipping && "border-red-500/50 bg-red-500/5"
        )}>
            <div className="container mx-auto px-6 py-3">
                <div className="flex items-center justify-between gap-6">
                    {/* Device Section */}
                    <div className="flex items-center gap-3 min-w-0 flex-1">
                        {hasError ? (
                            <AlertTriangle className="w-4 h-4 text-yellow-500 shrink-0" />
                        ) : hasDevice ? (
                            <Volume2 className="w-4 h-4 text-muted-foreground shrink-0" />
                        ) : (
                            <VolumeX className="w-4 h-4 text-muted-foreground shrink-0" />
                        )}

                        <div className="min-w-0">
                            <div
                                className="text-sm font-medium truncate"
                                title={device?.device_id || "No device"}
                            >
                                {hasError
                                    ? "Audio Error"
                                    : device?.device_name || "Output: Not available"
                                }
                            </div>
                            {hasError && (
                                <div className="text-xs text-yellow-500 truncate">
                                    {error}
                                </div>
                            )}
                        </div>
                    </div>

                    {/* Format Section */}
                    <div className="text-sm text-muted-foreground whitespace-nowrap">
                        {formatAudioFormat(device)}
                    </div>

                    {/* Peak Meter Section */}
                    <div className="flex items-center gap-3 shrink-0">
                        {/* Visual Meter */}
                        <div className="w-24 h-3 bg-muted rounded-full overflow-hidden relative">
                            {/* Gradient background marks */}
                            <div className="absolute inset-0 flex">
                                <div className="flex-1 bg-gradient-to-r from-green-600/40 to-green-500/40" />
                                <div className="w-[10%] bg-yellow-500/40" />
                                <div className="w-[8%] bg-red-500/40" />
                            </div>

                            {/* Active meter bar */}
                            <div
                                className={cn(
                                    "absolute left-0 top-0 h-full transition-all duration-75 rounded-full",
                                    getPeakMeterColorClass(peakDb)
                                )}
                                style={{ width: `${meterWidth}%` }}
                            />
                        </div>

                        {/* Numeric Readout */}
                        <div className={cn(
                            "text-sm font-mono font-semibold w-16 text-right tabular-nums",
                            isClipping && "text-red-500",
                            isWarning && !isClipping && "text-yellow-500",
                            !isWarning && !isClipping && "text-muted-foreground"
                        )}>
                            {isMonitoring ? `${formatPeakDb(peakDb)} dB` : "—"}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}


import { cn } from "@/lib/utils";
import { AlertTriangle, CheckCircle } from "lucide-react";

interface PeakMeterProps {
    peakDb: number;
}

export function PeakMeter({ peakDb }: PeakMeterProps) {
    const isClipping = peakDb > 0.05; // Small tolerance
    const colorClass = isClipping ? "text-red-500" : "text-green-500";
    const bgClass = isClipping ? "bg-red-500/10 border-red-500/20" : "bg-green-500/10 border-green-500/20";

    return (
        <div className={cn(
            "flex items-center justify-between px-4 py-3 rounded-lg border backdrop-blur-sm transition-colors",
            bgClass
        )}>
            <div className="flex items-center gap-3">
                {isClipping ? (
                    <AlertTriangle className="w-5 h-5 text-red-500 animate-pulse" />
                ) : (
                    <CheckCircle className="w-5 h-5 text-green-500" />
                )}
                <div className="flex flex-col">
                    <span className={cn("font-bold text-sm", colorClass)}>
                        {isClipping ? "Potential Clipping Detected" : "Headroom Safe"}
                    </span>
                    <span className="text-xs text-muted-foreground">
                        {isClipping
                            ? "Reduce Preamp or EQ gains to avoid distortion."
                            : "Signal levels are within safe limits."}
                    </span>
                </div>
            </div>

            <div className="flex flex-col items-end">
                <span className="text-xs text-muted-foreground uppercase tracking-wider font-semibold">Peak Gain</span>
                <span className={cn("text-xl font-mono font-bold", colorClass)}>
                    {peakDb > 0 ? "+" : ""}{peakDb.toFixed(1)} dB
                </span>
            </div>
        </div>
    );
}

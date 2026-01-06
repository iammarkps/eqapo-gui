"use client";

import type { SyncStatus } from "@/lib/types";

interface SyncIndicatorProps {
    status: SyncStatus;
    onForceSync: () => void;
}

const statusConfig = {
    synced: { icon: "✓", text: "Synced", color: "text-green-500" },
    syncing: { icon: "⟳", text: "Syncing...", color: "text-yellow-500 animate-spin" },
    pending: { icon: "◌", text: "Pending", color: "text-muted-foreground" },
    error: { icon: "✕", text: "Error", color: "text-destructive" },
};

export function SyncIndicator({ status, onForceSync }: SyncIndicatorProps) {
    const config = statusConfig[status];

    return (
        <button
            onClick={onForceSync}
            className={`flex items-center gap-1.5 px-3 py-1.5 rounded-md hover:bg-muted transition-colors ${config.color}`}
            title="Click to force sync"
        >
            <span className={status === "syncing" ? "animate-spin" : ""}>{config.icon}</span>
            <span className="text-sm">{config.text}</span>
        </button>
    );
}

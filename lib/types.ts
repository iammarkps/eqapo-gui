// Shared types for EQAPO GUI

export type FilterType = 'peaking' | 'lowshelf' | 'highshelf';

// ============================================================================
// Sync State (Discriminated Union)
// ============================================================================

/**
 * Represents the synchronization state between the UI and EqualizerAPO.
 * Uses a discriminated union to ensure type-safe error handling.
 */
export type SyncState =
    | { status: "synced" }
    | { status: "syncing" }
    | { status: "pending" }
    | { status: "error"; message: string };

/**
 * Type alias for just the sync status (for components that only need the status string).
 */
export type SyncStatus = SyncState["status"];

export interface ParametricBand {
    id: string;
    filter_type: FilterType;
    frequency: number;
    gain: number;
    q_factor: number;
}

export interface EqProfile {
    name: string;
    preamp?: number;
    bands: Omit<ParametricBand, 'id'>[];
}

// Default frequency presets for quick access
export const FREQUENCY_PRESETS = [
    { label: 'Sub Bass', hz: 32 },
    { label: 'Bass', hz: 80 },
    { label: 'Low Mid', hz: 250 },
    { label: 'Mid', hz: 1000 },
    { label: 'High Mid', hz: 4000 },
    { label: 'Presence', hz: 8000 },
    { label: 'Air', hz: 16000 },
] as const;

// Filter type display names
export const FILTER_TYPE_LABELS: Record<FilterType, string> = {
    peaking: 'Peaking',
    lowshelf: 'Low Shelf',
    highshelf: 'High Shelf',
};

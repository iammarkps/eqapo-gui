import type { FilterType } from "./types";

export const FILTER_TYPE_LABELS: Record<FilterType, string> = {
    peaking: "Peaking",
    lowshelf: "Low Shelf",
    highshelf: "High Shelf",
};

export const FILTER_TYPE_SHORT: Record<FilterType, string> = {
    peaking: "PK",
    lowshelf: "LS",
    highshelf: "HS",
};

// Audiophile frequency band labels with Hz
export const AUDIOPHILE_BANDS = [
    { freq: 32, label: "Sub Bass", hz: "32Hz" },
    { freq: 60, label: "Bass", hz: "60Hz" },
    { freq: 250, label: "Low Mids", hz: "250Hz" },
    { freq: 1000, label: "Mids", hz: "1kHz" },
    { freq: 4000, label: "Upper Mids", hz: "4kHz" },
    { freq: 8000, label: "Presence", hz: "8kHz" },
    { freq: 16000, label: "Air", hz: "16kHz" },
];

export const FREQUENCY_PRESETS = [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];

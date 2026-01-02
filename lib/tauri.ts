"use client";

import { invoke } from "@tauri-apps/api/core";
import type { ParametricBand, EqProfile } from "./types";

// Strip the 'id' field before sending to Rust
type RustBand = Omit<ParametricBand, "id">;

// Settings returned from backend
export interface AppSettings {
    current_profile: string | null;
    config_path: string | null;
    bands: RustBand[];
    preamp: number;
}

function toRustBands(bands: ParametricBand[]): RustBand[] {
    return bands.map(({ filter_type, frequency, gain, q_factor }) => ({
        filter_type,
        frequency,
        gain,
        q_factor,
    }));
}

export async function listProfiles(): Promise<string[]> {
    return invoke<string[]>("list_profiles");
}

export async function loadProfile(name: string): Promise<EqProfile> {
    return invoke<EqProfile>("load_profile", { name });
}

export async function saveProfile(
    name: string,
    preamp: number,
    bands: ParametricBand[]
): Promise<void> {
    return invoke("save_profile", { name, preamp, bands: toRustBands(bands) });
}

export async function applyProfile(bands: ParametricBand[], preamp: number, configPath?: string | null): Promise<void> {
    return invoke("apply_profile", { bands: toRustBands(bands), preamp, configPath });
}

export async function deleteProfile(name: string): Promise<void> {
    return invoke("delete_profile", { name });
}

export async function getCurrentProfile(): Promise<string | null> {
    return invoke<string | null>("get_current_profile");
}

export async function setCurrentProfile(name: string | null): Promise<void> {
    return invoke("set_current_profile", { name });
}

export async function refreshTrayMenu(): Promise<void> {
    return invoke("refresh_tray_menu");
}

export async function getSettings(): Promise<AppSettings> {
    return invoke<AppSettings>("get_settings");
}

export async function updateSettings(
    bands: ParametricBand[],
    preamp: number,
    currentProfile: string | null,
    configPath: string | null
): Promise<void> {
    return invoke("update_settings", {
        bands: toRustBands(bands),
        preamp,
        currentProfile,
        configPath,
    });
}

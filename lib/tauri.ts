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

// ============================================================================
// A/B Test Types and Commands
// ============================================================================

export type ABTestMode = "ab" | "blindab" | "abx";
export type SessionState = "setup" | "running" | "results";
export type ActiveOption = "a" | "b" | "x";

export interface ABStateForUI {
    mode: ABTestMode;
    state: SessionState;
    current_trial: number;
    total_trials: number;
    trim_db: number;
    auto_trim_db: number;
    active_option: ActiveOption | null;
    preset_a: string | null;
    preset_b: string | null;
}

export interface ABAnswer {
    trial: number;
    hidden_mapping: boolean;
    x_is_a: boolean | null;
    user_choice: string;
    correct: boolean | null;
    time_ms: number;
    trim_db: number;
}

export interface ABStatistics {
    preference_a: number;
    preference_b: number;
    correct: number;
    incorrect: number;
    p_value: number;
    verdict: string;
}

export interface ABSessionResults {
    mode: ABTestMode;
    preset_a: string;
    preset_b: string;
    trim_db: number;
    total_trials: number;
    answers: ABAnswer[];
    statistics: ABStatistics;
}

export async function startABSession(
    mode: ABTestMode,
    presetA: string,
    presetB: string,
    totalTrials: number,
    trimDb?: number
): Promise<ABStateForUI> {
    return invoke<ABStateForUI>("start_ab_session", {
        mode,
        presetA,
        presetB,
        totalTrials,
        trimDb,
    });
}

export async function applyABOption(option: string): Promise<void> {
    return invoke("apply_ab_option", { option });
}

export async function recordABAnswer(answer: string): Promise<ABStateForUI> {
    return invoke<ABStateForUI>("record_ab_answer", { answer });
}

export async function getABState(): Promise<ABStateForUI | null> {
    return invoke<ABStateForUI | null>("get_ab_state");
}

export async function finishABSession(): Promise<ABSessionResults> {
    return invoke<ABSessionResults>("finish_ab_session");
}

export async function updateABTrim(trimDb: number): Promise<void> {
    return invoke("update_ab_trim", { trimDb });
}

// ============================================================================
// Audio Status Types and Commands
// ============================================================================

export interface AudioOutputInfo {
    device_name: string;
    device_id: string;
    sample_rate: number;
    bit_depth: number;
    channel_count: number;
    is_default: boolean;
    format_tag: string;
}

export interface PeakMeterUpdate {
    peak_db: number;
    peak_linear: number;
    timestamp: number;
}

export async function getAudioOutputInfo(): Promise<AudioOutputInfo> {
    return invoke<AudioOutputInfo>("get_audio_output_info");
}

export async function startPeakMeter(): Promise<void> {
    return invoke("start_peak_meter");
}

export async function stopPeakMeter(): Promise<void> {
    return invoke("stop_peak_meter");
}

export async function getCurrentPeak(): Promise<PeakMeterUpdate> {
    return invoke<PeakMeterUpdate>("get_current_peak");
}


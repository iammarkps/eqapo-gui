"use client";

import { useCallback, useMemo } from "react";
import { Button } from "@/components/ui/button";
import { BandEditor } from "@/components/band-editor";
import { ProfileSelector } from "@/components/profile-selector";
import { EqGraph } from "@/components/eq-graph";
import { PeakMeter } from "@/components/peak-meter";
import { SyncIndicator } from "@/components/ui/sync-indicator";
import { PreampControl } from "@/components/preamp-control";
import { SetupDialog } from "@/components/setup-dialog";
import { useEqualizer } from "@/lib/use-equalizer";
import { calculatePeakGain } from "@/lib/audio-math";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
    DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import { open } from '@tauri-apps/plugin-dialog';

export default function Home() {
    const {
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
        exportProfile,
        importProfile,
        exportTxt,
        importTxt,
    } = useEqualizer();

    // Trigger debounced apply when bands or preamp change
    const handleApplyDebounced = useCallback(() => {
        debouncedApply(bands, preamp);
    }, [bands, preamp, debouncedApply]);

    const handlePreampChange = useCallback(
        (value: number) => {
            updatePreamp(value);
            debouncedApply(bands, value);
        },
        [updatePreamp, debouncedApply, bands]
    );

    const handleSetConfigPath = useCallback(async () => {
        try {
            const selected = await open({
                multiple: false,
                filters: [{
                    name: 'Config File',
                    extensions: ['txt']
                }]
            });

            if (selected && typeof selected === 'string') {
                setCustomConfigPath(selected);
            }
        } catch (err) {
            console.error("Failed to select path", err);
        }
    }, [setCustomConfigPath]);

    const peakGain = useMemo(() => calculatePeakGain(bands, preamp), [bands, preamp]);

    return (
        <>
            <SetupDialog open={!configPath} onSetPath={handleSetConfigPath} />
            <main className="min-h-screen bg-background text-foreground">
                {/* Header */}
                <header className="border-b border-border bg-card/50 backdrop-blur-sm sticky top-0 z-50">
                    <div className="container mx-auto px-6 py-4">
                        <div className="flex items-center justify-between">
                            <div className="flex items-center gap-4">
                                <h1 className="text-2xl font-bold bg-gradient-to-r from-purple-400 to-pink-500 bg-clip-text text-transparent">
                                    AntigravityEQ
                                </h1>
                                <SyncIndicator status={syncStatus} onForceSync={forceSync} />
                            </div>

                            <div className="flex items-center gap-3">
                                <DropdownMenu>
                                    <DropdownMenuTrigger asChild>
                                        <Button variant="ghost" size="sm">File</Button>
                                    </DropdownMenuTrigger>
                                    <DropdownMenuContent>
                                        <DropdownMenuItem onSelect={handleSetConfigPath}>
                                            Set Live Config Path...
                                        </DropdownMenuItem>
                                        {configPath && (
                                            <DropdownMenuItem disabled className="text-xs opacity-50">
                                                Using: {configPath.split('\\').pop()}
                                            </DropdownMenuItem>
                                        )}
                                        <DropdownMenuSeparator />
                                        <DropdownMenuItem onSelect={importProfile}>
                                            Import JSON Profile
                                        </DropdownMenuItem>
                                        <DropdownMenuItem onSelect={importTxt}>
                                            Import EQ APO (.txt)
                                        </DropdownMenuItem>
                                        <DropdownMenuSeparator />
                                        <DropdownMenuItem onSelect={exportProfile}>
                                            Export JSON Profile
                                        </DropdownMenuItem>
                                        <DropdownMenuItem onSelect={exportTxt}>
                                            Export EQ APO (.txt)
                                        </DropdownMenuItem>
                                    </DropdownMenuContent>
                                </DropdownMenu>

                                <ProfileSelector
                                    profiles={profiles}
                                    currentProfile={currentProfile}
                                    onSelect={loadProfileByName}
                                    onSave={saveCurrentProfile}
                                    onDelete={deleteProfileByName}
                                    isLoading={isLoading}
                                />

                                <a href="/ab-test">
                                    <Button variant="outline" size="sm">
                                        A/B Test
                                    </Button>
                                </a>
                            </div>
                        </div>
                    </div>
                </header>

                {/* Error Display */}
                {error && (
                    <div className="bg-destructive/15 text-destructive border-l-4 border-destructive p-4 container mx-auto mt-4 rounded-r-lg">
                        <p className="font-medium">Error Occurred</p>
                        <p className="text-sm opacity-90">{error}</p>
                    </div>
                )}

                <div className="container mx-auto px-6 py-8">
                    {/* Visualizer & Preamp */}
                    <div className="flex flex-col gap-6 mb-8">
                        {/* Preamp Section */}
                        <div className="w-full">
                            <PreampControl value={preamp} onChange={handlePreampChange} />
                        </div>

                        {/* Graph Section */}
                        <div className="bg-card border border-border rounded-lg p-6 shadow-sm flex flex-col gap-6">
                            <EqGraph bands={bands} preamp={preamp} />
                            <div className="border-t border-border pt-4">
                                <PeakMeter peakDb={peakGain} />
                            </div>
                        </div>
                    </div>

                    {/* Toolbar */}
                    <div className="flex items-center justify-between mb-4">
                        <h2 className="text-xl font-semibold opacity-90">Parametric Bands</h2>
                        <span className="text-xs text-muted-foreground font-mono">
                            {bands.length} / 32 Bands
                        </span>
                    </div>

                    {/* Band Editors */}
                    <div className="flex gap-4 overflow-x-auto pb-4 scrollbar-thin scrollbar-thumb-border scrollbar-track-transparent">
                        {bands.map((band, index) => (
                            <BandEditor
                                key={band.id}
                                index={index}
                                band={band}
                                onUpdate={updateBand}
                                onRemove={removeBand}
                                onApply={handleApplyDebounced}
                            />
                        ))}

                        {/* Add Band Button */}
                        <button
                            onClick={addBand}
                            className="flex flex-col items-center justify-center gap-2 p-4 w-[140px] shrink-0 border-2 border-dashed border-border rounded-lg hover:border-primary hover:bg-primary/5 transition-colors text-muted-foreground hover:text-primary group"
                        >
                            <span className="text-3xl group-hover:scale-110 transition-transform">+</span>
                            <span className="text-sm">Add Band</span>
                        </button>
                    </div>
                </div>

                {/* Footer */}
                <footer className="border-t border-border mt-12 py-6 bg-card/30">
                    <div className="container mx-auto px-6 text-center text-muted-foreground text-sm">
                        <p>AntigravityEQ - EqualizerAPO GUI Manager</p>
                    </div>
                </footer>
            </main>
        </>
    );
}

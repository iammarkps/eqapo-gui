"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
    Popover,
    PopoverContent,
    PopoverTrigger,
} from "@/components/ui/popover";
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
} from "@/components/ui/command";

interface ProfileSelectorProps {
    profiles: string[];
    currentProfile: string | null;
    onSelect: (name: string) => void;
    onSave: (name: string) => void;
    onDelete: (name: string) => void;
    isLoading: boolean;
}

export function ProfileSelector({
    profiles,
    currentProfile,
    onSelect,
    onSave,
    onDelete,
    isLoading,
}: ProfileSelectorProps) {
    const [open, setOpen] = useState(false);
    const [saveOpen, setSaveOpen] = useState(false);
    const [newName, setNewName] = useState("");

    const handleSave = () => {
        if (newName.trim()) {
            onSave(newName.trim());
            setNewName("");
            setSaveOpen(false);
        }
    };

    return (
        <div className="flex items-center gap-3">
            {/* Profile Selector */}
            <Popover open={open} onOpenChange={setOpen}>
                <PopoverTrigger asChild>
                    <Button
                        variant="outline"
                        className="w-[200px] justify-between"
                        disabled={isLoading}
                    >
                        {currentProfile || "Select profile..."}
                        <span className="ml-2 opacity-50">▼</span>
                    </Button>
                </PopoverTrigger>
                <PopoverContent className="w-[200px] p-0" align="start">
                    <Command>
                        <CommandInput placeholder="Search profiles..." />
                        <CommandList>
                            <CommandEmpty>No profiles found.</CommandEmpty>
                            <CommandGroup>
                                {profiles.map((profile) => (
                                    <CommandItem
                                        key={profile}
                                        value={profile}
                                        onSelect={() => {
                                            onSelect(profile);
                                            setOpen(false);
                                        }}
                                        className="flex justify-between"
                                    >
                                        <span>{profile}</span>
                                        {currentProfile === profile && (
                                            <span className="text-primary">✓</span>
                                        )}
                                    </CommandItem>
                                ))}
                            </CommandGroup>
                        </CommandList>
                    </Command>
                </PopoverContent>
            </Popover>

            {/* Save Profile */}
            <Popover open={saveOpen} onOpenChange={setSaveOpen}>
                <PopoverTrigger asChild>
                    <Button variant="secondary" disabled={isLoading}>
                        Save As
                    </Button>
                </PopoverTrigger>
                <PopoverContent className="w-[250px]" align="start">
                    <div className="space-y-3">
                        <Input
                            placeholder="Profile name..."
                            value={newName}
                            onChange={(e) => setNewName(e.target.value)}
                            onKeyDown={(e) => e.key === "Enter" && handleSave()}
                        />
                        <Button onClick={handleSave} className="w-full" disabled={!newName.trim()}>
                            Save Profile
                        </Button>
                    </div>
                </PopoverContent>
            </Popover>

            {/* Quick Save (if profile selected) */}
            {currentProfile && (
                <Button
                    variant="outline"
                    onClick={() => onSave(currentProfile)}
                    disabled={isLoading}
                >
                    Save
                </Button>
            )}

            {/* Delete */}
            {currentProfile && (
                <Button
                    variant="ghost"
                    onClick={() => onDelete(currentProfile)}
                    disabled={isLoading}
                    className="text-destructive hover:text-destructive"
                >
                    Delete
                </Button>
            )}
        </div>
    );
}

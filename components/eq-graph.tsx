"use client";

import { useEffect, useRef, useMemo, useState } from "react";
import type { ParametricBand } from "@/lib/types";
import {
    NUM_POINTS,
    LOG_FREQ_MIN,
    LOG_FREQ_MAX,
    FREQUENCIES,
    calcBiquadMagnitudeDb
} from "@/lib/audio-math";
import { AUDIOPHILE_BANDS } from "@/lib/constants";

interface EqGraphProps {
    bands: ParametricBand[];
    preamp: number;
    height?: number;
    sampleRate?: number;
}

export function EqGraph({ bands, preamp, height = 220, sampleRate = 48000 }: EqGraphProps) {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const [containerWidth, setContainerWidth] = useState(800);

    const responseDb = useMemo(() => {
        const response = new Float32Array(NUM_POINTS);

        for (let i = 0; i < NUM_POINTS; i++) {
            let totalDb = preamp;
            const freq = FREQUENCIES[i];

            for (const band of bands) {
                totalDb += calcBiquadMagnitudeDb(
                    freq,
                    band.frequency,
                    band.gain,
                    band.q_factor,
                    band.filter_type,
                    sampleRate
                );
            }

            response[i] = totalDb;
        }

        return response;
    }, [bands, preamp, sampleRate]);

    // Handle resize with ResizeObserver
    useEffect(() => {
        const container = containerRef.current;
        if (!container) return;

        const observer = new ResizeObserver((entries) => {
            for (const entry of entries) {
                setContainerWidth(entry.contentRect.width);
            }
        });

        observer.observe(container);
        setContainerWidth(container.clientWidth);

        return () => observer.disconnect();
    }, []);

    // Render to canvas - triggered by width change
    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas || containerWidth <= 0) return;

        const ctx = canvas.getContext("2d");
        if (!ctx) return;

        const width = containerWidth;
        const dpr = window.devicePixelRatio || 1;

        // Set canvas size with DPR for sharp rendering
        canvas.width = width * dpr;
        canvas.height = height * dpr;
        canvas.style.width = `${width}px`;
        canvas.style.height = `${height}px`;
        ctx.scale(dpr, dpr);

        // Clear
        ctx.fillStyle = "hsl(0, 0%, 6%)";
        ctx.fillRect(0, 0, width, height);

        // Draw grid
        ctx.strokeStyle = "hsl(0, 0%, 15%)";
        ctx.lineWidth = 1;

        const dbRange = 30;
        const dbStep = 6;

        // Horizontal grid lines (dB)
        ctx.fillStyle = "hsl(0, 0%, 35%)";
        ctx.font = "10px system-ui";
        for (let db = -12; db <= 12; db += dbStep) {
            const y = height / 2 - (db / dbRange) * height;
            ctx.beginPath();
            ctx.moveTo(0, y);
            ctx.lineTo(width, y);
            ctx.stroke();
            if (db !== 0) {
                ctx.fillText(`${db > 0 ? "+" : ""}${db}dB`, 4, y - 4);
            }
        }

        // Audiophile frequency band markers
        ctx.font = "10px system-ui";
        ctx.textAlign = "center";

        for (const band of AUDIOPHILE_BANDS) {
            const x = ((Math.log10(band.freq) - LOG_FREQ_MIN) / (LOG_FREQ_MAX - LOG_FREQ_MIN)) * width;

            // Vertical line
            ctx.strokeStyle = "hsl(0, 0%, 18%)";
            ctx.beginPath();
            ctx.moveTo(x, 0);
            ctx.lineTo(x, height - 32);
            ctx.stroke();

            // Labels - Name and Hz
            ctx.fillStyle = "hsl(0, 0%, 55%)";
            ctx.fillText(band.label, x, height - 18);
            ctx.fillStyle = "hsl(0, 0%, 40%)";
            ctx.font = "9px system-ui";
            ctx.fillText(band.hz, x, height - 6);
            ctx.font = "10px system-ui";
        }
        ctx.textAlign = "left";

        // Draw 0dB line
        ctx.strokeStyle = "hsl(0, 0%, 35%)";
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(0, height / 2);
        ctx.lineTo(width, height / 2);
        ctx.stroke();
        ctx.fillStyle = "hsl(0, 0%, 50%)";
        ctx.fillText("0dB", 4, height / 2 - 4);

        // Draw frequency response curve
        ctx.strokeStyle = "hsl(262, 83%, 58%)";
        ctx.lineWidth = 2.5;
        ctx.beginPath();

        for (let i = 0; i < NUM_POINTS; i++) {
            const x = (i / (NUM_POINTS - 1)) * width;
            const db = Math.max(-15, Math.min(15, responseDb[i]));
            const y = height / 2 - (db / dbRange) * height;

            if (i === 0) {
                ctx.moveTo(x, y);
            } else {
                ctx.lineTo(x, y);
            }
        }
        ctx.stroke();

        // Fill under curve
        ctx.lineTo(width, height / 2);
        ctx.lineTo(0, height / 2);
        ctx.closePath();
        const gradient = ctx.createLinearGradient(0, 0, 0, height);
        gradient.addColorStop(0, "hsla(262, 83%, 58%, 0.25)");
        gradient.addColorStop(0.5, "hsla(262, 83%, 58%, 0.08)");
        gradient.addColorStop(1, "hsla(262, 83%, 58%, 0.25)");
        ctx.fillStyle = gradient;
        ctx.fill();

    }, [responseDb, height, containerWidth]);

    return (
        <div ref={containerRef} className="w-full">
            <canvas
                ref={canvasRef}
                className="rounded-lg border border-border w-full"
            />
        </div>
    );
}

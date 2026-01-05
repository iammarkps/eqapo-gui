"use client";

import { useEffect, useRef, useMemo, useState } from "react";
import { useTheme } from "next-themes";
import type { ParametricBand } from "@/lib/types";
import {
    NUM_POINTS,
    LOG_FREQ_MIN,
    LOG_FREQ_MAX,
    FREQUENCIES,
    calcBiquadMagnitudeDb
} from "@/lib/audio-math";
import { AUDIOPHILE_BANDS } from "@/lib/constants";

// =============================================================================
// Constants
// =============================================================================

const GRAPH_COLORS_DARK = {
    background: "hsl(0, 0%, 6%)",
    gridLine: "hsl(0, 0%, 15%)",
    gridText: "hsl(0, 0%, 35%)",
    zeroLine: "hsl(0, 0%, 35%)",
    zeroLineText: "hsl(0, 0%, 50%)",
    freqMarkerLine: "hsl(0, 0%, 18%)",
    freqMarkerLabel: "hsl(0, 0%, 55%)",
    freqMarkerHz: "hsl(0, 0%, 40%)",
    curve: "hsl(262, 83%, 58%)",
    curveGradientTop: "hsla(262, 83%, 58%, 0.25)",
    curveGradientMid: "hsla(262, 83%, 58%, 0.08)",
    curveGradientBottom: "hsla(262, 83%, 58%, 0.25)",
} as const;

const GRAPH_COLORS_LIGHT = {
    background: "hsl(0, 0%, 98%)",
    gridLine: "hsl(0, 0%, 88%)",
    gridText: "hsl(0, 0%, 45%)",
    zeroLine: "hsl(0, 0%, 70%)",
    zeroLineText: "hsl(0, 0%, 40%)",
    freqMarkerLine: "hsl(0, 0%, 90%)",
    freqMarkerLabel: "hsl(0, 0%, 35%)",
    freqMarkerHz: "hsl(0, 0%, 50%)",
    curve: "hsl(262, 83%, 50%)",
    curveGradientTop: "hsla(262, 83%, 50%, 0.30)",
    curveGradientMid: "hsla(262, 83%, 50%, 0.10)",
    curveGradientBottom: "hsla(262, 83%, 50%, 0.30)",
};

interface GraphColors {
    background: string;
    gridLine: string;
    gridText: string;
    zeroLine: string;
    zeroLineText: string;
    freqMarkerLine: string;
    freqMarkerLabel: string;
    freqMarkerHz: string;
    curve: string;
    curveGradientTop: string;
    curveGradientMid: string;
    curveGradientBottom: string;
}

const DB_RANGE = 40;
const DB_STEP = 5;
const DB_MAX_DISPLAY = Math.floor(DB_RANGE / 2);
const DB_MIN_DISPLAY = -DB_MAX_DISPLAY;
const CURVE_LINE_WIDTH = 2.5;
const GRID_LINE_WIDTH = 1;
const FONT_MAIN = "12px system-ui";
const FONT_SMALL = "10px system-ui";
const LABEL_PADDING = 4;
const TOP_PADDING = 20;
const LABEL_AREA_HEIGHT = 32;

// =============================================================================
// Types
// =============================================================================

interface EqGraphProps {
    bands: ParametricBand[];
    preamp: number;
    height?: number;
    sampleRate?: number;
}

interface DrawContext {
    ctx: CanvasRenderingContext2D;
    width: number;
    height: number;
    colors: GraphColors;
}

// =============================================================================
// Drawing Helper Functions
// =============================================================================

/** Convert dB value to Y coordinate */
function dbToY(db: number, height: number): number {
    // Graph area excludes top padding and bottom label area
    const graphHeight = height - TOP_PADDING - LABEL_AREA_HEIGHT;
    const displayRange = DB_MAX_DISPLAY - DB_MIN_DISPLAY;
    // Map DB_MAX_DISPLAY to TOP_PADDING and DB_MIN_DISPLAY to (height - LABEL_AREA_HEIGHT)
    return TOP_PADDING + graphHeight / 2 - (db / displayRange) * graphHeight;
}

/** Convert frequency to X coordinate (log scale) */
function freqToX(freq: number, width: number): number {
    return ((Math.log10(freq) - LOG_FREQ_MIN) / (LOG_FREQ_MAX - LOG_FREQ_MIN)) * width;
}

/** Draw the background and clear the canvas */
function drawBackground({ ctx, width, height, colors }: DrawContext): void {
    ctx.fillStyle = colors.background;
    ctx.fillRect(0, 0, width, height);
}

/** Draw horizontal dB grid lines and labels */
function drawDbGrid({ ctx, width, height, colors }: DrawContext): void {
    ctx.strokeStyle = colors.gridLine;
    ctx.lineWidth = GRID_LINE_WIDTH;
    ctx.fillStyle = colors.gridText;
    ctx.font = FONT_MAIN;

    for (let db = DB_MIN_DISPLAY; db <= DB_MAX_DISPLAY; db += DB_STEP) {
        const y = dbToY(db, height);

        // Draw grid line
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(width, y);
        ctx.stroke();

        // Draw label (skip 0dB, drawn separately)
        if (db !== 0) {
            const label = `${db > 0 ? "+" : ""}${db}dB`;
            ctx.fillText(label, LABEL_PADDING, y - LABEL_PADDING);
        }
    }
}

/** Draw vertical frequency band markers with labels */
function drawFrequencyMarkers({ ctx, width, height, colors }: DrawContext): void {
    ctx.font = FONT_MAIN;
    ctx.textAlign = "center";

    for (const band of AUDIOPHILE_BANDS) {
        const x = freqToX(band.freq, width);

        // Draw vertical line
        ctx.strokeStyle = colors.freqMarkerLine;
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height - LABEL_AREA_HEIGHT);
        ctx.stroke();

        // Draw band name label
        ctx.fillStyle = colors.freqMarkerLabel;
        ctx.fillText(band.label, x, height - 18);

        // Draw Hz label
        ctx.fillStyle = colors.freqMarkerHz;
        ctx.font = FONT_SMALL;
        ctx.fillText(band.hz, x, height - 6);
        ctx.font = FONT_MAIN;
    }

    ctx.textAlign = "left";
}

/** Draw the 0dB reference line */
function drawZeroLine({ ctx, width, height, colors }: DrawContext): void {
    const y = dbToY(0, height);

    ctx.strokeStyle = colors.zeroLine;
    ctx.lineWidth = GRID_LINE_WIDTH;
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(width, y);
    ctx.stroke();

    ctx.fillStyle = colors.zeroLineText;
    ctx.fillText("0dB", LABEL_PADDING, y - LABEL_PADDING);
}

/** Draw the frequency response curve */
function drawResponseCurve(
    { ctx, width, height, colors }: DrawContext,
    responseDb: Float32Array
): void {
    ctx.strokeStyle = colors.curve;
    ctx.lineWidth = CURVE_LINE_WIDTH;
    ctx.beginPath();

    for (let i = 0; i < NUM_POINTS; i++) {
        const x = (i / (NUM_POINTS - 1)) * width;
        const db = Math.max(DB_MIN_DISPLAY, Math.min(DB_MAX_DISPLAY, responseDb[i]));
        const y = dbToY(db, height);

        if (i === 0) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    }
    ctx.stroke();
}

/** Draw the gradient fill under the curve */
function drawCurveFill(
    { ctx, width, height, colors }: DrawContext
): void {
    // Complete the path to form a closed shape at 0dB line
    const zeroY = dbToY(0, height);
    ctx.lineTo(width, zeroY);
    ctx.lineTo(0, zeroY);
    ctx.closePath();

    // Create and apply gradient fill
    const gradient = ctx.createLinearGradient(0, 0, 0, height);
    gradient.addColorStop(0, colors.curveGradientTop);
    gradient.addColorStop(0.5, colors.curveGradientMid);
    gradient.addColorStop(1, colors.curveGradientBottom);
    ctx.fillStyle = gradient;
    ctx.fill();
}

// =============================================================================
// Hooks
// =============================================================================

/** Calculate frequency response for all enabled bands */
function useFrequencyResponse(
    bands: ParametricBand[],
    preamp: number,
    sampleRate: number
): Float32Array {
    return useMemo(() => {
        const response = new Float32Array(NUM_POINTS);

        for (let i = 0; i < NUM_POINTS; i++) {
            let totalDb = preamp;
            const freq = FREQUENCIES[i];

            for (const band of bands) {
                // Only include enabled bands in the frequency response
                if (band.enabled) {
                    totalDb += calcBiquadMagnitudeDb(
                        freq,
                        band.frequency,
                        band.gain,
                        band.q_factor,
                        band.filter_type,
                        sampleRate
                    );
                }
            }

            response[i] = totalDb;
        }

        return response;
    }, [bands, preamp, sampleRate]);
}

/** Observe container width changes */
function useContainerWidth(containerRef: React.RefObject<HTMLDivElement | null>): number {
    const [containerWidth, setContainerWidth] = useState(800);

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
    }, [containerRef]);

    return containerWidth;
}

// =============================================================================
// Component
// =============================================================================

export function EqGraph({
    bands,
    preamp,
    height = 220,
    sampleRate = 48000
}: EqGraphProps) {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const containerRef = useRef<HTMLDivElement>(null);
    const { resolvedTheme } = useTheme();

    const responseDb = useFrequencyResponse(bands, preamp, sampleRate);
    const containerWidth = useContainerWidth(containerRef);

    // Get colors based on current theme
    const colors = resolvedTheme === "dark" ? GRAPH_COLORS_DARK : GRAPH_COLORS_LIGHT;

    // Render to canvas when dependencies change
    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas || containerWidth <= 0) return;

        const ctx = canvas.getContext("2d");
        if (!ctx) return;

        const width = containerWidth;
        const dpr = window.devicePixelRatio || 1;

        // Configure canvas for high-DPI displays
        canvas.width = width * dpr;
        canvas.height = height * dpr;
        canvas.style.width = `${width}px`;
        canvas.style.height = `${height}px`;
        ctx.scale(dpr, dpr);

        // Create draw context object
        const drawCtx: DrawContext = { ctx, width, height, colors };

        // Draw all elements in order
        drawBackground(drawCtx);
        drawDbGrid(drawCtx);
        drawFrequencyMarkers(drawCtx);
        drawZeroLine(drawCtx);
        drawResponseCurve(drawCtx, responseDb);
        drawCurveFill(drawCtx);

    }, [responseDb, height, containerWidth, colors]);

    return (
        <div ref={containerRef} className="w-full">
            <canvas
                ref={canvasRef}
                className="rounded-lg border border-border w-full"
            />
        </div>
    );
}

// Audio mathematics utility functions

// Pre-compute log frequency points (20Hz - 20kHz) - FAST lookup
const NUM_POINTS = 200;
const LOG_FREQ_MIN = Math.log10(20);
const LOG_FREQ_MAX = Math.log10(20000);

export const FREQUENCIES: number[] = [];
for (let i = 0; i < NUM_POINTS; i++) {
    const logFreq = LOG_FREQ_MIN + (i / (NUM_POINTS - 1)) * (LOG_FREQ_MAX - LOG_FREQ_MIN);
    FREQUENCIES.push(Math.pow(10, logFreq));
}

export { NUM_POINTS, LOG_FREQ_MIN, LOG_FREQ_MAX };

/**
 * Normalizes filter type string to internal canonical types
 */
function normalizeFilterType(type: string): "peaking" | "lowshelf" | "highshelf" {
    const t = type.toLowerCase();
    if (t === "peaking" || t === "pk" || t === "peq") return "peaking";
    if (t === "lowshelf" || t === "ls" || t === "lsc" || t === "low shelf") return "lowshelf";
    if (t === "highshelf" || t === "hs" || t === "hsc" || t === "high shelf") return "highshelf";
    return "peaking"; // Default
}

/**
 * Calculates the magnitude response (in dB) of a biquad filter at a specific frequency
 * Strict RBJ / EqualizerAPO implementation
 */
export function calcBiquadMagnitudeDb(
    freq: number,
    fc: number,
    gainDb: number,
    q: number,
    filterTypeStr: string,
    sampleRate: number = 48000
): number {
    // 1. Clamp frequencies to Nyquist
    const safeFc = Math.max(1, Math.min(fc, sampleRate / 2 - 1));
    const safeFreq = Math.max(0, Math.min(freq, sampleRate / 2));

    const w0 = (2 * Math.PI * safeFc) / sampleRate;
    const w = (2 * Math.PI * safeFreq) / sampleRate;
    const A = Math.pow(10, gainDb / 40);
    const cosW0 = Math.cos(w0);
    const sinW0 = Math.sin(w0);
    const filterType = normalizeFilterType(filterTypeStr);

    let b0 = 0, b1 = 0, b2 = 0, a0 = 1, a1 = 0, a2 = 0;

    if (filterType === "peaking") {
        const alpha = sinW0 / (2 * q);
        b0 = 1 + alpha * A;
        b1 = -2 * cosW0;
        b2 = 1 - alpha * A;
        a0 = 1 + alpha / A;
        a1 = -2 * cosW0;
        a2 = 1 - alpha / A;
    } else {
        // Shelving filters use Slope (S) mapping
        // Q -> S mapping: S = 1 / (2 * Q^2)
        // Guard against Q=0 (division by zero)
        const safeQ = Math.max(0.0001, q);
        const S = 1 / (2 * safeQ * safeQ);

        // RBJ Shelf Alpha
        const alpha = (sinW0 / 2) * Math.sqrt((A + 1 / A) * (1 / S - 1) + 2);
        const sqrtA = Math.sqrt(A);

        if (filterType === "lowshelf") {
            b0 = A * ((A + 1) - (A - 1) * cosW0 + 2 * sqrtA * alpha);
            b1 = 2 * A * ((A - 1) - (A + 1) * cosW0);
            b2 = A * ((A + 1) - (A - 1) * cosW0 - 2 * sqrtA * alpha);
            a0 = (A + 1) + (A - 1) * cosW0 + 2 * sqrtA * alpha;
            a1 = -2 * ((A - 1) + (A + 1) * cosW0);
            a2 = (A + 1) + (A - 1) * cosW0 - 2 * sqrtA * alpha;
        } else { // highshelf
            b0 = A * ((A + 1) + (A - 1) * cosW0 + 2 * sqrtA * alpha);
            b1 = -2 * A * ((A - 1) + (A + 1) * cosW0);
            b2 = A * ((A + 1) + (A - 1) * cosW0 - 2 * sqrtA * alpha);
            a0 = (A + 1) - (A - 1) * cosW0 + 2 * sqrtA * alpha;
            a1 = 2 * ((A - 1) - (A + 1) * cosW0);
            a2 = (A + 1) - (A - 1) * cosW0 - 2 * sqrtA * alpha;
        }
    }

    // Normalize
    b0 /= a0; b1 /= a0; b2 /= a0; a1 /= a0; a2 /= a0;

    // Calculate magnitude at frequency w
    const cosW = Math.cos(w);
    const cos2W = Math.cos(2 * w);
    const sinW = Math.sin(w);
    const sin2W = Math.sin(2 * w);

    const numReal = b0 + b1 * cosW + b2 * cos2W;
    const numImag = -(b1 * sinW + b2 * sin2W);
    const denReal = 1 + a1 * cosW + a2 * cos2W;
    const denImag = -(a1 * sinW + a2 * sin2W);

    const magSquared = (numReal * numReal + numImag * numImag) / (denReal * denReal + denImag * denImag);

    // Safety check for invalid magnitude
    if (magSquared <= 0 || !Number.isFinite(magSquared)) return -100; // Return low dB floor

    return 10 * Math.log10(magSquared);
}

/**
 * Calculates the maximum peak gain across the frequency spectrum
 */
export function calculatePeakGain(bands: any[], preamp: number, sampleRate: number = 48000): number {
    let maxDb = -Infinity;

    for (const freq of FREQUENCIES) {
        let totalDb = preamp;
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
        if (totalDb > maxDb) maxDb = totalDb;
    }

    return maxDb;
}

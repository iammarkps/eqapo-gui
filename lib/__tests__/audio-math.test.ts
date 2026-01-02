import { describe, it, expect } from 'vitest'
import {
    calcBiquadMagnitudeDb,
    calculatePeakGain,
    FREQUENCIES,
    NUM_POINTS,
    LOG_FREQ_MIN,
    LOG_FREQ_MAX,
} from '../audio-math'

// =============================================================================
// Constants Tests
// =============================================================================

describe('Audio Math Constants', () => {
    it('should have correct number of frequency points', () => {
        expect(NUM_POINTS).toBe(200)
        expect(FREQUENCIES.length).toBe(NUM_POINTS)
    })

    it('should have frequency range from 20Hz to 20kHz', () => {
        expect(FREQUENCIES[0]).toBeCloseTo(20, 0)
        expect(FREQUENCIES[FREQUENCIES.length - 1]).toBeCloseTo(20000, 0)
    })

    it('should have logarithmically spaced frequencies', () => {
        // Check that frequencies increase logarithmically
        const logRatio1 = FREQUENCIES[50] / FREQUENCIES[0]
        const logRatio2 = FREQUENCIES[100] / FREQUENCIES[50]
        // The ratios should be approximately equal for log spacing
        expect(logRatio1).toBeCloseTo(logRatio2, 0)
    })

    it('should export correct log bounds', () => {
        expect(LOG_FREQ_MIN).toBeCloseTo(Math.log10(20), 10)
        expect(LOG_FREQ_MAX).toBeCloseTo(Math.log10(20000), 10)
    })
})

// =============================================================================
// Peaking Filter Tests
// =============================================================================

describe('calcBiquadMagnitudeDb - Peaking Filter', () => {
    const sampleRate = 48000

    it('should return approximately 0dB at all frequencies when gain is 0', () => {
        const result = calcBiquadMagnitudeDb(1000, 1000, 0, 1.0, 'peaking', sampleRate)
        expect(result).toBeCloseTo(0, 1)
    })

    it('should return approximately the gain value at center frequency', () => {
        const gain = 6
        const fc = 1000
        const result = calcBiquadMagnitudeDb(fc, fc, gain, 1.0, 'peaking', sampleRate)
        expect(result).toBeCloseTo(gain, 0)
    })

    it('should return nearly 0dB far from center frequency with high Q', () => {
        const result = calcBiquadMagnitudeDb(100, 10000, 12, 10, 'peaking', sampleRate)
        expect(Math.abs(result)).toBeLessThan(1)
    })

    it('should handle negative gain (cut)', () => {
        const gain = -6
        const fc = 1000
        const result = calcBiquadMagnitudeDb(fc, fc, gain, 1.0, 'peaking', sampleRate)
        expect(result).toBeCloseTo(gain, 0)
    })

    it('should accept filter type aliases', () => {
        const fc = 1000
        const gain = 6
        const q = 1.0

        const pkResult = calcBiquadMagnitudeDb(fc, fc, gain, q, 'pk', sampleRate)
        const peqResult = calcBiquadMagnitudeDb(fc, fc, gain, q, 'peq', sampleRate)
        const peakingResult = calcBiquadMagnitudeDb(fc, fc, gain, q, 'peaking', sampleRate)

        expect(pkResult).toBeCloseTo(peakingResult, 5)
        expect(peqResult).toBeCloseTo(peakingResult, 5)
    })

    it('should handle extreme Q values', () => {
        // Very narrow Q
        const narrowQ = calcBiquadMagnitudeDb(1000, 1000, 6, 20, 'peaking', sampleRate)
        expect(narrowQ).toBeCloseTo(6, 0)

        // Very wide Q
        const wideQ = calcBiquadMagnitudeDb(1000, 1000, 6, 0.1, 'peaking', sampleRate)
        expect(wideQ).toBeCloseTo(6, 0)
    })
})

// =============================================================================
// Low Shelf Filter Tests
// =============================================================================

describe('calcBiquadMagnitudeDb - Low Shelf Filter', () => {
    const sampleRate = 48000

    it('should return approximately 0dB when gain is 0', () => {
        const result = calcBiquadMagnitudeDb(100, 1000, 0, 0.707, 'lowshelf', sampleRate)
        expect(result).toBeCloseTo(0, 1)
    })

    it('should boost low frequencies', () => {
        const fc = 500
        const gain = 6
        // Well below corner frequency
        const lowResult = calcBiquadMagnitudeDb(50, fc, gain, 0.707, 'lowshelf', sampleRate)
        // Well above corner frequency
        const highResult = calcBiquadMagnitudeDb(5000, fc, gain, 0.707, 'lowshelf', sampleRate)

        expect(lowResult).toBeGreaterThan(highResult)
        expect(lowResult).toBeCloseTo(gain, 1)
        expect(Math.abs(highResult)).toBeLessThan(1)
    })

    it('should accept filter type aliases', () => {
        const lsResult = calcBiquadMagnitudeDb(100, 500, 6, 0.707, 'ls', sampleRate)
        const lowshelfResult = calcBiquadMagnitudeDb(100, 500, 6, 0.707, 'lowshelf', sampleRate)
        expect(lsResult).toBeCloseTo(lowshelfResult, 5)
    })
})

// =============================================================================
// High Shelf Filter Tests
// =============================================================================

describe('calcBiquadMagnitudeDb - High Shelf Filter', () => {
    const sampleRate = 48000

    it('should return approximately 0dB when gain is 0', () => {
        const result = calcBiquadMagnitudeDb(10000, 1000, 0, 0.707, 'highshelf', sampleRate)
        expect(result).toBeCloseTo(0, 1)
    })

    it('should boost high frequencies', () => {
        const fc = 2000
        const gain = 6
        // Well below corner frequency
        const lowResult = calcBiquadMagnitudeDb(100, fc, gain, 0.707, 'highshelf', sampleRate)
        // Well above corner frequency
        const highResult = calcBiquadMagnitudeDb(15000, fc, gain, 0.707, 'highshelf', sampleRate)

        expect(highResult).toBeGreaterThan(lowResult)
        expect(highResult).toBeCloseTo(gain, 1)
        expect(Math.abs(lowResult)).toBeLessThan(1)
    })

    it('should accept filter type aliases', () => {
        const hsResult = calcBiquadMagnitudeDb(10000, 2000, 6, 0.707, 'hs', sampleRate)
        const highshelfResult = calcBiquadMagnitudeDb(10000, 2000, 6, 0.707, 'highshelf', sampleRate)
        expect(hsResult).toBeCloseTo(highshelfResult, 5)
    })
})

// =============================================================================
// Edge Cases Tests
// =============================================================================

describe('calcBiquadMagnitudeDb - Edge Cases', () => {
    const sampleRate = 48000

    it('should handle frequency at Nyquist', () => {
        const result = calcBiquadMagnitudeDb(24000, 1000, 6, 1, 'peaking', sampleRate)
        expect(Number.isFinite(result)).toBe(true)
    })

    it('should handle very low frequency', () => {
        const result = calcBiquadMagnitudeDb(1, 1000, 6, 1, 'peaking', sampleRate)
        expect(Number.isFinite(result)).toBe(true)
    })

    it('should handle zero frequency', () => {
        const result = calcBiquadMagnitudeDb(0, 1000, 6, 1, 'peaking', sampleRate)
        expect(Number.isFinite(result)).toBe(true)
    })

    it('should default unknown filter types to peaking', () => {
        const unknownResult = calcBiquadMagnitudeDb(1000, 1000, 6, 1, 'unknown', sampleRate)
        const peakingResult = calcBiquadMagnitudeDb(1000, 1000, 6, 1, 'peaking', sampleRate)
        expect(unknownResult).toBeCloseTo(peakingResult, 5)
    })

    it('should handle different sample rates', () => {
        const result441 = calcBiquadMagnitudeDb(1000, 1000, 6, 1, 'peaking', 44100)
        const result48 = calcBiquadMagnitudeDb(1000, 1000, 6, 1, 'peaking', 48000)
        // Both should give approximately 6dB at center frequency
        expect(result441).toBeCloseTo(6, 0)
        expect(result48).toBeCloseTo(6, 0)
    })
})

// =============================================================================
// Peak Gain Calculation Tests
// =============================================================================

describe('calculatePeakGain', () => {
    it('should return preamp when no bands are present', () => {
        const result = calculatePeakGain([], 0)
        expect(result).toBe(0)
    })

    it('should return preamp value when bands have 0 gain', () => {
        const bands = [
            { frequency: 1000, gain: 0, q_factor: 1, filter_type: 'peaking' },
        ]
        const result = calculatePeakGain(bands, 3)
        expect(result).toBeCloseTo(3, 0)
    })

    it('should find peak from single band boost', () => {
        const bands = [
            { frequency: 1000, gain: 6, q_factor: 1, filter_type: 'peaking' },
        ]
        const result = calculatePeakGain(bands, 0)
        expect(result).toBeCloseTo(6, 0)
    })

    it('should combine preamp and band gain', () => {
        const bands = [
            { frequency: 1000, gain: 6, q_factor: 1, filter_type: 'peaking' },
        ]
        const preamp = 3
        const result = calculatePeakGain(bands, preamp)
        expect(result).toBeCloseTo(9, 0)
    })

    it('should find maximum across multiple bands', () => {
        const bands = [
            { frequency: 100, gain: 3, q_factor: 1, filter_type: 'peaking' },
            { frequency: 1000, gain: 9, q_factor: 1, filter_type: 'peaking' },
            { frequency: 10000, gain: 6, q_factor: 1, filter_type: 'peaking' },
        ]
        const result = calculatePeakGain(bands, 0)
        expect(result).toBeGreaterThanOrEqual(9)
    })

    it('should handle negative gains (cuts)', () => {
        const bands = [
            { frequency: 1000, gain: -6, q_factor: 1, filter_type: 'peaking' },
        ]
        const result = calculatePeakGain(bands, 0)
        // Peak should be close to 0 (unaffected frequencies)
        expect(result).toBeCloseTo(0, 0)
    })

    it('should handle shelf filters', () => {
        const bands = [
            { frequency: 100, gain: 6, q_factor: 0.707, filter_type: 'lowshelf' },
        ]
        const result = calculatePeakGain(bands, 0)
        expect(result).toBeCloseTo(6, 0)
    })
})

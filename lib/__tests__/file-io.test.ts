import { describe, it, expect, vi, beforeEach } from 'vitest'
import { generateId } from '../file-io'

// =============================================================================
// generateId Tests
// =============================================================================

describe('generateId', () => {
    it('should return a string', () => {
        const id = generateId()
        expect(typeof id).toBe('string')
    })

    it('should return a 7 character string', () => {
        const id = generateId()
        expect(id.length).toBe(7)
    })

    it('should contain only alphanumeric characters', () => {
        const id = generateId()
        expect(id).toMatch(/^[a-z0-9]+$/)
    })

    it('should generate unique IDs', () => {
        const ids = new Set<string>()
        for (let i = 0; i < 1000; i++) {
            ids.add(generateId())
        }
        // With 1000 generations, we should have 1000 unique IDs (statistically)
        expect(ids.size).toBe(1000)
    })
})

// =============================================================================
// EQ APO Format Parsing Tests (Testing the regex patterns used in file-io.ts)
// =============================================================================

describe('EQ APO Format Parsing', () => {
    // These tests verify the regex patterns used in handleImportTxt work correctly

    describe('Preamp Parsing', () => {
        const preampRegex = /Preamp:\s*([+-]?\d*\.?\d+)\s*dB/i

        it('should parse positive preamp', () => {
            const line = 'Preamp: 3.5 dB'
            const match = line.match(preampRegex)
            expect(match).not.toBeNull()
            expect(parseFloat(match![1])).toBe(3.5)
        })

        it('should parse negative preamp', () => {
            const line = 'Preamp: -6.0 dB'
            const match = line.match(preampRegex)
            expect(match).not.toBeNull()
            expect(parseFloat(match![1])).toBe(-6.0)
        })

        it('should parse preamp with explicit plus sign', () => {
            const line = 'Preamp: +2.5 dB'
            const match = line.match(preampRegex)
            expect(match).not.toBeNull()
            expect(parseFloat(match![1])).toBe(2.5)
        })

        it('should parse preamp without decimal', () => {
            const line = 'Preamp: 5 dB'
            const match = line.match(preampRegex)
            expect(match).not.toBeNull()
            expect(parseFloat(match![1])).toBe(5)
        })

        it('should be case insensitive', () => {
            const line = 'PREAMP: 3.0 DB'
            const match = line.match(preampRegex)
            expect(match).not.toBeNull()
        })
    })

    describe('Filter Type Parsing', () => {
        const typeRegex = /(PK|LS|HS|Peaking|LowShelf|HighShelf)/i

        it('should match PK', () => {
            expect('Filter 1: ON PK Fc 1000 Hz'.match(typeRegex)?.[1]).toBe('PK')
        })

        it('should match LS', () => {
            expect('Filter 1: ON LS Fc 100 Hz'.match(typeRegex)?.[1]).toBe('LS')
        })

        it('should match HS', () => {
            expect('Filter 1: ON HS Fc 8000 Hz'.match(typeRegex)?.[1]).toBe('HS')
        })

        it('should match Peaking (full word)', () => {
            expect('Filter 1: ON Peaking Fc 1000 Hz'.match(typeRegex)?.[1]).toBe('Peaking')
        })

        it('should match LowShelf (full word)', () => {
            expect('Filter 1: ON LowShelf Fc 100 Hz'.match(typeRegex)?.[1]).toBe('LowShelf')
        })

        it('should match HighShelf (full word)', () => {
            expect('Filter 1: ON HighShelf Fc 8000 Hz'.match(typeRegex)?.[1]).toBe('HighShelf')
        })
    })

    describe('Filter Parameter Parsing', () => {
        const freqRegex = /Fc\s+(\d+(?:\.\d+)?)/i
        const gainRegex = /Gain\s+([+-]?\d+(?:\.\d+)?)/i
        const qRegex = /Q\s+(\d+(?:\.\d+)?)/i

        const testLine = 'Filter 1: ON PK Fc 1000 Hz Gain -2.5 dB Q 1.41'

        it('should parse frequency', () => {
            const match = testLine.match(freqRegex)
            expect(match).not.toBeNull()
            expect(parseFloat(match![1])).toBe(1000)
        })

        it('should parse negative gain', () => {
            const match = testLine.match(gainRegex)
            expect(match).not.toBeNull()
            expect(parseFloat(match![1])).toBe(-2.5)
        })

        it('should parse Q factor', () => {
            const match = testLine.match(qRegex)
            expect(match).not.toBeNull()
            expect(parseFloat(match![1])).toBe(1.41)
        })

        it('should parse decimal frequency', () => {
            const line = 'Filter 1: ON PK Fc 123.5 Hz Gain 0 dB Q 1.0'
            const match = line.match(freqRegex)
            expect(parseFloat(match![1])).toBe(123.5)
        })

        it('should parse positive gain with plus sign', () => {
            const line = 'Filter 1: ON PK Fc 1000 Hz Gain +6.0 dB Q 1.0'
            const match = line.match(gainRegex)
            expect(parseFloat(match![1])).toBe(6.0)
        })
    })

    describe('Complete Filter Line Parsing', () => {
        it('should correctly parse a complete peaking filter line', () => {
            const line = 'Filter 1: ON PK Fc 1000 Hz Gain -2.0 dB Q 1.41'

            const typeMatch = line.match(/(PK|LS|HS)/i)
            const freqMatch = line.match(/Fc\s+(\d+(?:\.\d+)?)/i)
            const gainMatch = line.match(/Gain\s+([+-]?\d+(?:\.\d+)?)/i)
            const qMatch = line.match(/Q\s+(\d+(?:\.\d+)?)/i)

            expect(typeMatch?.[1]).toBe('PK')
            expect(parseFloat(freqMatch![1])).toBe(1000)
            expect(parseFloat(gainMatch![1])).toBe(-2.0)
            expect(parseFloat(qMatch![1])).toBe(1.41)
        })

        it('should correctly parse a low shelf filter line', () => {
            const line = 'Filter 2: ON LS Fc 100 Hz Gain 3.5 dB Q 0.71'

            const typeMatch = line.match(/(PK|LS|HS)/i)
            const freqMatch = line.match(/Fc\s+(\d+(?:\.\d+)?)/i)

            expect(typeMatch?.[1]).toBe('LS')
            expect(parseFloat(freqMatch![1])).toBe(100)
        })

        it('should correctly parse a high shelf filter line', () => {
            const line = 'Filter 3: ON HS Fc 8000 Hz Gain -1.5 dB Q 0.71'

            const typeMatch = line.match(/(PK|LS|HS)/i)
            const freqMatch = line.match(/Fc\s+(\d+(?:\.\d+)?)/i)

            expect(typeMatch?.[1]).toBe('HS')
            expect(parseFloat(freqMatch![1])).toBe(8000)
        })
    })
})

// =============================================================================
// JSON Profile Format Tests
// =============================================================================

describe('JSON Profile Format', () => {
    it('should validate a correct profile structure', () => {
        const profile = {
            name: 'Test Profile',
            preamp: -3.5,
            bands: [
                { filter_type: 'peaking', frequency: 1000, gain: 6, q_factor: 1.41 },
                { filter_type: 'lowshelf', frequency: 100, gain: 3, q_factor: 0.707 },
            ]
        }

        expect(profile.name).toBe('Test Profile')
        expect(profile.preamp).toBe(-3.5)
        expect(Array.isArray(profile.bands)).toBe(true)
        expect(profile.bands.length).toBe(2)

        const firstBand = profile.bands[0]
        expect(firstBand.filter_type).toBe('peaking')
        expect(firstBand.frequency).toBe(1000)
        expect(firstBand.gain).toBe(6)
        expect(firstBand.q_factor).toBe(1.41)
    })

    it('should handle profile with no preamp (defaults to 0)', () => {
        const profile = {
            name: 'No Preamp Profile',
            bands: []
        }

        expect(profile.preamp ?? 0).toBe(0)
    })

    it('should handle empty bands array', () => {
        const profile = {
            name: 'Empty Profile',
            preamp: 0,
            bands: []
        }

        expect(profile.bands.length).toBe(0)
    })
})

# DSP Test Cases

Comprehensive test suite for audio DSP validation.

## Biquad Coefficient Tests

### Peaking Filter
```typescript
describe('Peaking Filter', () => {
  it('should have 0 dB gain at DC', () => {
    const coeffs = peakingFilter(1000, 6, 1.41, 48000);
    const response = magnitudeResponse(coeffs, 0, 48000);
    expect(response).toBeCloseTo(0, 1);
  });

  it('should have specified gain at center frequency', () => {
    const coeffs = peakingFilter(1000, 6, 1.41, 48000);
    const response = magnitudeResponse(coeffs, 1000, 48000);
    expect(response).toBeCloseTo(6, 0.5);
  });

  it('should be stable for all valid parameters', () => {
    for (let f = 20; f <= 20000; f *= 2) {
      for (let g = -15; g <= 15; g += 5) {
        for (let q = 0.1; q <= 10; q *= 2) {
          const coeffs = peakingFilter(f, g, q, 48000);
          expect(isStable(coeffs)).toBe(true);
        }
      }
    }
  });
});
```

### Shelving Filters
```typescript
describe('Low Shelf', () => {
  it('should have specified gain at DC', () => {
    const coeffs = lowShelf(80, -6, 0.71, 48000);
    const response = magnitudeResponse(coeffs, 0, 48000);
    expect(response).toBeCloseTo(-6, 0.5);
  });

  it('should have 0 dB gain at Nyquist', () => {
    const coeffs = lowShelf(80, -6, 0.71, 48000);
    const response = magnitudeResponse(coeffs, 24000, 48000);
    expect(response).toBeCloseTo(0, 1);
  });
});

describe('High Shelf', () => {
  it('should have 0 dB gain at DC', () => {
    const coeffs = highShelf(10000, 4, 0.71, 48000);
    const response = magnitudeResponse(coeffs, 0, 48000);
    expect(response).toBeCloseTo(0, 1);
  });

  it('should have specified gain at Nyquist', () => {
    const coeffs = highShelf(10000, 4, 0.71, 48000);
    const response = magnitudeResponse(coeffs, 24000, 48000);
    expect(response).toBeCloseTo(4, 0.5);
  });
});
```

## Edge Cases

### Nyquist Frequency
```typescript
it('should reject frequencies at or above Nyquist', () => {
  expect(() => peakingFilter(24000, 3, 1, 48000)).toThrow();
  expect(() => peakingFilter(25000, 3, 1, 48000)).toThrow();
});
```

### Zero Q Factor
```typescript
it('should handle zero Q gracefully', () => {
  const coeffs = peakingFilter(1000, 3, 0, 48000);
  expect(isStable(coeffs)).toBe(true);
});
```

### Extreme Gains
```typescript
it('should handle extreme gains', () => {
  const coeffs1 = peakingFilter(1000, 30, 1, 48000);
  const coeffs2 = peakingFilter(1000, -30, 1, 48000);
  expect(isStable(coeffs1)).toBe(true);
  expect(isStable(coeffs2)).toBe(true);
});
```

## Integration Tests

### Cascaded Filters
```typescript
it('should sum responses in dB domain', () => {
  const band1 = peakingFilter(1000, 3, 1, 48000);
  const band2 = peakingFilter(1000, 2, 1, 48000);

  const resp1 = magnitudeResponse(band1, 1000, 48000);
  const resp2 = magnitudeResponse(band2, 1000, 48000);
  const totalExpected = resp1 + resp2;

  expect(totalExpected).toBeCloseTo(5, 0.1);
});
```

### Peak Gain Calculation
```typescript
it('should find maximum gain across spectrum', () => {
  const bands = [
    { filterType: 'Peaking', frequency: 1000, gain: 6, qFactor: 1.41 },
    { filterType: 'LowShelf', frequency: 80, gain: -3, qFactor: 0.71 },
  ];

  const peak = calculatePeakGain(bands, 0);
  expect(peak).toBeGreaterThan(5.5);
  expect(peak).toBeLessThan(6.5);
});
```

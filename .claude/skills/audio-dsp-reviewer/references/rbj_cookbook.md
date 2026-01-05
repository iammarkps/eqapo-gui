# RBJ Audio EQ Cookbook Reference

Complete reference for Robert Bristow-Johnson's Audio EQ Cookbook biquad filter formulas.

## Source

Original by Robert Bristow-Johnson: https://www.w3.org/TR/audio-eq-cookbook/

## Biquad Transfer Function

```
        b0 + b1*z^-1 + b2*z^-2
H(z) = ----------------------
        a0 + a1*z^-1 + a2*z^-2
```

## Common Parameters

```
Fs = sample rate (Hz)
f0 = center frequency (Hz)
dBgain = gain in decibels

Q = quality factor (bandwidth parameter)
BW = bandwidth in octaves

ω0 = 2*π*f0/Fs  (normalized frequency)
A = 10^(dBgain/40)  (amplitude for peaking/shelving)
```

## Alpha Calculations

For **Peaking and Shelving filters**:
```
α = sin(ω0)/(2*Q)
```

Alternative for bandwidth in octaves:
```
α = sin(ω0) * sinh(ln(2)/2 * BW * ω0/sin(ω0))
```

## Peaking Filter (Bell Curve EQ)

**Use case**: Boost or cut at a specific frequency

```
b0 =  1 + α*A
b1 = -2*cos(ω0)
b2 =  1 - α*A
a0 =  1 + α/A
a1 = -2*cos(ω0)
a2 =  1 - α/A
```

**Characteristics**:
- Center frequency: f0
- Gain at f0: dBgain
- Width controlled by Q (higher Q = narrower)
- Gain at DC and Nyquist: 0 dB

**TypeScript Implementation**:
```typescript
function peakingFilter(
  f0: number,
  dBgain: number,
  Q: number,
  Fs: number = 48000
): BiquadCoeffs {
  const A = Math.pow(10, dBgain / 40);
  const ω0 = (2 * Math.PI * f0) / Fs;
  const sinω0 = Math.sin(ω0);
  const cosω0 = Math.cos(ω0);
  const α = sinω0 / (2 * Q);

  return {
    b0: 1 + α * A,
    b1: -2 * cosω0,
    b2: 1 - α * A,
    a0: 1 + α / A,
    a1: -2 * cosω0,
    a2: 1 - α / A,
  };
}
```

## Low Shelf Filter

**Use case**: Bass control (boost/cut below f0)

```
b0 =    A*((A+1) - (A-1)*cos(ω0) + 2*sqrt(A)*α)
b1 =  2*A*((A-1) - (A+1)*cos(ω0))
b2 =    A*((A+1) - (A-1)*cos(ω0) - 2*sqrt(A)*α)
a0 =       (A+1) + (A-1)*cos(ω0) + 2*sqrt(A)*α
a1 =   -2*((A-1) + (A+1)*cos(ω0))
a2 =       (A+1) + (A-1)*cos(ω0) - 2*sqrt(A)*α
```

**Characteristics**:
- Transition frequency: f0
- Gain below f0: dBgain
- Slope controlled by Q
- Gain at Nyquist: 0 dB

**TypeScript Implementation**:
```typescript
function lowShelf(
  f0: number,
  dBgain: number,
  Q: number,
  Fs: number = 48000
): BiquadCoeffs {
  const A = Math.pow(10, dBgain / 40);
  const ω0 = (2 * Math.PI * f0) / Fs;
  const sinω0 = Math.sin(ω0);
  const cosω0 = Math.cos(ω0);
  const α = sinω0 / (2 * Q);
  const sqrtA = Math.sqrt(A);

  return {
    b0: A * ((A + 1) - (A - 1) * cosω0 + 2 * sqrtA * α),
    b1: 2 * A * ((A - 1) - (A + 1) * cosω0),
    b2: A * ((A + 1) - (A - 1) * cosω0 - 2 * sqrtA * α),
    a0: (A + 1) + (A - 1) * cosω0 + 2 * sqrtA * α,
    a1: -2 * ((A - 1) + (A + 1) * cosω0),
    a2: (A + 1) + (A - 1) * cosω0 - 2 * sqrtA * α,
  };
}
```

## High Shelf Filter

**Use case**: Treble control (boost/cut above f0)

```
b0 =    A*((A+1) + (A-1)*cos(ω0) + 2*sqrt(A)*α)
b1 = -2*A*((A-1) + (A+1)*cos(ω0))
b2 =    A*((A+1) + (A-1)*cos(ω0) - 2*sqrt(A)*α)
a0 =       (A+1) - (A-1)*cos(ω0) + 2*sqrt(A)*α
a1 =    2*((A-1) - (A+1)*cos(ω0))
a2 =       (A+1) - (A-1)*cos(ω0) - 2*sqrt(A)*α
```

**Characteristics**:
- Transition frequency: f0
- Gain above f0: dBgain
- Slope controlled by Q
- Gain at DC: 0 dB

**TypeScript Implementation**:
```typescript
function highShelf(
  f0: number,
  dBgain: number,
  Q: number,
  Fs: number = 48000
): BiquadCoeffs {
  const A = Math.pow(10, dBgain / 40);
  const ω0 = (2 * Math.PI * f0) / Fs;
  const sinω0 = Math.sin(ω0);
  const cosω0 = Math.cos(ω0);
  const α = sinω0 / (2 * Q);
  const sqrtA = Math.sqrt(A);

  return {
    b0: A * ((A + 1) + (A - 1) * cosω0 + 2 * sqrtA * α),
    b1: -2 * A * ((A - 1) + (A + 1) * cosω0),
    b2: A * ((A + 1) + (A - 1) * cosω0 - 2 * sqrtA * α),
    a0: (A + 1) - (A - 1) * cosω0 + 2 * sqrtA * α,
    a1: 2 * ((A - 1) - (A + 1) * cosω0),
    a2: (A + 1) - (A - 1) * cosω0 - 2 * sqrtA * α,
  };
}
```

## Frequency Response Calculation

### Magnitude Response

```typescript
function magnitudeResponse(
  coeffs: BiquadCoeffs,
  frequency: number,
  Fs: number = 48000
): number {
  const ω = (2 * Math.PI * frequency) / Fs;
  const cosω = Math.cos(ω);
  const sinω = Math.sin(ω);
  const cos2ω = Math.cos(2 * ω);
  const sin2ω = Math.sin(2 * ω);

  // Numerator (real and imaginary parts)
  const numRe = coeffs.b0 + coeffs.b1 * cosω + coeffs.b2 * cos2ω;
  const numIm = -coeffs.b1 * sinω - coeffs.b2 * sin2ω;
  const numMagSq = numRe * numRe + numIm * numIm;

  // Denominator (real and imaginary parts)
  const denRe = coeffs.a0 + coeffs.a1 * cosω + coeffs.a2 * cos2ω;
  const denIm = -coeffs.a1 * sinω - coeffs.a2 * sin2ω;
  const denMagSq = denRe * denRe + denIm * denIm;

  // |H(e^jω)|^2 = |numerator|^2 / |denominator|^2
  const magSq = numMagSq / denMagSq;

  // Convert to dB: 10*log10(mag^2) = 20*log10(mag)
  return 10 * Math.log10(magSq);
}
```

### Phase Response

```typescript
function phaseResponse(
  coeffs: BiquadCoeffs,
  frequency: number,
  Fs: number = 48000
): number {
  const ω = (2 * Math.PI * frequency) / Fs;
  const cosω = Math.cos(ω);
  const sinω = Math.sin(ω);
  const cos2ω = Math.cos(2 * ω);
  const sin2ω = Math.sin(2 * ω);

  // Numerator phase
  const numRe = coeffs.b0 + coeffs.b1 * cosω + coeffs.b2 * cos2ω;
  const numIm = -coeffs.b1 * sinω - coeffs.b2 * sin2ω;
  const numPhase = Math.atan2(numIm, numRe);

  // Denominator phase
  const denRe = coeffs.a0 + coeffs.a1 * cosω + coeffs.a2 * cos2ω;
  const denIm = -coeffs.a1 * sinω - coeffs.a2 * sin2ω;
  const denPhase = Math.atan2(denIm, denRe);

  // Total phase (in radians)
  return numPhase - denPhase;
}
```

## Filter Normalization

RBJ formulas assume a0 ≠ 1. For implementation, normalize:

```typescript
function normalize(coeffs: BiquadCoeffs): BiquadCoeffs {
  return {
    b0: coeffs.b0 / coeffs.a0,
    b1: coeffs.b1 / coeffs.a0,
    b2: coeffs.b2 / coeffs.a0,
    a0: 1.0,
    a1: coeffs.a1 / coeffs.a0,
    a2: coeffs.a2 / coeffs.a0,
  };
}
```

## Stability Conditions

Biquad filters are stable if poles are inside the unit circle:

```typescript
function isStable(coeffs: BiquadCoeffs): boolean {
  const a1 = coeffs.a1 / coeffs.a0;
  const a2 = coeffs.a2 / coeffs.a0;

  // Jury stability test
  return Math.abs(a2) < 1 && Math.abs(a1) < 1 + a2;
}
```

For RBJ filters with valid parameters, stability is guaranteed.

## Common Pitfalls

### 1. Wrong A Calculation

```typescript
// WRONG: 20 dB conversion (magnitude)
const A = Math.pow(10, dBgain / 20);

// CORRECT: 40 dB conversion (power)
const A = Math.pow(10, dBgain / 40);
```

RBJ uses power gain, not magnitude gain.

### 2. Nyquist Frequency

```typescript
// Check f0 < Nyquist
if (f0 >= Fs / 2) {
  throw new Error(`Frequency ${f0} Hz exceeds Nyquist (${Fs / 2} Hz)`);
}
```

### 3. Zero Q Factor

```typescript
// Prevent division by zero
const Q = Math.max(qFactor, 0.01);
```

## Parameter Ranges

**Typical values**:
- f0: 20 Hz to 20 kHz (< Nyquist)
- dBgain: -30 dB to +30 dB
- Q: 0.1 to 30
  - Q < 1: Wide filter
  - Q = 1.41 (√2): Butterworth response
  - Q > 10: Very narrow

## References

- [RBJ Audio EQ Cookbook](https://www.w3.org/TR/audio-eq-cookbook/)
- [Digital Signal Processing by Julius O. Smith III](https://ccrma.stanford.edu/~jos/filters/)
- [Biquad Calculator](https://www.earlevel.com/main/2013/10/13/biquad-calculator-v2/)

# Sample Size Calculator

Formulas for determining required trial counts in A/B testing.

## ABX Test Sample Size

### Formula

```
n = (Z_α/2 + Z_β)² * p(1-p) / (p - p0)²

Where:
- Z_α/2 = 1.96 (for α = 0.05, two-tailed)
- Z_β = 0.84 (for 80% power)
- p = expected accuracy
- p0 = 0.5 (null hypothesis: guessing)
```

### TypeScript Implementation

```typescript
function calculateSampleSize(
  expectedAccuracy: number,
  alpha: number = 0.05,
  power: number = 0.80
): number {
  const Za2 = 1.96; // For alpha = 0.05
  const Zb = 0.84;  // For power = 0.80

  const p = expectedAccuracy;
  const p0 = 0.5;

  const n = Math.pow(Za2 + Zb, 2) * p * (1 - p) / Math.pow(p - p0, 2);

  return Math.ceil(n);
}
```

### Sample Size Table

| Expected Accuracy | Alpha | Power | Required Trials |
|------------------|-------|-------|----------------|
| 55% | 0.05 | 0.80 | 783 |
| 60% | 0.05 | 0.80 | 196 |
| 65% | 0.05 | 0.80 | 87 |
| 70% | 0.05 | 0.80 | 49 |
| 75% | 0.05 | 0.80 | 31 |
| 80% | 0.05 | 0.80 | 21 |
| 85% | 0.05 | 0.80 | 15 |
| 90% | 0.05 | 0.80 | 11 |

## Preference Test Sample Size

For blind AB preference testing:

```
n = (Z_α/2)² * p(1-p) / E²

Where:
- Z_α/2 = 1.96 (95% confidence)
- p = expected proportion (default 0.5)
- E = margin of error (typically 0.1 = 10%)
```

### Example

```typescript
function preferenceTestSampleSize(marginOfError: number = 0.1): number {
  const z = 1.96;
  const p = 0.5; // Conservative assumption

  const n = Math.pow(z, 2) * p * (1 - p) / Math.pow(marginOfError, 2);

  return Math.ceil(n);
}

// Margin of error 10%: 96 trials
// Margin of error 5%: 385 trials
```

## Minimum Detectable Effect

What effect size can be detected with given sample size?

```
MDE = (Z_α/2 + Z_β) * √(2*p*(1-p)/n)

Where:
- n = sample size
- p = baseline rate (0.5 for AB tests)
```

### Example

```typescript
function minimumDetectableEffect(n: number): number {
  const Za2 = 1.96;
  const Zb = 0.84;
  const p = 0.5;

  const mde = (Za2 + Zb) * Math.sqrt(2 * p * (1 - p) / n);

  return mde;
}

// n=20: MDE = 0.31 (can detect 31% difference from 50%)
// n=50: MDE = 0.20 (can detect 20% difference)
// n=100: MDE = 0.14 (can detect 14% difference)
```

## Practical Recommendations

### Quick Reference

**For subtle differences (hard to hear)**:
- Expected accuracy: 55-60%
- Recommended trials: 50-100

**For moderate differences**:
- Expected accuracy: 65-75%
- Recommended trials: 20-40

**For obvious differences**:
- Expected accuracy: 80%+
- Recommended trials: 15-20

### Conservative Approach

When unsure, use:
- **Minimum**: 20 trials
- **Standard**: 30 trials
- **Rigorous**: 50+ trials

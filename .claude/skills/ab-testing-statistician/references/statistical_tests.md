# Statistical Tests for A/B Testing

Complete guide to statistical methods used in blind audio testing.

## Binomial Test

Tests if observed proportion differs significantly from expected (null hypothesis).

**Use Case**: ABX testing - are correct answers better than 50% guessing?

### Formula

```
P(X ≥ k) = Σ(i=k to n) C(n,i) * p^i * (1-p)^(n-i)

Where:
- n = number of trials
- k = number of successes
- p = probability under null hypothesis (0.5 for guessing)
- C(n,i) = binomial coefficient "n choose i"
```

### TypeScript Implementation

```typescript
function binomialTest(successes: number, trials: number, p: number = 0.5): number {
  // Two-tailed binomial test
  const pmf = (k: number) => {
    return binomialCoeff(trials, k) * Math.pow(p, k) * Math.pow(1 - p, trials - k);
  };

  const observedP = pmf(successes);
  let pValue = observedP;

  for (let k = 0; k <= trials; k++) {
    const kP = pmf(k);
    if (kP <= observedP && k !== successes) {
      pValue += kP;
    }
  }

  return Math.min(pValue, 1.0);
}
```

### Interpretation

| P-Value | Interpretation |
|---------|----------------|
| < 0.001 | Extremely significant (***) |
| < 0.01  | Very significant (**) |
| < 0.05  | Significant (*) |
| ≥ 0.05  | Not significant (ns) |

**Example**:
```
ABX Test: 18/20 correct
P-value = 0.0004
Conclusion: User can reliably hear the difference (p < 0.001)
```

## Chi-Square Test

Tests if observed distribution differs from expected.

**Use Case**: Blind AB preference testing with unequal preferences.

### Formula

```
χ² = Σ (O - E)² / E

Where:
- O = observed frequency
- E = expected frequency
- df = degrees of freedom (k - 1, where k = categories)
```

### Example

```
Blind AB Test (30 trials):
A selected: 20 times
B selected: 10 times

Expected: 15/15 (50/50)

χ² = (20-15)²/15 + (10-15)²/15
χ² = 25/15 + 25/15
χ² = 3.33

df = 1
p-value ≈ 0.068 (not significant)
```

## Effect Size (Cohen's h)

Measures practical significance (not just statistical).

### Formula

```
h = 2 * (arcsin(√p1) - arcsin(√p2))

Where:
- p1 = proportion in group 1
- p2 = proportion in group 2
```

### Interpretation

| |h| | Effect Size |
|-----|-------------|
| 0.2 | Small |
| 0.5 | Medium |
| 0.8 | Large |

## Confidence Intervals

Estimate range where true value likely falls.

### Wilson Score Interval

Better than normal approximation for small samples.

```typescript
function wilsonInterval(successes: number, trials: number, confidence: number = 0.95): [number, number] {
  const z = 1.96; // 95% confidence
  const p = successes / trials;
  const n = trials;

  const center = (p + z * z / (2 * n)) / (1 + z * z / n);
  const margin = (z / (1 + z * z / n)) * Math.sqrt((p * (1 - p) + z * z / (4 * n)) / n);

  return [center - margin, center + margin];
}
```

**Example**:
```
ABX: 15/20 correct (75%)
95% CI: [53.3%, 89.4%]
Interpretation: True accuracy likely between 53% and 89%
```

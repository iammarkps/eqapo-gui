# Experimental Design Best Practices

Guidelines for designing valid blind audio comparison tests.

## Randomization

### Per-Trial Randomization (Correct)

```rust
let hidden_mapping: Vec<bool> = (0..trials)
    .map(|_| rng.gen_bool(0.5))
    .collect();
```

**Why**: Each trial is independent, prevents patterns.

### Block Randomization (Wrong for A/B)

```rust
// DON'T DO THIS
let mapping = if rng.gen_bool(0.5) { true } else { false };
// Use same mapping for all trials
```

**Why**: Not truly blind - user could detect pattern.

## Counterbalancing

Ensure equal A and B presentations.

### Check Balance

```rust
fn check_balance(mapping: &[bool]) -> f64 {
    let a_count = mapping.iter().filter(|&&x| x).count();
    let ratio = a_count as f64 / mapping.len() as f64;
    (ratio - 0.5).abs()
}
```

**Acceptable**: < 0.15 deviation (45%-55% split)

### Force Balance

```rust
fn balanced_mapping(trials: usize) -> Vec<bool> {
    let mut mapping = vec![true; trials / 2];
    mapping.extend(vec![false; trials / 2]);
    mapping.shuffle(&mut thread_rng());
    mapping
}
```

## Loudness Matching

**Critical**: Louder = perceived as better (even 0.5 dB difference).

### Auto-Match Algorithm

```rust
fn calculate_trim(bands_a: &[Band], preamp_a: f32, bands_b: &[Band], preamp_b: f32) -> f32 {
    let peak_a = calculate_peak_gain(bands_a, preamp_a);
    let peak_b = calculate_peak_gain(bands_b, preamp_b);
    peak_a - peak_b // Adjust B to match A
}
```

### Manual Trim

Allow ±1 dB adjustment for user preference.

## Sample Size

### Power Analysis

**Goal**: 80% power to detect medium effect (d = 0.5)

```
n = 2 * (Z_α/2 + Z_β)² * σ² / δ²

Where:
- Z_α/2 = 1.96 (α = 0.05, two-tailed)
- Z_β = 0.84 (power = 0.80)
- δ = expected difference
```

### Recommended Minimums

| Test Type | Expected Accuracy | Min Trials |
|-----------|------------------|------------|
| ABX (subtle) | 60% | 50 |
| ABX (moderate) | 70% | 30 |
| ABX (obvious) | 80% | 20 |
| Blind AB | Any | 20-30 |

## Trial Independence

### Good Practices

- ✅ Randomize each trial independently
- ✅ Allow unlimited switching between options
- ✅ No time pressure
- ✅ Rest breaks every 10 trials

### Bad Practices

- ❌ Fixed patterns (ABABABAB)
- ❌ Limited playback time
- ❌ Showing previous answers
- ❌ No breaks (listener fatigue)

## Validity Checks

### Attention Checks

Include 1-2 "easy" trials (large difference) to verify attention.

### Practice Trials

2-3 practice trials (not counted) to familiarize user.

### Time Limits

- Min per trial: 10 seconds
- Max session: 30 minutes
- Recommend breaks: Every 10 trials

## Data Quality

### Exclusion Criteria

Exclude sessions if:
- < 50% completion
- Too fast (< 5 sec/trial average)
- Failed attention checks
- Self-reported fatigue/distraction

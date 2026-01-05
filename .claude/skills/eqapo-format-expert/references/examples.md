# EqualizerAPO Configuration Examples

Real-world configuration file examples.

## Example 1: Headphone Compensation (HD 650)

```
# Sennheiser HD 650 Harman Target
# Generated from AutoEQ

Preamp: -6.7 dB
Filter: ON PK Fc 20 Hz Gain 6.5 dB Q 0.59
Filter: ON PK Fc 150 Hz Gain -2.3 dB Q 0.71
Filter: ON PK Fc 3200 Hz Gain 3.4 dB Q 2.15
Filter: ON PK Fc 4900 Hz Gain -2.1 dB Q 4.31
Filter: ON PK Fc 8000 Hz Gain 4.0 dB Q 4.45
```

## Example 2: V-Shaped Signature

```
# Bass and Treble boost for fun listening

Preamp: -4.0 dB
Filter: ON LS Fc 100 Hz Gain 4.0 dB Q 0.71
Filter: ON PK Fc 60 Hz Gain 3.0 dB Q 1.0
Filter: ON HS Fc 8000 Hz Gain 4.0 dB Q 0.71
Filter: ON PK Fc 12000 Hz Gain 2.0 dB Q 2.0
```

## Example 3: Vocal Clarity Enhancement

```
# Enhance vocal presence

Preamp: -3.0 dB
Filter: ON PK Fc 200 Hz Gain -2.0 dB Q 1.0
Filter: ON PK Fc 3000 Hz Gain 3.0 dB Q 1.41
Filter: ON PK Fc 5000 Hz Gain 2.0 dB Q 2.0
```

## Example 4: Multi-Channel Setup

```
# Different EQ for left and right

Channel: L
Filter: ON PK Fc 1000 Hz Gain 3.0 dB Q 1.41

Channel: R
Filter: ON PK Fc 1000 Hz Gain 2.0 dB Q 1.41

Channel: ALL
Preamp: -3.0 dB
```

## Example 5: Include Files

```
# Main config
Include: bass_boost.txt
Include: treble_smooth.txt

Preamp: -5.0 dB
```

**bass_boost.txt**:
```
Filter: ON LS Fc 80 Hz Gain 4.0 dB Q 0.71
Filter: ON PK Fc 60 Hz Gain 2.0 dB Q 1.0
```

**treble_smooth.txt**:
```
Filter: ON PK Fc 8000 Hz Gain -2.0 dB Q 2.0
Filter: ON HS Fc 12000 Hz Gain -1.0 dB Q 0.71
```

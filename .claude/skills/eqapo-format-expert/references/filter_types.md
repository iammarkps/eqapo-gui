# EqualizerAPO Filter Types

Complete reference for all EAPO filter types and their syntax.

## Peaking (PK)

**Syntax**: `Filter: ON PK Fc <freq> Hz Gain <gain> dB Q <q>`

**Use**: Boost or cut at specific frequency

**Examples**:
```
Filter: ON PK Fc 1000 Hz Gain 3.0 dB Q 1.41
Filter: ON PK Fc 100 Hz Gain -6 dB Q 0.7
```

## Low Shelf (LS)

**Syntax**: `Filter: ON LS Fc <freq> Hz Gain <gain> dB Q <q>`

**Use**: Bass adjustment

**Examples**:
```
Filter: ON LS Fc 80 Hz Gain -3.0 dB Q 0.71
Filter: ON LS Fc 200 Hz Gain 6 dB Q 1.0
```

## High Shelf (HS)

**Syntax**: `Filter: ON HS Fc <freq> Hz Gain <gain> dB Q <q>`

**Use**: Treble adjustment

**Examples**:
```
Filter: ON HS Fc 10000 Hz Gain 4.0 dB Q 0.71
Filter: ON HS Fc 8000 Hz Gain -2 dB Q 1.0
```

## Low Pass (LP/LPQ)

**Syntax**: `Filter: ON LP Fc <freq> Hz`

**Use**: Remove high frequencies

**Examples**:
```
Filter: ON LP Fc 20000 Hz
Filter: ON LPQ Fc 15000 Hz Q 0.707
```

## High Pass (HP/HPQ)

**Syntax**: `Filter: ON HP Fc <freq> Hz`

**Use**: Remove low frequencies

**Examples**:
```
Filter: ON HP Fc 20 Hz
Filter: ON HPQ Fc 30 Hz Q 0.707
```

## Notch (NO)

**Syntax**: `Filter: ON NO Fc <freq> Hz Q <q>`

**Use**: Remove specific frequency (hum, resonance)

**Examples**:
```
Filter: ON NO Fc 60 Hz Q 10
Filter: ON NO Fc 1000 Hz Q 20
```

## Band Pass (BP)

**Syntax**: `Filter: ON BP Fc <freq> Hz BW <octaves> Oct`

**Use**: Pass only specific frequency range

**Examples**:
```
Filter: ON BP Fc 1000 Hz BW 1 Oct
```

# EqualizerAPO Configuration File Specification

Complete specification for EqualizerAPO config files.

## File Format

- **Type**: Plain text (UTF-8)
- **Extension**: `.txt`
- **Location**: `C:\Program Files\EqualizerAPO\config\config.txt`
- **Encoding**: UTF-8 (no BOM)
- **Line endings**: Windows (CRLF) or Unix (LF)

## Basic Syntax

### Comments
```
# This is a comment
// This is also a comment (C++ style)
```

### Preamp
```
Preamp: <value> dB
```

### Filter
```
Filter: <ON|OFF> <TYPE> Fc <frequency> Hz Gain <gain> dB Q <q_factor>
```

## Filter Types

| Code | Name | Parameters |
|------|------|------------|
| PK   | Peaking | Fc, Gain, Q |
| LS   | Low Shelf | Fc, Gain, Q |
| HS   | High Shelf | Fc, Gain, Q |
| LP   | Low Pass | Fc, Q |
| HP   | High Pass | Fc, Q |
| BP   | Band Pass | Fc, BW |
| NO   | Notch | Fc, Q |
| AP   | All Pass | Fc, Q |
| LPQ  | Low Pass (Q) | Fc, Q |
| HPQ  | High Pass (Q) | Fc, Q |

## Channel Selection

```
Channel: <L|R|C|SUB|SL|SR|RL|RR|ALL>
Filter: ...
Filter: ...

Channel: ALL
```

## Advanced Features

### Include Directive
```
Include: C:\Path\To\Another\Config.txt
Include: subwoofer_eq.txt
```

### Copy (Channel Mixing)
```
Copy: L=L+0.5*R R=R+0.5*L
```

### Delay
```
Delay: L=0 R=2.5
```

### Convolution (Impulse Response)
```
Convolution: room_correction.wav
```

### Device Selection
```
Device: Speakers (Realtek Audio)
```

## Parameter Limits

- **Frequency**: 0.1 Hz to Nyquist (Fs/2)
- **Gain**: -100 dB to +100 dB (practical: ±30 dB)
- **Q**: 0.001 to 1000 (practical: 0.1-30)
- **Preamp**: -100 dB to +100 dB

## Case Sensitivity

**Case-insensitive**: Filter keywords, ON/OFF, channel names
**Case-sensitive**: File paths in Include directives

## Examples

### Basic EQ
```
Preamp: -5.0 dB
Filter: ON PK Fc 1000 Hz Gain 3.0 dB Q 1.41
Filter: ON LS Fc 80 Hz Gain -2.5 dB Q 0.71
Filter: ON HS Fc 10000 Hz Gain 4.0 dB Q 0.71
```

### Disabled Filter
```
Filter: OFF PK Fc 2000 Hz Gain 6.0 dB Q 1.0
```

### Multi-Channel
```
Channel: L
Filter: ON PK Fc 1000 Hz Gain 3.0 dB Q 1.41

Channel: R
Filter: ON PK Fc 1000 Hz Gain 2.0 dB Q 1.41

Channel: ALL
```

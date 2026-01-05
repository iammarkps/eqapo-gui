# Filter Type Reference

Complete listing of filter types supported in audio EQ applications.

## Implemented in EQAPO GUI

### 1. Peaking (PK / Bell)
- **Purpose**: Boost or cut at specific frequency
- **Parameters**: Fc (center), Gain, Q
- **Use cases**: Resonance control, tone shaping
- **Q range**: 0.1-30 (typical: 0.7-2.0)

### 2. Low Shelf (LS)
- **Purpose**: Bass boost/cut
- **Parameters**: Fc (transition), Gain, Q  
- **Use cases**: Bass adjustment, low-end correction
- **Typical Fc**: 60-250 Hz

### 3. High Shelf (HS)
- **Purpose**: Treble boost/cut
- **Parameters**: Fc (transition), Gain, Q
- **Use cases**: Treble adjustment, air boost
- **Typical Fc**: 8-16 kHz

## Future Filter Types

### 4. Low Pass (LP)
- **Purpose**: Remove high frequencies
- **Parameters**: Fc (cutoff), Q (slope)
- **Use cases**: Sub-bass isolation, rumble removal

### 5. High Pass (HP)
- **Purpose**: Remove low frequencies  
- **Parameters**: Fc (cutoff), Q (slope)
- **Use cases**: Sub-sonic filtering, DC removal

### 6. Band Pass (BP)
- **Purpose**: Pass only a frequency range
- **Parameters**: Fc (center), BW (bandwidth)
- **Use cases**: Isolating specific ranges

### 7. Notch / Band Stop (NO)
- **Purpose**: Remove specific frequency
- **Parameters**: Fc (center), Q (width)
- **Use cases**: Hum removal, resonance suppression

### 8. All Pass (AP)
- **Purpose**: Phase adjustment only
- **Parameters**: Fc, Q
- **Use cases**: Phase alignment, crossover design

## Parameter Guidelines

| Filter Type | Typical Fc Range | Gain Range | Q Range |
|-------------|------------------|------------|---------|
| Peaking     | 20 Hz - 20 kHz  | ±15 dB     | 0.3-10  |
| Low Shelf   | 60-250 Hz       | ±12 dB     | 0.5-1.5 |
| High Shelf  | 8-16 kHz        | ±12 dB     | 0.5-1.5 |
| Low Pass    | 50 Hz - 20 kHz  | N/A        | 0.5-2.0 |
| High Pass   | 20-200 Hz       | N/A        | 0.5-2.0 |
| Notch       | 50 Hz - 10 kHz  | N/A        | 5-50    |

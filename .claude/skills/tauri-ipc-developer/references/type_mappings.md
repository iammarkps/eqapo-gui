# Rust ↔ TypeScript Type Mappings

Complete reference for type conversions between Rust (Serde JSON) and TypeScript in Tauri IPC.

## Primitive Types

| Rust | TypeScript | Notes |
|------|-----------|-------|
| `bool` | `boolean` | Direct mapping |
| `i8`, `i16`, `i32`, `i64` | `number` | JS safe range: ±2^53 |
| `u8`, `u16`, `u32`, `u64` | `number` | May overflow, use `i64` for large values |
| `f32`, `f64` | `number` | Direct mapping, NaN/Infinity supported |
| `String` | `string` | UTF-8 encoded |
| `char` | `string` | Single character string |
| `()` (unit) | `null` or `void` | Depends on context |

## Collections

| Rust | TypeScript | Example |
|------|-----------|---------|
| `Vec<T>` | `T[]` | `Vec<String>` → `string[]` |
| `[T; N]` | `T[]` | `[f32; 3]` → `number[]` (length not preserved) |
| `HashMap<K, V>` | `Record<K, V>` | `HashMap<String, i32>` → `Record<string, number>` |
| `BTreeMap<K, V>` | `Record<K, V>` | Same as HashMap in JSON |
| `HashSet<T>` | `T[]` | `HashSet<String>` → `string[]` |

## Option & Result

| Rust | TypeScript | Serialization |
|------|-----------|---------------|
| `Option<T>` | `T \| null` | `Some(x)` → `x`, `None` → `null` |
| `Result<T, E>` | `Promise<T>` | `Ok(x)` → resolve, `Err(e)` → reject with string |

**Example:**
```rust
#[command]
async fn get_profile(name: String) -> Result<EqProfile, String> { ... }
```
```typescript
async function getProfile(name: string): Promise<EqProfile> { ... }
```

## Structs

### Basic Struct

**Rust:**
```rust
#[derive(Serialize, Deserialize)]
pub struct ParametricBand {
    pub frequency: f32,
    pub gain: f32,
    pub q_factor: f32,
}
```

**TypeScript:**
```typescript
interface ParametricBand {
  frequency: number;
  gain: number;
  q_factor: number;
}
```

### CamelCase Conversion

**Rust:**
```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParametricBand {
    pub filter_type: String,
    pub q_factor: f32,
}
```

**TypeScript:**
```typescript
interface ParametricBand {
  filterType: string;
  qFactor: number;
}
```

## Enums

### Simple Enum (String Variants)

**Rust:**
```rust
#[derive(Serialize, Deserialize)]
pub enum FilterType {
    Peaking,
    LowShelf,
    HighShelf,
}
```

**TypeScript:**
```typescript
type FilterType = 'Peaking' | 'LowShelf' | 'HighShelf';
```

### Tagged Enum (Variants with Data)

**Rust:**
```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    ProfileSaved { name: String },
    ErrorOccurred { message: String },
}
```

**TypeScript:**
```typescript
type Event =
  | { type: 'ProfileSaved'; name: string }
  | { type: 'ErrorOccurred'; message: string };
```

### Untagged Enum

**Rust:**
```rust
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Integer(i32),
    Float(f64),
    Text(String),
}
```

**TypeScript:**
```typescript
type Value = number | string;
```

## Special Cases

### Tuples

**Rust:**
```rust
(String, i32, bool)
```

**TypeScript:**
```typescript
[string, number, boolean]
```

### Nested Structures

**Rust:**
```rust
#[derive(Serialize, Deserialize)]
pub struct EqProfile {
    pub name: String,
    pub bands: Vec<ParametricBand>,
    pub metadata: Option<HashMap<String, String>>,
}
```

**TypeScript:**
```typescript
interface EqProfile {
  name: string;
  bands: ParametricBand[];
  metadata: Record<string, string> | null;
}
```

## Date/Time

| Rust | TypeScript | Format |
|------|-----------|--------|
| `chrono::DateTime<Utc>` | `string` | ISO 8601: `"2026-01-04T12:00:00Z"` |
| `std::time::SystemTime` | `number` | Unix timestamp (seconds) |

**Example:**
```rust
use chrono::{DateTime, Utc};

#[derive(Serialize)]
pub struct Metadata {
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
}
```

## Path Types

| Rust | TypeScript | Recommendation |
|------|-----------|----------------|
| `std::path::PathBuf` | `string` | Use `PathBuf::to_string_lossy()` |
| `&std::path::Path` | `string` | Convert to string before serializing |

**Never serialize `Path` directly - convert to `String` first.**

## Non-Serializable Types

These types **cannot** be sent over IPC:

- Function pointers / Closures
- Trait objects (`dyn Trait`)
- References (`&T`, `&mut T`)
- Raw pointers (`*const T`, `*mut T`)
- File handles (`std::fs::File`)
- Thread handles (`std::thread::JoinHandle`)
- Mutexes, RwLocks (send ID instead)

## Type Generation Tools

### ts-rs (Recommended)

Automatically generate TypeScript types from Rust:

```rust
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ParametricBand {
    pub frequency: f32,
    pub gain: f32,
}
```

Generates `bindings/ParametricBand.ts`:
```typescript
export interface ParametricBand {
  frequency: number;
  gain: number;
}
```

### Manual Maintenance

If not using `ts-rs`, maintain types in parallel:

1. Define Rust type in `src-tauri/src/types.rs`
2. Mirror in `lib/types.ts`
3. Add a comment linking the files:

```rust
// Mirror this in lib/types.ts
#[derive(Serialize, Deserialize)]
pub struct ParametricBand { ... }
```

```typescript
// Mirror of src-tauri/src/types.rs::ParametricBand
export interface ParametricBand { ... }
```

## Validation

### Runtime Type Checking (TypeScript)

Use Zod for runtime validation:

```typescript
import { z } from 'zod';

const ParametricBandSchema = z.object({
  frequency: z.number().min(20).max(20000),
  gain: z.number().min(-15).max(15),
  qFactor: z.number().min(0.1).max(30),
});

const data = await invoke('get_band', { id: 0 });
const band = ParametricBandSchema.parse(data); // Throws if invalid
```

### Compile-Time Checking (Rust)

Use newtypes for domain validation:

```rust
#[derive(Serialize, Deserialize)]
pub struct Frequency(f32);

impl Frequency {
    pub fn new(hz: f32) -> Result<Self, &'static str> {
        if (20.0..=20000.0).contains(&hz) {
            Ok(Frequency(hz))
        } else {
            Err("Frequency out of range")
        }
    }
}
```

## Common Pitfalls

1. **Forgetting `#[serde(rename_all = "camelCase")]`**
   - Rust: `q_factor`
   - TypeScript: `q_factor` (should be `qFactor`)

2. **Integer Overflow**
   - Rust `u64::MAX` doesn't fit in JS number
   - Use `i64` or `String` for large numbers

3. **NaN Handling**
   - Rust `f32::NAN` serializes as `null` in JSON
   - Check for `NaN` before serializing

4. **PathBuf on Windows**
   - Backslashes in paths: `C:\Users\...`
   - Use `to_string_lossy()` for cross-platform

5. **Missing Derives**
   - Forgot `#[derive(Serialize)]` → serialization fails
   - Forgot `#[derive(Deserialize)]` → deserialization fails

## Testing Type Compatibility

Create integration tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization() {
        let band = ParametricBand {
            frequency: 1000.0,
            gain: 3.0,
            q_factor: 1.41,
        };
        let json = serde_json::to_string(&band).unwrap();
        assert_eq!(json, r#"{"frequency":1000.0,"gain":3.0,"q_factor":1.41}"#);
    }
}
```

```typescript
import { describe, it, expect } from 'vitest';

describe('Type Compatibility', () => {
  it('should match Rust serialization', () => {
    const band: ParametricBand = {
      frequency: 1000,
      gain: 3.0,
      qFactor: 1.41,
    };
    expect(JSON.stringify(band)).toContain('frequency');
  });
});
```

# Common Tauri IPC Command Patterns

Proven patterns for implementing Tauri commands in EQAPO GUI.

## Pattern 1: Simple Query (Get Data)

**Use Case:** Retrieve application state without side effects.

**Rust:**
```rust
#[command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    state
        .settings
        .lock()
        .map_err(|e| format!("Lock error: {}", e))
        .map(|settings| settings.clone())
}
```

**TypeScript:**
```typescript
export async function getSettings(): Promise<Settings> {
  return await invoke<Settings>('get_settings');
}
```

**React Hook:**
```typescript
export function useSettings() {
  const [settings, setSettings] = useState<Settings | null>(null);

  useEffect(() => {
    getSettings().then(setSettings);
  }, []);

  return settings;
}
```

---

## Pattern 2: Simple Command (Mutate State)

**Use Case:** Update application state with validation.

**Rust:**
```rust
#[command]
pub fn update_preamp(
    preamp: f32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if !(-20.0..=20.0).contains(&preamp) {
        return Err("Preamp must be between -20 and +20 dB".to_string());
    }

    state
        .settings
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?
        .preamp = preamp;

    Ok(())
}
```

**TypeScript:**
```typescript
export async function updatePreamp(preamp: number): Promise<void> {
  return await invoke('update_preamp', { preamp });
}
```

---

## Pattern 3: Async I/O Command

**Use Case:** File operations, network requests.

**Rust:**
```rust
use tokio::fs;

#[command]
pub async fn load_profile(name: String) -> Result<EqProfile, String> {
    let profile_path = get_profile_path(&name)?;

    let contents = fs::read_to_string(&profile_path)
        .await
        .map_err(|e| format!("Failed to read profile: {}", e))?;

    serde_json::from_str(&contents)
        .map_err(|e| format!("Invalid profile format: {}", e))
}

fn get_profile_path(name: &str) -> Result<PathBuf, String> {
    let mut path = dirs::document_dir()
        .ok_or("Could not find documents directory")?;
    path.push("EQAPO GUI");
    path.push("profiles");
    path.push(format!("{}.json", name));
    Ok(path)
}
```

**TypeScript:**
```typescript
export async function loadProfile(name: string): Promise<EqProfile> {
  return await invoke<EqProfile>('load_profile', { name });
}
```

---

## Pattern 4: Batch Update

**Use Case:** Update multiple related settings atomically.

**Rust:**
```rust
#[derive(Deserialize)]
pub struct ProfileUpdate {
    pub bands: Vec<ParametricBand>,
    pub preamp: f32,
    pub apply_to_eapo: bool,
}

#[command]
pub async fn apply_profile(update: ProfileUpdate) -> Result<(), String> {
    // Validate all bands first
    for band in &update.bands {
        validate_band(band)?;
    }

    // Apply atomically
    if update.apply_to_eapo {
        write_eapo_config(&update.bands, update.preamp).await?;
    }

    // Update state last (after successful write)
    update_app_state(update)?;

    Ok(())
}
```

**TypeScript:**
```typescript
export async function applyProfile(
  bands: ParametricBand[],
  preamp: number,
  applyToEapo: boolean = true
): Promise<void> {
  return await invoke('apply_profile', {
    update: { bands, preamp, applyToEapo },
  });
}
```

---

## Pattern 5: List/Search Operations

**Use Case:** Query collections with filtering.

**Rust:**
```rust
#[command]
pub async fn list_profiles(search: Option<String>) -> Result<Vec<String>, String> {
    let profile_dir = get_profile_dir()?;

    let mut entries = fs::read_dir(&profile_dir)
        .await
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    let mut profiles = Vec::new();

    while let Some(entry) = entries.next_entry().await.transpose() {
        let entry = entry.map_err(|e| format!("Directory entry error: {}", e))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                let name_str = name.to_string();

                // Filter by search term if provided
                if let Some(ref search_term) = search {
                    if name_str.to_lowercase().contains(&search_term.to_lowercase()) {
                        profiles.push(name_str);
                    }
                } else {
                    profiles.push(name_str);
                }
            }
        }
    }

    profiles.sort();
    Ok(profiles)
}
```

**TypeScript:**
```typescript
export async function listProfiles(search?: string): Promise<string[]> {
  return await invoke<string[]>('list_profiles', { search });
}
```

---

## Pattern 6: Delete/Cleanup Operations

**Use Case:** Remove resources with confirmation.

**Rust:**
```rust
#[command]
pub async fn delete_profile(name: String) -> Result<String, String> {
    let profile_path = get_profile_path(&name)?;

    if !profile_path.exists() {
        return Err(format!("Profile '{}' does not exist", name));
    }

    fs::remove_file(&profile_path)
        .await
        .map_err(|e| format!("Failed to delete profile: {}", e))?;

    Ok(format!("Profile '{}' deleted successfully", name))
}
```

**TypeScript:**
```typescript
export async function deleteProfile(name: string): Promise<string> {
  const confirmed = window.confirm(`Delete profile "${name}"?`);
  if (!confirmed) throw new Error('Cancelled');

  return await invoke<string>('delete_profile', { name });
}
```

---

## Pattern 7: Event-Driven Updates

**Use Case:** Push real-time updates from backend to frontend.

**Rust (Background Task):**
```rust
use tauri::{AppHandle, Emitter};
use std::time::Duration;

#[derive(Clone, Serialize)]
struct PeakMeterUpdate {
    peak_db: f32,
    timestamp: u64,
}

pub fn start_audio_monitoring(app: AppHandle) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(33)); // 30 FPS

        loop {
            interval.tick().await;

            if let Ok(peak_db) = get_current_peak_level() {
                let update = PeakMeterUpdate {
                    peak_db,
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                };

                let _ = app.emit("peak_meter_update", update);
            }
        }
    });
}

#[command]
pub fn start_peak_monitoring(app: AppHandle) -> Result<(), String> {
    start_audio_monitoring(app);
    Ok(())
}

#[command]
pub fn stop_peak_monitoring() -> Result<(), String> {
    // Implementation to stop monitoring
    Ok(())
}
```

**TypeScript:**
```typescript
import { listen, UnlistenFn } from '@tauri-apps/api/event';

interface PeakMeterUpdate {
  peakDb: number;
  timestamp: number;
}

export function usePeakMeter() {
  const [peak, setPeak] = useState<number>(0);

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    // Start monitoring
    invoke('start_peak_monitoring').then(() => {
      listen<PeakMeterUpdate>('peak_meter_update', (event) => {
        setPeak(event.payload.peakDb);
      }).then((fn) => {
        unlisten = fn;
      });
    });

    // Cleanup
    return () => {
      invoke('stop_peak_monitoring');
      unlisten?.();
    };
  }, []);

  return peak;
}
```

---

## Pattern 8: State Management with Tauri

**Use Case:** Centralized application state.

**Rust:**
```rust
use std::sync::Mutex;

pub struct AppState {
    pub current_profile: Mutex<Option<EqProfile>>,
    pub settings: Mutex<Settings>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_profile: Mutex::new(None),
            settings: Mutex::new(Settings::default()),
        }
    }
}

// In lib.rs
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// In commands
#[command]
pub fn get_current_profile(
    state: State<'_, AppState>,
) -> Result<Option<EqProfile>, String> {
    state
        .current_profile
        .lock()
        .map_err(|e| format!("Lock error: {}", e))
        .map(|profile| profile.clone())
}
```

---

## Pattern 9: Progress Reporting

**Use Case:** Long-running operations with progress updates.

**Rust:**
```rust
#[derive(Clone, Serialize)]
pub struct ImportProgress {
    pub current: usize,
    pub total: usize,
    pub message: String,
}

#[command]
pub async fn import_bulk_profiles(
    files: Vec<String>,
    app: AppHandle,
) -> Result<usize, String> {
    let total = files.len();

    for (idx, file_path) in files.iter().enumerate() {
        // Emit progress
        let _ = app.emit("import_progress", ImportProgress {
            current: idx + 1,
            total,
            message: format!("Importing {}...", file_path),
        });

        // Do work
        import_single_profile(file_path).await?;

        // Small delay to avoid overwhelming UI
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    Ok(total)
}
```

**TypeScript:**
```typescript
export async function importBulkProfiles(files: string[]): Promise<number> {
  const unlisten = await listen<ImportProgress>('import_progress', (event) => {
    const { current, total, message } = event.payload;
    console.log(`Progress: ${current}/${total} - ${message}`);
  });

  try {
    const count = await invoke<number>('import_bulk_profiles', { files });
    return count;
  } finally {
    unlisten();
  }
}
```

---

## Pattern 10: Caching Expensive Operations

**Use Case:** Cache file reads, avoid repeated I/O.

**Rust:**
```rust
use std::collections::HashMap;
use std::sync::Mutex;

pub struct ProfileCache {
    cache: Mutex<HashMap<String, (EqProfile, SystemTime)>>,
    ttl: Duration,
}

impl ProfileCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    pub async fn get_or_load(&self, name: &str) -> Result<EqProfile, String> {
        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some((profile, cached_at)) = cache.get(name) {
                let age = SystemTime::now().duration_since(*cached_at).unwrap();
                if age < self.ttl {
                    return Ok(profile.clone());
                }
            }
        }

        // Cache miss or expired - load from disk
        let profile = load_profile_from_disk(name).await?;

        // Update cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(name.to_string(), (profile.clone(), SystemTime::now()));
        }

        Ok(profile)
    }
}

// In lib.rs
tauri::Builder::default()
    .manage(ProfileCache::new(60)) // 60 second TTL
    .invoke_handler(...)
```

---

## Anti-Patterns to Avoid

### ❌ Blocking Operations in Commands

```rust
// BAD: Blocks Tauri's thread pool
#[command]
pub fn expensive_computation() -> String {
    std::thread::sleep(Duration::from_secs(10)); // Blocks!
    "done".to_string()
}

// GOOD: Use async + tokio
#[command]
pub async fn expensive_computation() -> String {
    tokio::time::sleep(Duration::from_secs(10)).await;
    "done".to_string()
}
```

### ❌ Unwrap in Commands

```rust
// BAD: Panics crash the app
#[command]
pub fn load_file(path: String) -> String {
    std::fs::read_to_string(path).unwrap() // Crash if file missing!
}

// GOOD: Return Result
#[command]
pub fn load_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))
}
```

### ❌ Chatty IPC (Too Many Small Calls)

```typescript
// BAD: 100 IPC calls
for (let i = 0; i < 100; i++) {
  await updateBand(i, bands[i]);
}

// GOOD: 1 IPC call
await updateAllBands(bands);
```

### ❌ Missing Input Validation

```rust
// BAD: Trust frontend input
#[command]
pub fn set_volume(vol: f32) {
    unsafe_set_system_volume(vol); // What if vol = 999999?
}

// GOOD: Validate
#[command]
pub fn set_volume(vol: f32) -> Result<(), String> {
    if !(0.0..=1.0).contains(&vol) {
        return Err("Volume must be 0.0 - 1.0".to_string());
    }
    safe_set_volume(vol);
    Ok(())
}
```

---

## Performance Tips

1. **Debounce on Frontend**: For real-time controls (sliders), debounce before invoking
2. **Batch Updates**: Send arrays instead of individual items
3. **Cache Aggressively**: Use Tauri state for frequently accessed data
4. **Async for I/O**: Always use `async` for file/network operations
5. **Stream Large Data**: Use events for large datasets instead of single returns
6. **Profile First**: Use `cargo flamegraph` before optimizing

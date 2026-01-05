# Event-Driven Communication Patterns in Tauri

Complete guide to implementing bidirectional events between Rust backend and React frontend in Tauri v2.

## Overview

Tauri supports two types of communication:
1. **Commands** (Frontend → Backend): Request-response pattern
2. **Events** (Backend → Frontend): Push notifications, real-time updates

## Basic Event Emission (Rust → Frontend)

### Emitting from Rust

```rust
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
struct Notification {
    message: String,
    level: String,
}

pub fn notify_user(app: &AppHandle, message: String) {
    let notification = Notification {
        message,
        level: "info".to_string(),
    };

    // Emit to all windows
    let _ = app.emit("notification", notification);
}

// Emit to specific window
pub fn notify_window(app: &AppHandle, label: &str, message: String) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.emit("notification", Notification {
            message,
            level: "info".to_string(),
        });
    }
}
```

### Listening in Frontend

```typescript
import { listen, UnlistenFn } from '@tauri-apps/api/event';

interface Notification {
  message: string;
  level: string;
}

// One-time listener
const unlisten = await listen<Notification>('notification', (event) => {
  console.log('Received:', event.payload.message);
});

// Cleanup
unlisten();
```

## Pattern 1: Real-Time Monitoring (EQAPO Peak Meter)

### Backend: Continuous Updates

```rust
use tauri::{AppHandle, Emitter};
use std::time::Duration;

#[derive(Clone, serde::Serialize)]
pub struct PeakMeterUpdate {
    pub peak_db: f32,
    pub device_name: String,
    pub sample_rate: u32,
    pub timestamp: u64,
}

pub fn start_peak_monitoring(app: AppHandle) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(33)); // 30 FPS

        loop {
            interval.tick().await;

            // Get current audio peak level (WASAPI)
            let peak_db = get_current_peak_level(); // Implementation in audio_monitor.rs

            let update = PeakMeterUpdate {
                peak_db,
                device_name: get_device_name(),
                sample_rate: get_sample_rate(),
                timestamp: current_timestamp_ms(),
            };

            // Emit to all windows
            let _ = app.emit("peak_meter_update", update);
        }
    });
}
```

### Frontend: React Hook

```typescript
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';

interface PeakMeterUpdate {
  peakDb: number;
  deviceName: string;
  sampleRate: number;
  timestamp: number;
}

export function usePeakMeter() {
  const [peakData, setPeakData] = useState<PeakMeterUpdate | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    listen<PeakMeterUpdate>('peak_meter_update', (event) => {
      setPeakData(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  return peakData;
}

// Usage in component
export function PeakMeter() {
  const peakData = usePeakMeter();

  if (!peakData) return <div>No audio data</div>;

  return (
    <div>
      <p>Peak: {peakData.peakDb.toFixed(1)} dB</p>
      <p>Device: {peakData.deviceName}</p>
      <p>Sample Rate: {peakData.sampleRate} Hz</p>
    </div>
  );
}
```

## Pattern 2: Progress Updates (Long-Running Operations)

### Backend: Progress Events

```rust
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
pub struct ImportProgress {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
    pub status: String,
}

#[tauri::command]
pub async fn import_profiles(
    files: Vec<String>,
    app: AppHandle,
) -> Result<usize, String> {
    let total = files.len();

    for (idx, file_path) in files.iter().enumerate() {
        // Emit progress
        let _ = app.emit("import_progress", ImportProgress {
            current: idx + 1,
            total,
            current_file: file_path.clone(),
            status: "processing".to_string(),
        });

        // Do work
        process_file(file_path).await?;
    }

    // Emit completion
    let _ = app.emit("import_progress", ImportProgress {
        current: total,
        total,
        current_file: String::new(),
        status: "completed".to_string(),
    });

    Ok(total)
}
```

### Frontend: Progress UI

```typescript
import { listen } from '@tauri-apps/api/event';

interface ImportProgress {
  current: number;
  total: number;
  currentFile: string;
  status: string;
}

export function ImportDialog() {
  const [progress, setProgress] = useState<ImportProgress | null>(null);

  const handleImport = async (files: string[]) => {
    // Start listening for progress
    const unlisten = await listen<ImportProgress>('import_progress', (event) => {
      setProgress(event.payload);
    });

    try {
      await invoke('import_profiles', { files });
    } finally {
      unlisten();
    }
  };

  return (
    <div>
      {progress && (
        <div>
          <progress value={progress.current} max={progress.total} />
          <p>{progress.current} / {progress.total}</p>
          <p>Processing: {progress.currentFile}</p>
        </div>
      )}
    </div>
  );
}
```

## Pattern 3: State Synchronization

### Backend: State Change Events

```rust
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

pub struct AppState {
    pub eq_enabled: Mutex<bool>,
}

#[derive(Clone, serde::Serialize)]
struct EqStateChange {
    enabled: bool,
}

#[tauri::command]
pub fn toggle_eq(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<bool, String> {
    let mut enabled = state.eq_enabled
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    *enabled = !*enabled;
    let new_state = *enabled;

    // Notify all windows of state change
    let _ = app.emit("eq_state_changed", EqStateChange { enabled: new_state });

    Ok(new_state)
}
```

### Frontend: Sync UI State

```typescript
import { listen } from '@tauri-apps/api/event';

interface EqStateChange {
  enabled: boolean;
}

export function useEqState() {
  const [enabled, setEnabled] = useState(false);

  useEffect(() => {
    // Listen for state changes from backend (or other windows)
    const unlisten = listen<EqStateChange>('eq_state_changed', (event) => {
      setEnabled(event.payload.enabled);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const toggle = async () => {
    const newState = await invoke<boolean>('toggle_eq');
    setEnabled(newState); // Optimistic update
  };

  return { enabled, toggle };
}
```

## Pattern 4: Error Notifications

### Backend: Error Events

```rust
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
pub struct ErrorEvent {
    pub code: String,
    pub message: String,
    pub severity: String,
}

pub fn emit_error(app: &AppHandle, code: &str, message: String, severity: &str) {
    let error = ErrorEvent {
        code: code.to_string(),
        message,
        severity: severity.to_string(),
    };

    let _ = app.emit("app_error", error);
}

// Usage in commands
#[tauri::command]
pub async fn save_config(app: AppHandle) -> Result<(), String> {
    match write_config_file().await {
        Ok(_) => Ok(()),
        Err(e) => {
            emit_error(&app, "CONFIG_WRITE_FAILED", e.to_string(), "error");
            Err(format!("Failed to save: {}", e))
        }
    }
}
```

### Frontend: Toast Notifications

```typescript
import { listen } from '@tauri-apps/api/event';
import { toast } from 'sonner'; // Or your toast library

interface ErrorEvent {
  code: string;
  message: string;
  severity: 'error' | 'warning' | 'info';
}

export function useErrorListener() {
  useEffect(() => {
    const unlisten = listen<ErrorEvent>('app_error', (event) => {
      const { message, severity } = event.payload;

      switch (severity) {
        case 'error':
          toast.error(message);
          break;
        case 'warning':
          toast.warning(message);
          break;
        default:
          toast.info(message);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}
```

## Pattern 5: Bidirectional Communication

### Backend: Request-Acknowledge Pattern

```rust
#[derive(Clone, serde::Serialize)]
struct ConfirmationRequest {
    id: String,
    message: String,
}

#[derive(serde::Deserialize)]
struct ConfirmationResponse {
    id: String,
    confirmed: bool,
}

// Emit confirmation request
pub async fn request_confirmation(app: &AppHandle, message: String) -> bool {
    let id = uuid::Uuid::new_v4().to_string();

    let request = ConfirmationRequest {
        id: id.clone(),
        message,
    };

    let _ = app.emit("confirmation_request", request);

    // Wait for response via command
    // Frontend calls confirm_action command
    wait_for_confirmation(&id).await
}
```

### Frontend: Confirmation Dialog

```typescript
import { listen } from '@tauri-apps/api/event';

interface ConfirmationRequest {
  id: string;
  message: string;
}

export function ConfirmationListener() {
  const [requests, setRequests] = useState<ConfirmationRequest[]>([]);

  useEffect(() => {
    const unlisten = listen<ConfirmationRequest>('confirmation_request', (event) => {
      setRequests((prev) => [...prev, event.payload]);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const handleConfirm = async (id: string, confirmed: boolean) => {
    await invoke('confirm_action', { id, confirmed });
    setRequests((prev) => prev.filter((r) => r.id !== id));
  };

  return (
    <>
      {requests.map((req) => (
        <Dialog key={req.id}>
          <p>{req.message}</p>
          <button onClick={() => handleConfirm(req.id, true)}>Confirm</button>
          <button onClick={() => handleConfirm(req.id, false)}>Cancel</button>
        </Dialog>
      ))}
    </>
  );
}
```

## Best Practices

### 1. Event Naming Conventions

```
domain_action_detail

Examples:
- audio_peak_update
- profile_saved
- eq_state_changed
- import_progress
- app_error
```

### 2. Payload Size Limits

Keep payloads small (<100KB). For large data:
- Use commands instead
- Send IDs and fetch data on-demand
- Implement pagination

### 3. Event Frequency

Throttle high-frequency events:

```rust
use std::time::{Duration, Instant};

pub struct ThrottledEmitter {
    last_emit: Mutex<Instant>,
    min_interval: Duration,
}

impl ThrottledEmitter {
    pub fn emit<T>(&self, app: &AppHandle, event: &str, payload: T) -> bool
    where
        T: serde::Serialize + Clone,
    {
        let mut last = self.last_emit.lock().unwrap();
        let now = Instant::now();

        if now.duration_since(*last) >= self.min_interval {
            *last = now;
            let _ = app.emit(event, payload);
            true
        } else {
            false
        }
    }
}
```

### 4. Memory Leaks Prevention

Always unlisten in React cleanup:

```typescript
useEffect(() => {
  let unlisten: UnlistenFn | null = null;

  listen('event_name', handler).then((fn) => {
    unlisten = fn;
  });

  return () => {
    unlisten?.(); // CRITICAL: Prevents memory leaks
  };
}, []);
```

### 5. Type Safety

Generate TypeScript types from Rust:

```rust
// Use ts-rs crate
use ts_rs::TS;

#[derive(Clone, serde::Serialize, TS)]
#[ts(export)]
pub struct PeakMeterUpdate {
    pub peak_db: f32,
    pub timestamp: u64,
}
```

Generates `bindings/PeakMeterUpdate.ts`:
```typescript
export interface PeakMeterUpdate {
  peak_db: number;
  timestamp: number;
}
```

## Performance Considerations

1. **Batch Updates**: Send 60 updates/sec max (60 FPS)
2. **Debounce Listeners**: Avoid re-rendering on every event
3. **Use once() for One-Time Events**: `once()` auto-unlistens
4. **Target Specific Windows**: Don't broadcast if not needed

## Security

Events are **NOT sandboxed** - any window can listen to any event.

For sensitive data:
- Use commands with authentication
- Encrypt payloads if necessary
- Validate event sources in multi-window apps

//! Windows Audio Monitor Module
//!
//! Provides real-time audio monitoring using WASAPI:
//! - Output device detection and change notifications
//! - Stream format (sample rate, bit depth) retrieval
//! - Real-time peak metering via loopback capture

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDevice, IMMDeviceEnumerator,
    MMDeviceEnumerator, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX,
    WAVEFORMATEXTENSIBLE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
    COINIT_MULTITHREADED, STGM_READ,
};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, PROPERTYKEY};

/// Information about the current audio output device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioOutputInfo {
    pub device_name: String,
    pub device_id: String,
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub channel_count: u16,
    pub is_default: bool,
    pub format_tag: String,
}

/// Peak meter update data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeakMeterUpdate {
    pub peak_db: f32,
    pub peak_linear: f32,
    pub timestamp: u64,
}

/// Shared state for peak meter data
struct PeakMeterState {
    current_peak: f32,
    peak_hold: f32,
    peak_hold_time: Instant,
}

impl Default for PeakMeterState {
    fn default() -> Self {
        Self {
            current_peak: 0.0,
            peak_hold: 0.0,
            peak_hold_time: Instant::now(),
        }
    }
}

/// Audio monitor managing WASAPI connections
pub struct AudioMonitor {
    peak_state: Arc<Mutex<PeakMeterState>>,
    is_monitoring: Arc<AtomicBool>,
    capture_thread: Mutex<Option<JoinHandle<()>>>,
}

// PKEY_Device_FriendlyName = {a45c254e-df1c-4efd-8020-67d146a850e0}, 14
const PKEY_DEVICE_FRIENDLYNAME: PROPERTYKEY = PROPERTYKEY {
    fmtid: windows::core::GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
    pid: 14,
};

// PKEY_AudioEngine_DeviceFormat = {f19f064d-082c-4e27-bc73-6882a1bb8e4c}, 0
const PKEY_AUDIOENGINE_DEVICEFORMAT: PROPERTYKEY = PROPERTYKEY {
    fmtid: windows::core::GUID::from_u128(0xf19f064d_082c_4e27_bc73_6882a1bb8e4c),
    pid: 0,
};

impl AudioMonitor {
    /// Create a new audio monitor
    pub fn new() -> Self {
        Self {
            peak_state: Arc::new(Mutex::new(PeakMeterState::default())),
            is_monitoring: Arc::new(AtomicBool::new(false)),
            capture_thread: Mutex::new(None),
        }
    }

    /// Get current audio output device information
    pub fn get_audio_output_info(&self) -> Result<AudioOutputInfo, String> {
        unsafe { self.get_device_info_internal() }
    }

    unsafe fn get_device_info_internal(&self) -> Result<AudioOutputInfo, String> {
        // Initialize COM for this thread
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        // Create device enumerator
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| format!("Failed to create device enumerator: {}", e))?;

        // Get default audio endpoint
        let device: IMMDevice = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|e| format!("Failed to get default audio endpoint: {}", e))?;

        // Get device ID
        let device_id_ptr = device
            .GetId()
            .map_err(|e| format!("Failed to get device ID: {}", e))?;

        let device_id = if !device_id_ptr.0.is_null() {
            let len = (0..).take_while(|&i| *device_id_ptr.0.add(i) != 0).count();
            String::from_utf16_lossy(std::slice::from_raw_parts(device_id_ptr.0, len))
        } else {
            String::new()
        };

        // Get device friendly name via property store
        let device_name = self
            .get_device_name(&device)
            .unwrap_or_else(|_| "Unknown Device".to_string());

        // Get device format from property store (user-configured format)
        let (sample_rate, bit_depth, channel_count, format_tag) =
            self.get_device_format(&device).unwrap_or_else(|_| {
                // Fallback to GetMixFormat if property store fails
                self.get_mix_format_fallback(&device).unwrap_or((
                    48000,
                    32,
                    2,
                    "IEEE Float".to_string(),
                ))
            });

        let info = AudioOutputInfo {
            device_name,
            device_id,
            sample_rate,
            bit_depth,
            channel_count,
            is_default: true,
            format_tag,
        };

        CoUninitialize();
        Ok(info)
    }

    /// Get device format from property store (PKEY_AudioEngine_DeviceFormat)
    unsafe fn get_device_format(
        &self,
        device: &IMMDevice,
    ) -> Result<(u32, u16, u16, String), String> {
        let store: IPropertyStore = device
            .OpenPropertyStore(STGM_READ)
            .map_err(|e| format!("Failed to open property store: {}", e))?;

        let prop = store
            .GetValue(&PKEY_AUDIOENGINE_DEVICEFORMAT)
            .map_err(|e| format!("Failed to get device format: {}", e))?;

        // The PROPVARIANT for VT_BLOB has the following layout:
        // vt (2 bytes) + reserved (6 bytes) + blob (BLOB struct: cbSize u32 + pBlobData *u8)
        // Total offset to blob: 8 bytes, pBlobData at offset 12
        let propvar_ptr = &prop as *const _ as *const u8;

        // Read vt to check it's VT_BLOB (65)
        let vt = *(propvar_ptr as *const u16);
        if vt != 65 {
            // VT_BLOB = 0x41 = 65
            return Err(format!("Property is not VT_BLOB, got vt={}", vt));
        }

        // Read cbSize at offset 8
        let cb_size = *((propvar_ptr.add(8)) as *const u32);
        // Read pBlobData at offset 8+4 = 12 on 32-bit, but on 64-bit it's at offset 16 due to pointer alignment
        // Actually BLOB is { cbSize: ULONG (4 bytes), pBlobData: *mut u8 }
        // On 64-bit, pBlobData is pointer-aligned so it's at offset 8 + 8 = 16
        let blob_data = *((propvar_ptr.add(16)) as *const *const u8);

        if blob_data.is_null() || cb_size < std::mem::size_of::<WAVEFORMATEX>() as u32 {
            return Err("Invalid format blob".to_string());
        }

        let format = &*(blob_data as *const WAVEFORMATEX);
        let (bit_depth, format_tag) = self.get_format_details(format);

        Ok((
            format.nSamplesPerSec,
            bit_depth,
            format.nChannels,
            format_tag,
        ))
    }

    /// Fallback: get format via GetMixFormat
    unsafe fn get_mix_format_fallback(
        &self,
        device: &IMMDevice,
    ) -> Result<(u32, u16, u16, String), String> {
        let audio_client: IAudioClient = device
            .Activate(CLSCTX_ALL, None)
            .map_err(|e| format!("Failed to activate audio client: {}", e))?;

        let format_ptr = audio_client
            .GetMixFormat()
            .map_err(|e| format!("Failed to get mix format: {}", e))?;

        if format_ptr.is_null() {
            return Err("Mix format is null".to_string());
        }

        let format = &*format_ptr;
        let (bit_depth, format_tag) = self.get_format_details(format);

        let result = (
            format.nSamplesPerSec,
            bit_depth,
            format.nChannels,
            format_tag,
        );

        CoTaskMemFree(Some(format_ptr as *const _));

        Ok(result)
    }

    unsafe fn get_device_name(&self, device: &IMMDevice) -> Result<String, String> {
        let store: IPropertyStore = device
            .OpenPropertyStore(STGM_READ)
            .map_err(|e| format!("Failed to open property store: {}", e))?;

        let prop = store
            .GetValue(&PKEY_DEVICE_FRIENDLYNAME)
            .map_err(|e| format!("Failed to get device name: {}", e))?;

        // Try to get as string
        let name = prop.to_string();
        if name.is_empty() {
            Ok("Unknown Device".to_string())
        } else {
            Ok(name)
        }
    }

    fn get_format_details(&self, format: &WAVEFORMATEX) -> (u16, String) {
        // WAVE_FORMAT_PCM = 1, WAVE_FORMAT_IEEE_FLOAT = 3, WAVE_FORMAT_EXTENSIBLE = 0xFFFE
        match format.wFormatTag {
            1 => (format.wBitsPerSample, "PCM".to_string()),
            3 => (format.wBitsPerSample, "IEEE Float".to_string()),
            0xFFFE => {
                // Extensible format - look at SubFormat
                // Use raw pointer to avoid unaligned reference issues with packed struct
                let ext_ptr = format as *const WAVEFORMATEX as *const WAVEFORMATEXTENSIBLE;
                // Use wBitsPerSample from base format - it's more reliable than wValidBitsPerSample
                let bits = format.wBitsPerSample;
                // Check SubFormat GUID for float vs PCM
                // KSDATAFORMAT_SUBTYPE_IEEE_FLOAT = 00000003-0000-0010-8000-00aa00389b71
                let float_guid =
                    windows::core::GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);
                // Read SubFormat using read_unaligned via raw pointer offset
                let sub_format_ptr = unsafe { std::ptr::addr_of!((*ext_ptr).SubFormat) };
                let sub_format = unsafe { std::ptr::read_unaligned(sub_format_ptr) };
                let format_tag = if sub_format == float_guid {
                    "IEEE Float".to_string()
                } else {
                    "PCM".to_string()
                };
                (bits, format_tag)
            }
            _ => (format.wBitsPerSample, "Unknown".to_string()),
        }
    }

    /// Start peak meter monitoring
    pub fn start_peak_monitoring<F>(&self, callback: F) -> Result<(), String>
    where
        F: Fn(PeakMeterUpdate) + Send + 'static,
    {
        if self.is_monitoring.load(Ordering::SeqCst) {
            return Ok(()); // Already monitoring
        }

        self.is_monitoring.store(true, Ordering::SeqCst);

        let peak_state = Arc::clone(&self.peak_state);
        let is_monitoring = Arc::clone(&self.is_monitoring);

        let handle = thread::spawn(move || unsafe {
            if let Err(e) = capture_loop(peak_state, is_monitoring, callback) {
                eprintln!("Peak capture error: {}", e);
            }
        });

        *self.capture_thread.lock() = Some(handle);
        Ok(())
    }

    /// Stop peak meter monitoring
    pub fn stop_peak_monitoring(&self) {
        self.is_monitoring.store(false, Ordering::SeqCst);

        if let Some(handle) = self.capture_thread.lock().take() {
            let _ = handle.join();
        }
    }

    /// Get current peak value
    pub fn get_current_peak(&self) -> PeakMeterUpdate {
        let state = self.peak_state.lock();
        let peak_linear = state.current_peak;
        let peak_db = if peak_linear > 0.0 {
            20.0 * peak_linear.log10()
        } else {
            -100.0
        };

        PeakMeterUpdate {
            peak_db,
            peak_linear,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }
}

impl Drop for AudioMonitor {
    fn drop(&mut self) {
        self.stop_peak_monitoring();
    }
}

/// Audio capture loop running on a separate thread
/// This loop automatically reconnects if the audio device or format changes
unsafe fn capture_loop<F>(
    peak_state: Arc<Mutex<PeakMeterState>>,
    is_monitoring: Arc<AtomicBool>,
    callback: F,
) -> Result<(), String>
where
    F: Fn(PeakMeterUpdate),
{
    // Initialize COM
    let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();

    // Outer loop handles reconnection on device/format changes
    while is_monitoring.load(Ordering::SeqCst) {
        // Try to capture, reconnect if it fails
        match capture_session(&peak_state, &is_monitoring, &callback) {
            Ok(()) => break, // Normal exit (monitoring stopped)
            Err(e) => {
                eprintln!("Capture session error (will retry): {}", e);
                // Wait a bit before reconnecting
                thread::sleep(Duration::from_millis(500));
            }
        }
    }

    CoUninitialize();
    Ok(())
}

/// Single capture session - returns Ok(()) when monitoring stopped, Err on device change/error
unsafe fn capture_session<F>(
    peak_state: &Arc<Mutex<PeakMeterState>>,
    is_monitoring: &Arc<AtomicBool>,
    callback: &F,
) -> Result<(), String>
where
    F: Fn(PeakMeterUpdate),
{
    // Get default device
    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .map_err(|e| format!("Failed to create enumerator: {}", e))?;

    let device: IMMDevice = enumerator
        .GetDefaultAudioEndpoint(eRender, eConsole)
        .map_err(|e| format!("Failed to get endpoint: {}", e))?;

    // Create audio client for loopback
    let audio_client: IAudioClient = device
        .Activate(CLSCTX_ALL, None)
        .map_err(|e| format!("Failed to activate client: {}", e))?;

    // Get mix format
    let format_ptr = audio_client
        .GetMixFormat()
        .map_err(|e| format!("Failed to get format: {}", e))?;

    let format = &*format_ptr;
    let bytes_per_sample = format.wBitsPerSample / 8;
    let channels = format.nChannels as usize;
    let is_float = format.wFormatTag == 3
        || (format.wFormatTag == 0xFFFE && {
            let ext_ptr = format_ptr as *const WAVEFORMATEXTENSIBLE;
            let float_guid = windows::core::GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);
            let sub_format_ptr = std::ptr::addr_of!((*ext_ptr).SubFormat);
            let sub_format = std::ptr::read_unaligned(sub_format_ptr);
            sub_format == float_guid
        });

    // Initialize audio client for loopback capture
    let buffer_duration = 10_000_000i64; // 1 second in 100ns units
    audio_client
        .Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            buffer_duration,
            0,
            format_ptr,
            None,
        )
        .map_err(|e| format!("Failed to initialize client: {}", e))?;

    // Get capture client
    let capture_client: IAudioCaptureClient = audio_client
        .GetService()
        .map_err(|e| format!("Failed to get capture client: {}", e))?;

    // Start capture
    audio_client
        .Start()
        .map_err(|e| format!("Failed to start capture: {}", e))?;

    let mut last_emit = Instant::now();
    let emit_interval = Duration::from_millis(33); // ~30 FPS
    let poll_interval = Duration::from_millis(10); // Poll every 10ms

    // Decay constants
    let decay_factor = 0.95f32;
    let peak_hold_duration = Duration::from_secs(1);

    // Track consecutive errors to detect device changes
    let mut consecutive_errors = 0u32;

    while is_monitoring.load(Ordering::SeqCst) {
        // Sleep to avoid busy-waiting
        thread::sleep(poll_interval);

        // Get available data
        let mut buffer_ptr = std::ptr::null_mut();
        let mut frames_available = 0u32;
        let mut flags = 0u32;

        let get_result = capture_client.GetBuffer(
            &mut buffer_ptr,
            &mut frames_available,
            &mut flags,
            None,
            None,
        );

        if get_result.is_err() {
            consecutive_errors += 1;
            // After 10 consecutive errors (~100ms), assume device changed
            if consecutive_errors > 10 {
                audio_client.Stop().ok();
                CoTaskMemFree(Some(format_ptr as *const _));
                return Err("Device or format changed".to_string());
            }
            continue;
        }

        consecutive_errors = 0;

        if frames_available > 0 && !buffer_ptr.is_null() {
            let sample_count = frames_available as usize * channels;

            // Calculate peak from samples
            let mut max_sample = 0.0f32;

            if is_float && bytes_per_sample == 4 {
                let samples = std::slice::from_raw_parts(buffer_ptr as *const f32, sample_count);
                for &s in samples {
                    let abs = s.abs();
                    if abs > max_sample {
                        max_sample = abs;
                    }
                }
            } else if bytes_per_sample == 2 {
                let samples = std::slice::from_raw_parts(buffer_ptr as *const i16, sample_count);
                for &s in samples {
                    let normalized = (s as f32) / 32768.0;
                    let abs = normalized.abs();
                    if abs > max_sample {
                        max_sample = abs;
                    }
                }
            } else if bytes_per_sample == 3 {
                // 24-bit audio
                let data = std::slice::from_raw_parts(buffer_ptr, sample_count * 3);
                for i in 0..sample_count {
                    let offset = i * 3;
                    let sample_i32 = ((data[offset] as i32) << 8)
                        | ((data[offset + 1] as i32) << 16)
                        | ((data[offset + 2] as i32) << 24);
                    let normalized = (sample_i32 as f32) / 2147483648.0;
                    let abs = normalized.abs();
                    if abs > max_sample {
                        max_sample = abs;
                    }
                }
            } else if bytes_per_sample == 4 && !is_float {
                // 32-bit integer
                let samples = std::slice::from_raw_parts(buffer_ptr as *const i32, sample_count);
                for &s in samples {
                    let normalized = (s as f32) / 2147483648.0;
                    let abs = normalized.abs();
                    if abs > max_sample {
                        max_sample = abs;
                    }
                }
            }

            // Update peak state with fast attack, slow decay
            {
                let mut state = peak_state.lock();

                // Fast attack
                if max_sample > state.current_peak {
                    state.current_peak = max_sample;
                } else {
                    // Slow decay
                    state.current_peak *= decay_factor;
                }

                // Peak hold
                if max_sample > state.peak_hold {
                    state.peak_hold = max_sample;
                    state.peak_hold_time = Instant::now();
                } else if state.peak_hold_time.elapsed() > peak_hold_duration {
                    state.peak_hold = state.current_peak;
                }
            }

            // Release buffer
            let _ = capture_client.ReleaseBuffer(frames_available);

            // Emit update at throttled rate
            if last_emit.elapsed() >= emit_interval {
                let state = peak_state.lock();
                let peak_linear = state.current_peak;
                let peak_db = if peak_linear > 0.0 {
                    20.0 * peak_linear.log10()
                } else {
                    -100.0
                };

                callback(PeakMeterUpdate {
                    peak_db,
                    peak_linear,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                });

                last_emit = Instant::now();
            }
        }
    }

    // Cleanup
    audio_client.Stop().ok();
    CoTaskMemFree(Some(format_ptr as *const _));

    Ok(())
}

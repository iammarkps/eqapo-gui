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

// =============================================================================
// Constants
// =============================================================================

/// Buffer duration for WASAPI capture in 100-nanosecond units (1 second)
const WASAPI_BUFFER_DURATION_100NS: i64 = 10_000_000;

/// Interval between peak meter UI updates (~30 FPS)
const PEAK_METER_EMIT_INTERVAL: Duration = Duration::from_millis(33);

/// Interval between audio buffer polling
const AUDIO_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Decay factor for peak meter (per poll interval)
const PEAK_DECAY_FACTOR: f32 = 0.95;

/// Duration to hold peak value before decay
const PEAK_HOLD_DURATION: Duration = Duration::from_secs(1);

/// Number of consecutive errors before assuming device change
const DEVICE_CHANGE_ERROR_THRESHOLD: u32 = 10;

/// Delay before attempting to reconnect after device change
const DEVICE_RECONNECT_DELAY: Duration = Duration::from_millis(500);

// COM initialization result codes
/// S_FALSE - COM already initialized (acceptable)
const COM_S_FALSE: u32 = 1;

/// RPC_E_CHANGED_MODE - COM initialized with different threading model (acceptable)
const COM_RPC_E_CHANGED_MODE: u32 = 0x80010106;

// PROPVARIANT / BLOB constants
/// VT_BLOB variant type identifier
const VT_BLOB: u16 = 65;

/// Offset to BLOB cbSize field in PROPVARIANT structure
const PROPVARIANT_BLOB_SIZE_OFFSET: usize = 8;

/// Offset to BLOB pBlobData pointer in PROPVARIANT structure (64-bit)
const PROPVARIANT_BLOB_DATA_OFFSET: usize = 16;

// WAVE_FORMAT constants
/// WAVE_FORMAT_PCM - Standard PCM audio format
const WAVE_FORMAT_PCM: u16 = 1;

/// WAVE_FORMAT_IEEE_FLOAT - 32-bit IEEE float audio format
const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;

/// WAVE_FORMAT_EXTENSIBLE - Extended format with SubFormat GUID
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;

// Audio sample format constants
/// Bytes per sample for 16-bit PCM audio
const BYTES_PER_SAMPLE_16BIT: u16 = 2;

/// Bytes per sample for 24-bit PCM audio
const BYTES_PER_SAMPLE_24BIT: u16 = 3;

/// Bytes per sample for 32-bit audio (PCM or float)
const BYTES_PER_SAMPLE_32BIT: u16 = 4;

/// Maximum value for 16-bit signed PCM samples (normalization divisor)
const PCM_16BIT_MAX: f32 = 32768.0;

/// Maximum value for 32-bit signed PCM samples (normalization divisor)
const PCM_32BIT_MAX: f32 = 2147483648.0;

/// Bits per byte
const BITS_PER_BYTE: u16 = 8;

/// Bit shift amount for first byte in 24-bit PCM unpacking
const BIT_SHIFT_24BIT_BYTE0: i32 = 8;

/// Bit shift amount for second byte in 24-bit PCM unpacking
const BIT_SHIFT_24BIT_BYTE1: i32 = 16;

/// Bit shift amount for third byte in 24-bit PCM unpacking
const BIT_SHIFT_24BIT_BYTE2: i32 = 24;

// dB conversion constants
/// Multiplier for linear to dB conversion (20 * log10)
const DB_CONVERSION_FACTOR: f32 = 20.0;

/// dB value representing silence (when peak is 0.0)
const DB_SILENCE_THRESHOLD: f32 = -100.0;

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

impl std::fmt::Debug for AudioMonitor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioMonitor")
            .field("is_monitoring", &self.is_monitoring.load(Ordering::SeqCst))
            .field("has_capture_thread", &self.capture_thread.lock().is_some())
            .finish()
    }
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
        // SAFETY: This calls Windows COM APIs which require unsafe. The safety
        // invariants are:
        // 1. COM is initialized before any COM calls and uninitialized after
        // 2. All COM interface pointers are valid (obtained from Windows APIs)
        // 3. All memory from COM (CoTaskMemAlloc) is freed with CoTaskMemFree
        // 4. Slices are created from valid pointers with correct lengths
        unsafe { self.get_device_info_internal() }
    }

    /// Internal implementation of device info retrieval.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it:
    /// - Calls Windows COM APIs that require proper initialization
    /// - Reads from raw pointers returned by Windows APIs
    /// - Interprets memory as WAVEFORMATEX structures
    ///
    /// Caller must ensure this is called on a thread where COM can be initialized.
    unsafe fn get_device_info_internal(&self) -> Result<AudioOutputInfo, String> {
        // Initialize COM for this thread. CoInitializeEx returns S_OK on first init,
        // S_FALSE if already initialized (which is fine), or an error.
        // We ignore S_FALSE as it's expected when COM is already initialized.
        // RPC_E_CHANGED_MODE (0x80010106) means COM was initialized with different
        // threading model, which is acceptable - we can still use COM.
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        if hr.is_err() {
            let code = hr.0 as u32;
            // Accept S_FALSE (already initialized) and RPC_E_CHANGED_MODE (different threading model)
            if code != COM_S_FALSE && code != COM_RPC_E_CHANGED_MODE {
                return Err(format!("COM initialization failed: HRESULT 0x{:08X}", code));
            }
        }

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

        // SAFETY: device_id_ptr is a valid PWSTR allocated by Windows.
        // We find the null terminator by scanning, then create a slice of that length.
        // The pointer remains valid until we're done reading.
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
    ///
    /// # Safety
    ///
    /// Caller must ensure COM is initialized on the current thread.
    /// This function reads raw memory from PROPVARIANT structures returned by Windows.
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

        // SAFETY: We need to read the raw PROPVARIANT structure to extract the blob.
        // The PROPVARIANT for VT_BLOB has the following layout:
        // vt (2 bytes) + reserved (6 bytes) + blob (BLOB struct: cbSize u32 + pBlobData *u8)
        // Total offset to blob: 8 bytes, pBlobData at offset 12 (32-bit) or 16 (64-bit)
        let propvar_ptr = &prop as *const _ as *const u8;

        // Read vt to check it's VT_BLOB
        let vt = *(propvar_ptr as *const u16);
        if vt != VT_BLOB {
            return Err(format!("Property is not VT_BLOB, got vt={}", vt));
        }

        // SAFETY: Read cbSize at offset 8, then pBlobData at offset 16 (64-bit aligned).
        // The BLOB struct is { cbSize: ULONG (4 bytes), pBlobData: *mut u8 }.
        // On 64-bit, pBlobData is pointer-aligned so it's at offset 8 + 8 = 16.
        let cb_size = *((propvar_ptr.add(PROPVARIANT_BLOB_SIZE_OFFSET)) as *const u32);
        let blob_data = *((propvar_ptr.add(PROPVARIANT_BLOB_DATA_OFFSET)) as *const *const u8);

        if blob_data.is_null() || cb_size < std::mem::size_of::<WAVEFORMATEX>() as u32 {
            return Err("Invalid format blob".to_string());
        }

        // SAFETY: blob_data points to valid WAVEFORMATEX data allocated by Windows,
        // and we verified the size is sufficient.
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
    ///
    /// # Safety
    ///
    /// Caller must ensure COM is initialized. The returned format pointer from
    /// GetMixFormat is freed with CoTaskMemFree before returning.
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

        // SAFETY: format_ptr is valid and non-null (checked above).
        // We read the format data before freeing the memory.
        let format = &*format_ptr;
        let (bit_depth, format_tag) = self.get_format_details(format);

        let result = (
            format.nSamplesPerSec,
            bit_depth,
            format.nChannels,
            format_tag,
        );

        // SAFETY: format_ptr was allocated by Windows via CoTaskMemAlloc,
        // so we must free it with CoTaskMemFree.
        CoTaskMemFree(Some(format_ptr as *const _));

        Ok(result)
    }

    /// Get device friendly name from property store.
    ///
    /// # Safety
    ///
    /// Caller must ensure COM is initialized on the current thread.
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
        match format.wFormatTag {
            WAVE_FORMAT_PCM => (format.wBitsPerSample, "PCM".to_string()),
            WAVE_FORMAT_IEEE_FLOAT => (format.wBitsPerSample, "IEEE Float".to_string()),
            WAVE_FORMAT_EXTENSIBLE => {
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
            DB_CONVERSION_FACTOR * peak_linear.log10()
        } else {
            DB_SILENCE_THRESHOLD
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

/// Calculates the maximum absolute sample value from an audio buffer.
///
/// # Safety
///
/// - `buffer_ptr` must be a valid pointer to audio sample data
/// - `sample_count` must not exceed the actual buffer size
/// - `bytes_per_sample` must match the actual sample format
/// - `is_float` must correctly indicate if samples are IEEE float format
///
/// # Returns
///
/// The maximum absolute sample value normalized to [0.0, 1.0] range.
unsafe fn calculate_peak_from_buffer(
    buffer_ptr: *mut u8,
    sample_count: usize,
    bytes_per_sample: u16,
    is_float: bool,
) -> f32 {
    let mut max_sample = 0.0f32;

    if is_float && bytes_per_sample == BYTES_PER_SAMPLE_32BIT {
        // 32-bit IEEE float
        let samples = std::slice::from_raw_parts(buffer_ptr as *const f32, sample_count);
        for &s in samples {
            let abs = s.abs();
            if abs > max_sample {
                max_sample = abs;
            }
        }
    } else if bytes_per_sample == BYTES_PER_SAMPLE_16BIT {
        // 16-bit PCM
        let samples = std::slice::from_raw_parts(buffer_ptr as *const i16, sample_count);
        for &s in samples {
            let normalized = (s as f32) / PCM_16BIT_MAX;
            let abs = normalized.abs();
            if abs > max_sample {
                max_sample = abs;
            }
        }
    } else if bytes_per_sample == BYTES_PER_SAMPLE_24BIT {
        // 24-bit PCM (packed as 3 bytes per sample)
        let data = std::slice::from_raw_parts(buffer_ptr, sample_count * BYTES_PER_SAMPLE_24BIT as usize);
        for i in 0..sample_count {
            let offset = i * BYTES_PER_SAMPLE_24BIT as usize;
            // Sign-extend 24-bit to 32-bit by shifting to top of i32
            let sample_i32 = ((data[offset] as i32) << BIT_SHIFT_24BIT_BYTE0)
                | ((data[offset + 1] as i32) << BIT_SHIFT_24BIT_BYTE1)
                | ((data[offset + 2] as i32) << BIT_SHIFT_24BIT_BYTE2);
            let normalized = (sample_i32 as f32) / PCM_32BIT_MAX;
            let abs = normalized.abs();
            if abs > max_sample {
                max_sample = abs;
            }
        }
    } else if bytes_per_sample == BYTES_PER_SAMPLE_32BIT && !is_float {
        // 32-bit PCM integer
        let samples = std::slice::from_raw_parts(buffer_ptr as *const i32, sample_count);
        for &s in samples {
            let normalized = (s as f32) / PCM_32BIT_MAX;
            let abs = normalized.abs();
            if abs > max_sample {
                max_sample = abs;
            }
        }
    }

    max_sample
}

/// Audio capture loop running on a separate thread.
/// This loop automatically reconnects if the audio device or format changes.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Initializes and uses Windows COM APIs
/// - Calls capture_session which performs unsafe audio buffer operations
///
/// Must be called on a dedicated thread (not the main thread) to avoid
/// blocking the UI.
unsafe fn capture_loop<F>(
    peak_state: Arc<Mutex<PeakMeterState>>,
    is_monitoring: Arc<AtomicBool>,
    callback: F,
) -> Result<(), String>
where
    F: Fn(PeakMeterUpdate),
{
    // Initialize COM for this thread. We accept RPC_E_CHANGED_MODE as it means
    // COM is already initialized with a different threading model, which is fine.
    let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
    if hr.is_err() {
        let code = hr.0 as u32;
        // Accept S_FALSE (already initialized) and RPC_E_CHANGED_MODE (different threading model)
        if code != COM_S_FALSE && code != COM_RPC_E_CHANGED_MODE {
            return Err(format!("COM initialization failed: HRESULT 0x{:08X}", code));
        }
    }

    // Outer loop handles reconnection on device/format changes
    while is_monitoring.load(Ordering::SeqCst) {
        // Try to capture, reconnect if it fails
        match capture_session(&peak_state, &is_monitoring, &callback) {
            Ok(()) => break, // Normal exit (monitoring stopped)
            Err(e) => {
                eprintln!("Capture session error (will retry): {}", e);
                // Wait before reconnecting to avoid busy-loop on persistent errors
                thread::sleep(DEVICE_RECONNECT_DELAY);
            }
        }
    }

    CoUninitialize();
    Ok(())
}

/// Single capture session - returns Ok(()) when monitoring stopped, Err on device change/error.
///
/// # Safety
///
/// This function is unsafe because it:
/// - Reads from raw audio buffers returned by WASAPI
/// - Interprets buffer memory as audio samples (f32, i16, i32, or 24-bit)
/// - Must properly release buffers back to WASAPI
///
/// Caller must ensure:
/// - COM is initialized on the current thread
/// - is_monitoring is properly managed to allow clean shutdown
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

    // SAFETY: format_ptr is valid, returned by GetMixFormat.
    let format = &*format_ptr;
    let bytes_per_sample = format.wBitsPerSample / BITS_PER_BYTE;
    let channels = format.nChannels as usize;

    // SAFETY: Check if format is IEEE float by examining wFormatTag or SubFormat GUID.
    let is_float = format.wFormatTag == WAVE_FORMAT_IEEE_FLOAT
        || (format.wFormatTag == WAVE_FORMAT_EXTENSIBLE && {
            let ext_ptr = format_ptr as *const WAVEFORMATEXTENSIBLE;
            let float_guid = windows::core::GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);
            // Use read_unaligned because WAVEFORMATEXTENSIBLE may not be properly aligned
            let sub_format_ptr = std::ptr::addr_of!((*ext_ptr).SubFormat);
            let sub_format = std::ptr::read_unaligned(sub_format_ptr);
            sub_format == float_guid
        });

    // Initialize audio client for loopback capture
    audio_client
        .Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            WASAPI_BUFFER_DURATION_100NS,
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

    // Track consecutive errors to detect device changes
    let mut consecutive_errors = 0u32;

    while is_monitoring.load(Ordering::SeqCst) {
        // Sleep to avoid busy-waiting
        thread::sleep(AUDIO_POLL_INTERVAL);

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
            // After threshold consecutive errors, assume device changed
            if consecutive_errors > DEVICE_CHANGE_ERROR_THRESHOLD {
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
            // SAFETY: buffer_ptr is valid and contains `frames_available * channels` samples.
            // The buffer format matches what we detected from GetMixFormat.
            let max_sample = calculate_peak_from_buffer(
                buffer_ptr,
                sample_count,
                bytes_per_sample,
                is_float,
            );

            // Update peak state with fast attack, slow decay
            {
                let mut state = peak_state.lock();

                // Fast attack
                if max_sample > state.current_peak {
                    state.current_peak = max_sample;
                } else {
                    // Slow decay
                    state.current_peak *= PEAK_DECAY_FACTOR;
                }

                // Peak hold
                if max_sample > state.peak_hold {
                    state.peak_hold = max_sample;
                    state.peak_hold_time = Instant::now();
                } else if state.peak_hold_time.elapsed() > PEAK_HOLD_DURATION {
                    state.peak_hold = state.current_peak;
                }
            }

            // Release buffer back to WASAPI - must always be called after GetBuffer succeeds
            let _ = capture_client.ReleaseBuffer(frames_available);

            // Emit update at throttled rate
            if last_emit.elapsed() >= PEAK_METER_EMIT_INTERVAL {
                let state = peak_state.lock();
                let peak_linear = state.current_peak;
                let peak_db = if peak_linear > 0.0 {
                    DB_CONVERSION_FACTOR * peak_linear.log10()
                } else {
                    DB_SILENCE_THRESHOLD
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

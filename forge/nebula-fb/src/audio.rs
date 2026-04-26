/// Audio system — WAV playback via OSS /dev/dsp (primary) or ALSA /dev/snd/pcmC0D0p (fallback).
///
/// Provides:
/// - `AudioPlayer::new()` — detects available audio device
/// - `play_boot_chime()` — plays embedded BOOT.wav one-shot
/// - `play_post_music()` — loops /usr/share/sounds/post.wav, returns PlayHandle
/// - `PlayHandle::fade_out(ms)` / `PlayHandle::stop()`

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

static BOOT_WAV: &[u8] = include_bytes!("../assets/boot.wav");

/// Parsed WAV header info.
struct WavInfo {
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
    data_offset: usize,
    data_len: usize,
}

fn parse_wav_header(data: &[u8]) -> Option<WavInfo> {
    if data.len() < 44 {
        return None;
    }
    if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }

    let mut pos = 12;
    let mut fmt_channels = 0u16;
    let mut fmt_rate = 0u32;
    let mut fmt_bits = 0u16;
    let mut data_offset = 0usize;
    let mut data_len = 0usize;

    while pos + 8 <= data.len() {
        let chunk_id = &data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;

        if chunk_id == b"fmt " && chunk_size >= 16 {
            fmt_channels = u16::from_le_bytes([data[pos + 10], data[pos + 11]]);
            fmt_rate = u32::from_le_bytes([data[pos + 12], data[pos + 13], data[pos + 14], data[pos + 15]]);
            fmt_bits = u16::from_le_bytes([data[pos + 22], data[pos + 23]]);
        } else if chunk_id == b"data" {
            data_offset = pos + 8;
            data_len = chunk_size;
            break;
        }

        pos += 8 + chunk_size;
        if pos % 2 != 0 {
            pos += 1;
        }
    }

    if data_offset == 0 || fmt_rate == 0 {
        return None;
    }

    Some(WavInfo {
        channels: fmt_channels,
        sample_rate: fmt_rate,
        bits_per_sample: fmt_bits,
        data_offset,
        data_len,
    })
}

/// Handle to a playing audio stream — supports fade-out and stop.
pub struct PlayHandle {
    stop_flag: Arc<AtomicBool>,
    volume: Arc<AtomicU32>,
    fade_flag: Arc<AtomicBool>,
    fade_duration_ms: Arc<AtomicU32>,
}

impl PlayHandle {
    pub fn fade_out(&self, duration_ms: u32) {
        self.fade_duration_ms.store(duration_ms, Ordering::Relaxed);
        self.fade_flag.store(true, Ordering::Relaxed);
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

/// Which audio backend is available.
#[derive(Clone, Copy, PartialEq)]
enum AudioBackend {
    Oss,   // /dev/dsp
    Alsa,  // /dev/snd/pcmC0D0p
    None,
}

pub struct AudioPlayer {
    backend: AudioBackend,
}

impl AudioPlayer {
    pub fn new() -> Self {
        // Try OSS first (simpler ioctls, handles format conversion)
        let backend = if std::path::Path::new("/dev/dsp").exists() {
            eprintln!("[audio] OSS device /dev/dsp found");
            AudioBackend::Oss
        } else if std::path::Path::new("/dev/snd/pcmC0D0p").exists() {
            eprintln!("[audio] ALSA device /dev/snd/pcmC0D0p found");
            AudioBackend::Alsa
        } else {
            eprintln!("[audio] No audio device found — audio disabled");
            AudioBackend::None
        };
        Self { backend }
    }

    pub fn play_boot_chime(&self) {
        let backend = self.backend;
        if backend == AudioBackend::None {
            return;
        }
        std::thread::spawn(move || {
            if let Err(e) = play_wav_data(BOOT_WAV, backend, None) {
                eprintln!("[audio] Boot chime error: {}", e);
            }
        });
    }

    pub fn play_post_music(&self) -> Option<PlayHandle> {
        if self.backend == AudioBackend::None {
            return None;
        }

        let stop_flag = Arc::new(AtomicBool::new(false));
        let volume = Arc::new(AtomicU32::new(1000));
        let fade_flag = Arc::new(AtomicBool::new(false));
        let fade_duration_ms = Arc::new(AtomicU32::new(3000));

        let handle = PlayHandle {
            stop_flag: stop_flag.clone(),
            volume: volume.clone(),
            fade_flag: fade_flag.clone(),
            fade_duration_ms: fade_duration_ms.clone(),
        };

        let backend = self.backend;
        std::thread::spawn(move || {
            let data = match std::fs::read("/usr/share/sounds/post.wav") {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("[audio] Cannot read post.wav: {}", e);
                    return;
                }
            };

            let ctrl = Some(PlayControl {
                stop_flag,
                volume,
                fade_flag,
                fade_duration_ms,
            });

            loop {
                if ctrl.as_ref().map_or(false, |c| c.stop_flag.load(Ordering::Relaxed)) {
                    break;
                }
                match play_wav_data(&data, backend, ctrl.as_ref()) {
                    Ok(stopped) => {
                        if stopped {
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("[audio] POST music error: {}", e);
                        break;
                    }
                }
            }
        });

        Some(handle)
    }
}

struct PlayControl {
    stop_flag: Arc<AtomicBool>,
    volume: Arc<AtomicU32>,
    fade_flag: Arc<AtomicBool>,
    fade_duration_ms: Arc<AtomicU32>,
}

/// Play WAV data to the audio device. Returns Ok(true) if stopped early.
fn play_wav_data(data: &[u8], backend: AudioBackend, ctrl: Option<&PlayControl>) -> Result<bool, String> {
    let info = parse_wav_header(data).ok_or("Invalid WAV header")?;

    eprintln!(
        "[audio] Playing: {}ch {}Hz {}bit, {} bytes PCM",
        info.channels, info.sample_rate, info.bits_per_sample, info.data_len
    );

    // Get raw PCM data from WAV
    let pcm_data = &data[info.data_offset..];
    let actual_len = pcm_data.len().min(info.data_len);
    let pcm_data = &pcm_data[..actual_len];

    // For ALSA: always configure at 48kHz stereo S16 and convert audio data.
    // For OSS: configure at native format (OSS kernel driver handles conversion).
    let (mut file, play_rate, play_channels) = match backend {
        AudioBackend::Oss => {
            let f = open_and_configure_oss(&info)?;
            (f, info.sample_rate, info.channels)
        }
        AudioBackend::Alsa => {
            let dev = open_and_configure_alsa()?;
            (dev.file, dev.sample_rate, dev.channels)
        }
        AudioBackend::None => return Err("No audio backend".to_string()),
    };

    // Convert audio to device format if needed
    let converted;
    let play_data: &[u8] = if info.channels != play_channels || info.sample_rate != play_rate {
        eprintln!(
            "[audio] Converting: {}ch {}Hz → {}ch {}Hz",
            info.channels, info.sample_rate, play_channels, play_rate
        );
        converted = convert_pcm_s16(pcm_data, info.channels, info.sample_rate, play_channels, play_rate);
        &converted
    } else {
        pcm_data
    };

    // Write PCM data in chunks (~100ms each)
    let bytes_per_frame = play_channels as usize * 2; // S16 = 2 bytes/sample
    let chunk_size = ((play_rate as usize * bytes_per_frame) / 10).max(4096);
    let total_len = play_data.len();

    let mut offset = 0;
    let mut fade_start: Option<std::time::Instant> = None;

    while offset < total_len {
        if let Some(c) = ctrl {
            if c.stop_flag.load(Ordering::Relaxed) {
                return Ok(true);
            }

            if c.fade_flag.load(Ordering::Relaxed) {
                if fade_start.is_none() {
                    fade_start = Some(std::time::Instant::now());
                }
                let elapsed_ms = fade_start.unwrap().elapsed().as_millis() as u32;
                let duration = c.fade_duration_ms.load(Ordering::Relaxed);
                if elapsed_ms >= duration {
                    return Ok(true);
                }
                let vol = 1000u32.saturating_sub(elapsed_ms * 1000 / duration);
                c.volume.store(vol, Ordering::Relaxed);
            }
        }

        let end = (offset + chunk_size).min(total_len);
        let chunk = &play_data[offset..end];

        let vol = ctrl.map_or(1000, |c| c.volume.load(Ordering::Relaxed));
        if vol < 1000 {
            let mut scaled = chunk.to_vec();
            for pair in scaled.chunks_exact_mut(2) {
                let sample = i16::from_le_bytes([pair[0], pair[1]]);
                let scaled_sample = (sample as i32 * vol as i32 / 1000) as i16;
                let bytes = scaled_sample.to_le_bytes();
                pair[0] = bytes[0];
                pair[1] = bytes[1];
            }
            write_audio(&mut file, &scaled, backend)?;
        } else {
            write_audio(&mut file, chunk, backend)?;
        }

        offset = end;
    }

    Ok(false)
}

/// Write audio data, handling ALSA XRUN recovery (EPIPE → re-prepare and retry).
fn write_audio(file: &mut std::fs::File, data: &[u8], backend: AudioBackend) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::io::AsRawFd;

    match file.write_all(data) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(32) && backend == AudioBackend::Alsa => {
            // EPIPE (32) = XRUN in ALSA — recover by re-preparing
            eprintln!("[audio] XRUN detected, recovering...");
            const SNDRV_PCM_IOCTL_PREPARE: libc::c_int = 0x00004140;
            let ret = unsafe { libc::ioctl(file.as_raw_fd(), SNDRV_PCM_IOCTL_PREPARE as _) };
            if ret < 0 {
                return Err(format!("XRUN recovery failed: {}", std::io::Error::last_os_error()));
            }
            file.write_all(data).map_err(|e| format!("audio write after recovery: {}", e))
        }
        Err(e) => Err(format!("audio write: {}", e)),
    }
}

/// Convert S16 PCM: channel and sample rate conversion.
fn convert_pcm_s16(data: &[u8], from_ch: u16, from_rate: u32, to_ch: u16, to_rate: u32) -> Vec<u8> {
    let mut result = data.to_vec();

    // Mono → stereo: duplicate each sample
    if from_ch == 1 && to_ch == 2 {
        let mut stereo = Vec::with_capacity(result.len() * 2);
        for pair in result.chunks_exact(2) {
            stereo.extend_from_slice(pair); // L
            stereo.extend_from_slice(pair); // R
        }
        result = stereo;
    }

    // Sample rate conversion with linear interpolation
    if from_rate != to_rate {
        let ch = to_ch as usize;
        let frame_size = ch * 2;
        let num_frames = result.len() / frame_size;
        if num_frames < 2 {
            return result;
        }
        let out_frames = (num_frames as u64 * to_rate as u64 / from_rate as u64) as usize;
        let mut out = Vec::with_capacity(out_frames * frame_size);

        for i in 0..out_frames {
            let src_pos = i as f64 * (num_frames - 1) as f64 / (out_frames - 1).max(1) as f64;
            let src_idx = src_pos as usize;
            let frac = (src_pos - src_idx as f64) as f32;
            let next_idx = (src_idx + 1).min(num_frames - 1);

            for c in 0..ch {
                let off0 = (src_idx * ch + c) * 2;
                let off1 = (next_idx * ch + c) * 2;
                let s0 = i16::from_le_bytes([result[off0], result[off0 + 1]]) as f32;
                let s1 = i16::from_le_bytes([result[off1], result[off1 + 1]]) as f32;
                let s = (s0 + (s1 - s0) * frac) as i16;
                out.extend_from_slice(&s.to_le_bytes());
            }
        }
        result = out;
    }

    result
}

// ---- OSS backend (/dev/dsp) ----

/// OSS ioctl constants.
const SNDCTL_DSP_SPEED: libc::c_int = ioctl_code(2);
const SNDCTL_DSP_SETFMT: libc::c_int = ioctl_code(5);
const SNDCTL_DSP_CHANNELS: libc::c_int = ioctl_code(6);
const AFMT_S16_LE: i32 = 0x10;

const fn ioctl_code(nr: u32) -> libc::c_int {
    ((3u32 << 30) | (4u32 << 16) | (0x50u32 << 8) | nr) as i32
}

fn open_and_configure_oss(info: &WavInfo) -> Result<std::fs::File, String> {
    use std::os::unix::io::AsRawFd;

    let file = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/dsp")
        .map_err(|e| format!("open /dev/dsp: {}", e))?;

    let fd = file.as_raw_fd();

    let mut fmt = AFMT_S16_LE;
    let ret = unsafe { libc::ioctl(fd, SNDCTL_DSP_SETFMT as _, &mut fmt) };
    if ret < 0 {
        eprintln!("[audio] OSS SETFMT failed: {}", std::io::Error::last_os_error());
    }

    let mut channels = info.channels as i32;
    let ret = unsafe { libc::ioctl(fd, SNDCTL_DSP_CHANNELS as _, &mut channels) };
    if ret < 0 {
        eprintln!("[audio] OSS CHANNELS failed: {}", std::io::Error::last_os_error());
    }

    let mut rate = info.sample_rate as i32;
    let ret = unsafe { libc::ioctl(fd, SNDCTL_DSP_SPEED as _, &mut rate) };
    if ret < 0 {
        eprintln!("[audio] OSS SPEED failed: {}", std::io::Error::last_os_error());
    }

    eprintln!("[audio] OSS configured: {}Hz {}ch", rate, channels);
    Ok(file)
}

// ---- ALSA backend (/dev/snd/pcmC0D0p) ----

/// ALSA device configured at a fixed format.
struct AlsaDevice {
    file: std::fs::File,
    sample_rate: u32,
    channels: u16,
}

/// Open and configure ALSA at 48kHz stereo S16_LE (widely supported by HDA codecs).
fn open_and_configure_alsa() -> Result<AlsaDevice, String> {
    use std::os::unix::io::AsRawFd;

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/snd/pcmC0D0p")
        .map_err(|e| format!("open pcm: {}", e))?;

    let fd = file.as_raw_fd();

    // ALSA ioctl numbers
    const SNDRV_PCM_IOCTL_HW_PARAMS: libc::c_int = 0xc2604111u32 as i32;
    const SNDRV_PCM_IOCTL_SW_PARAMS: libc::c_int = 0xc0884113u32 as i32;
    const SNDRV_PCM_IOCTL_PREPARE: libc::c_int = 0x00004140;

    // Fixed device format
    const DEV_RATE: u32 = 48000;
    const DEV_CHANNELS: u32 = 2;
    const DEV_BITS: u32 = 16;
    const DEV_FORMAT: u32 = 2; // S16_LE

    // ---- HW_PARAMS ----

    #[repr(C)]
    struct SndPcmHwParams {
        flags: u32,
        masks: [[u32; 8]; 3],
        mres: [[u32; 8]; 5],
        intervals: [SndInterval; 12],
        ires: [SndInterval; 9],
        rmask: u32,
        cmask: u32,
        info: u32,
        msbits: u32,
        rate_num: u32,
        rate_den: u32,
        fifo_size: u64,
        _reserved: [u8; 64],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct SndInterval {
        min: u32,
        max: u32,
        flags: u32,
    }

    const _: () = assert!(std::mem::size_of::<SndPcmHwParams>() == 608);

    let mut params: SndPcmHwParams = unsafe { std::mem::zeroed() };

    // Active masks: "any" (all bits set); reserved masks stay zeroed
    for mask in &mut params.masks {
        for word in mask.iter_mut() { *word = 0xFFFFFFFF; }
    }
    // Active intervals: unconstrained; reserved intervals stay zeroed
    for interval in &mut params.intervals {
        interval.min = 0;
        interval.max = u32::MAX;
        interval.flags = 0;
    }

    params.info = 0xFFFFFFFF;
    params.rmask = 0xFFFFFFFF;

    // Access: RW_INTERLEAVED only (bit 3)
    params.masks[0] = [0; 8];
    params.masks[0][0] = 1 << 3;

    // Format: S16_LE (bit 2)
    params.masks[1] = [0; 8];
    params.masks[1][0] = 1 << DEV_FORMAT;

    // Subformat: standard (bit 0)
    params.masks[2] = [0; 8];
    params.masks[2][0] = 1;

    // Intervals: constrain to our fixed format
    let frame_bits = DEV_BITS * DEV_CHANNELS;
    params.intervals[0] = SndInterval { min: DEV_BITS, max: DEV_BITS, flags: 0 };         // SAMPLE_BITS
    params.intervals[1] = SndInterval { min: frame_bits, max: frame_bits, flags: 0 };     // FRAME_BITS
    params.intervals[2] = SndInterval { min: DEV_CHANNELS, max: DEV_CHANNELS, flags: 0 }; // CHANNELS
    params.intervals[3] = SndInterval { min: DEV_RATE, max: DEV_RATE, flags: 0 };         // RATE
    params.intervals[5] = SndInterval { min: 256, max: 16384, flags: 0 };                 // PERIOD_SIZE
    params.intervals[7] = SndInterval { min: 2, max: 8, flags: 0 };                       // PERIODS

    let ret = unsafe {
        libc::ioctl(fd, SNDRV_PCM_IOCTL_HW_PARAMS as _, &mut params as *mut SndPcmHwParams)
    };
    if ret < 0 {
        let err = std::io::Error::last_os_error();
        eprintln!("[audio] ALSA HW_PARAMS failed: {}", err);
        return Err(format!("ALSA HW_PARAMS: {}", err));
    }

    // Read back refined values
    let period_size = params.intervals[5].min;
    let buffer_size = params.intervals[5].min * params.intervals[7].min;
    eprintln!(
        "[audio] ALSA HW configured: {}Hz {}ch {}bit, period={} buffer={}",
        DEV_RATE, DEV_CHANNELS, DEV_BITS, period_size, buffer_size
    );

    // ---- SW_PARAMS ----
    // Set start_threshold = buffer_size so DMA doesn't start until buffer is full,
    // preventing immediate underrun (EPIPE/XRUN).

    #[repr(C)]
    struct SndPcmSwParams {
        tstamp_mode: i32,
        period_step: u32,
        sleep_min: u32,
        _pad0: u32,
        avail_min: u64,
        xfer_align: u64,
        start_threshold: u64,
        stop_threshold: u64,
        silence_threshold: u64,
        silence_size: u64,
        boundary: u64,
        proto: u32,
        tstamp_type: u32,
        _reserved: [u8; 56],
    }

    const _SW: () = assert!(std::mem::size_of::<SndPcmSwParams>() == 136);

    let mut sw: SndPcmSwParams = unsafe { std::mem::zeroed() };
    sw.period_step = 1;
    sw.avail_min = period_size as u64;
    sw.xfer_align = 1;
    sw.start_threshold = buffer_size as u64;
    sw.stop_threshold = buffer_size as u64;
    sw.boundary = buffer_size as u64;
    // Compute large boundary (prevents pointer wrap issues)
    while sw.boundary * 2 <= 0x7FFFFFFF {
        sw.boundary *= 2;
    }

    let ret = unsafe {
        libc::ioctl(fd, SNDRV_PCM_IOCTL_SW_PARAMS as _, &mut sw as *mut SndPcmSwParams)
    };
    if ret < 0 {
        let err = std::io::Error::last_os_error();
        eprintln!("[audio] ALSA SW_PARAMS failed: {} (non-fatal)", err);
        // Non-fatal: continue with kernel defaults
    } else {
        eprintln!("[audio] ALSA SW configured: start_threshold={}", buffer_size);
    }

    // ---- PREPARE ----
    let ret = unsafe { libc::ioctl(fd, SNDRV_PCM_IOCTL_PREPARE as _) };
    if ret < 0 {
        let err = std::io::Error::last_os_error();
        eprintln!("[audio] ALSA PREPARE failed: {}", err);
        return Err(format!("ALSA PREPARE: {}", err));
    }

    Ok(AlsaDevice {
        file,
        sample_rate: DEV_RATE,
        channels: DEV_CHANNELS as u16,
    })
}

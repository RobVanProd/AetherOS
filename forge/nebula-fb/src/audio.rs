/// Audio system — WAV playback via raw ALSA ioctls on /dev/snd/pcmC0D0p.
///
/// Provides:
/// - `AudioPlayer::new()` — opens the ALSA device (or logs warning)
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
    // "RIFF" check
    if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }

    // Find "fmt " chunk
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
        // Word-align
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
    /// Volume in 0..1000 (permille). 1000 = full volume.
    volume: Arc<AtomicU32>,
    fade_flag: Arc<AtomicBool>,
    fade_duration_ms: Arc<AtomicU32>,
}

impl PlayHandle {
    /// Start a fade-out over the given duration in milliseconds.
    pub fn fade_out(&self, duration_ms: u32) {
        self.fade_duration_ms.store(duration_ms, Ordering::Relaxed);
        self.fade_flag.store(true, Ordering::Relaxed);
    }

    /// Immediately stop playback.
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }
}

pub struct AudioPlayer {
    available: bool,
}

impl AudioPlayer {
    pub fn new() -> Self {
        // Check if ALSA device exists
        let available = std::path::Path::new("/dev/snd/pcmC0D0p").exists();
        if available {
            eprintln!("[audio] ALSA PCM device found");
        } else {
            eprintln!("[audio] No ALSA device found — audio disabled");
        }
        Self { available }
    }

    /// Play the embedded boot chime (one-shot, fire-and-forget).
    pub fn play_boot_chime(&self) {
        if !self.available {
            return;
        }
        std::thread::spawn(move || {
            if let Err(e) = play_wav_data(BOOT_WAV, false, None) {
                eprintln!("[audio] Boot chime error: {}", e);
            }
        });
    }

    /// Play POST music from filesystem in a loop. Returns a PlayHandle for fade/stop.
    pub fn play_post_music(&self) -> Option<PlayHandle> {
        if !self.available {
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

        std::thread::spawn(move || {
            // Read post.wav from filesystem
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

            // Loop until stopped
            loop {
                if ctrl.as_ref().map_or(false, |c| c.stop_flag.load(Ordering::Relaxed)) {
                    break;
                }
                match play_wav_data(&data, true, ctrl.as_ref()) {
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

/// Low-level WAV playback to /dev/snd/pcmC0D0p using write().
/// Returns Ok(true) if stopped early, Ok(false) if played to completion.
fn play_wav_data(data: &[u8], _looping: bool, ctrl: Option<&PlayControl>) -> Result<bool, String> {
    let info = parse_wav_header(data).ok_or("Invalid WAV header")?;

    eprintln!(
        "[audio] Playing: {}ch {}Hz {}bit, {} bytes of PCM data",
        info.channels, info.sample_rate, info.bits_per_sample, info.data_len
    );

    // Open ALSA device
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/snd/pcmC0D0p")
        .map_err(|e| format!("open pcm: {}", e))?;

    // Configure ALSA via ioctl — use hw_params
    configure_alsa(&file, info.sample_rate, info.channels, info.bits_per_sample)?;

    // Write PCM data in chunks
    let pcm_data = &data[info.data_offset..];
    let actual_len = pcm_data.len().min(info.data_len);
    let chunk_size = (info.sample_rate as usize * info.channels as usize * (info.bits_per_sample as usize / 8)) / 10; // ~100ms chunks
    let chunk_size = chunk_size.max(4096);

    let mut offset = 0;
    let mut fade_start: Option<std::time::Instant> = None;

    while offset < actual_len {
        // Check stop
        if let Some(c) = ctrl {
            if c.stop_flag.load(Ordering::Relaxed) {
                return Ok(true);
            }

            // Handle fade
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

        let end = (offset + chunk_size).min(actual_len);
        let chunk = &pcm_data[offset..end];

        // Apply volume scaling if fading
        let vol = ctrl.map_or(1000, |c| c.volume.load(Ordering::Relaxed));
        if vol < 1000 && info.bits_per_sample == 16 {
            // Scale 16-bit samples in-place via a temporary buffer
            let mut scaled = chunk.to_vec();
            for pair in scaled.chunks_exact_mut(2) {
                let sample = i16::from_le_bytes([pair[0], pair[1]]);
                let scaled_sample = (sample as i32 * vol as i32 / 1000) as i16;
                let bytes = scaled_sample.to_le_bytes();
                pair[0] = bytes[0];
                pair[1] = bytes[1];
            }
            write_all_alsa(&mut file, &scaled)?;
        } else {
            write_all_alsa(&mut file, chunk)?;
        }

        offset = end;
    }

    Ok(false)
}

fn write_all_alsa(file: &mut std::fs::File, data: &[u8]) -> Result<(), String> {
    use std::io::Write;
    file.write_all(data).map_err(|e| format!("pcm write: {}", e))
}

/// Configure ALSA hardware parameters via ioctl.
/// This uses the SNDRV_PCM_IOCTL_HW_PARAMS ioctl to set format, rate, channels.
fn configure_alsa(file: &std::fs::File, sample_rate: u32, channels: u16, bits: u16) -> Result<(), String> {
    use std::os::unix::io::AsRawFd;

    let fd = file.as_raw_fd();

    // ALSA ioctl numbers (cast to Ioctl = c_int on musl)
    const SNDRV_PCM_IOCTL_HW_PARAMS: libc::c_int = 0xc2604111u32 as i32;
    const SNDRV_PCM_IOCTL_PREPARE: libc::c_int = 0x00004140;

    // Format: S16_LE = 2, S24_LE = 6, S32_LE = 10
    let format = match bits {
        16 => 2u32,
        24 => 6u32,
        32 => 10u32,
        _ => 2u32,
    };

    // snd_pcm_hw_params structure — 608 bytes on 64-bit Linux
    // We use a simplified approach: fill the masks and intervals
    #[repr(C)]
    struct SndPcmHwParams {
        flags: u32,
        masks: [[u32; 8]; 3],      // 3 masks: access, format, subformat (256 bits each)
        _reserved_masks: [[u32; 8]; 2],
        intervals: [SndInterval; 12], // 12 intervals
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
        openmin_openmax_integer_empty: u32,
    }

    let mut params: SndPcmHwParams = unsafe { std::mem::zeroed() };

    // Set all masks to "any" (all bits set)
    for mask in &mut params.masks {
        for word in mask.iter_mut() {
            *word = 0xFFFFFFFF;
        }
    }
    for mask in &mut params._reserved_masks {
        for word in mask.iter_mut() {
            *word = 0xFFFFFFFF;
        }
    }

    // Set all intervals to full range initially
    for interval in &mut params.intervals {
        interval.min = 0;
        interval.max = u32::MAX;
        interval.openmin_openmax_integer_empty = 0;
    }

    // Refine mask: access = MMAP_INTERLEAVED (0) | RW_INTERLEAVED (3)
    // Mask index 0 = access
    params.masks[0] = [0; 8];
    params.masks[0][0] = (1 << 0) | (1 << 3); // MMAP_INTERLEAVED | RW_INTERLEAVED

    // Refine mask: format — set only our format bit
    // Mask index 1 = format
    params.masks[1] = [0; 8];
    params.masks[1][(format / 32) as usize] = 1 << (format % 32);

    // Refine mask: subformat — standard (bit 0)
    params.masks[2] = [0; 8];
    params.masks[2][0] = 1;

    // Interval indices:
    // 0 = sample_bits, 1 = frame_bits, 2 = channels, 3 = rate
    // 4 = period_time, 5 = period_size, 6 = period_bytes
    // 7 = periods, 8 = buffer_time, 9 = buffer_size, 10 = buffer_bytes
    // 11 = tick_time

    // Sample bits
    let sample_bits = bits as u32;
    params.intervals[0] = SndInterval { min: sample_bits, max: sample_bits, openmin_openmax_integer_empty: 0 };

    // Frame bits
    let frame_bits = sample_bits * channels as u32;
    params.intervals[1] = SndInterval { min: frame_bits, max: frame_bits, openmin_openmax_integer_empty: 0 };

    // Channels
    params.intervals[2] = SndInterval { min: channels as u32, max: channels as u32, openmin_openmax_integer_empty: 0 };

    // Sample rate
    params.intervals[3] = SndInterval { min: sample_rate, max: sample_rate, openmin_openmax_integer_empty: 0 };

    // Let ALSA figure out period/buffer sizes
    // Period size: suggest ~1024 frames
    params.intervals[5] = SndInterval { min: 256, max: 16384, openmin_openmax_integer_empty: 0 };

    // Periods: 2-8
    params.intervals[7] = SndInterval { min: 2, max: 8, openmin_openmax_integer_empty: 0 };

    // rmask = all bits (refine everything)
    params.rmask = 0xFFFFFFFF;

    let ret = unsafe {
        libc::ioctl(fd, SNDRV_PCM_IOCTL_HW_PARAMS, &mut params as *mut SndPcmHwParams)
    };

    if ret < 0 {
        let errno = std::io::Error::last_os_error();
        eprintln!("[audio] HW_PARAMS ioctl failed: {} (ret={})", errno, ret);
        // Fall back to just writing raw PCM data — some devices accept it
        eprintln!("[audio] Attempting raw write without explicit hw_params...");
    } else {
        eprintln!("[audio] ALSA configured: {}Hz {}ch {}bit", sample_rate, channels, bits);
    }

    // Prepare the device for playback
    let ret = unsafe { libc::ioctl(fd, SNDRV_PCM_IOCTL_PREPARE) };
    if ret < 0 {
        eprintln!("[audio] PREPARE ioctl failed: {}", std::io::Error::last_os_error());
    }

    Ok(())
}

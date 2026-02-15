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
        // Try OSS first (simpler ioctls, more reliable)
        let backend = if std::path::Path::new("/dev/dsp").exists() {
            eprintln!("[audio] OSS device /dev/dsp found");
            AudioBackend::Oss
        } else if std::path::Path::new("/dev/snd/pcmC0D0p").exists() {
            eprintln!("[audio] ALSA device /dev/snd/pcmC0D0p found");
            AudioBackend::Alsa
        } else {
            eprintln!("[audio] No audio device found — audio disabled");
            eprintln!("[audio] (Kernel needs CONFIG_SOUND=y CONFIG_SND_PCM_OSS=y and kernel rebuild)");
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

    let mut file = match backend {
        AudioBackend::Oss => open_and_configure_oss(&info)?,
        AudioBackend::Alsa => open_and_configure_alsa(&info)?,
        AudioBackend::None => return Err("No audio backend".to_string()),
    };

    // Write PCM data in chunks (~100ms each)
    let pcm_data = &data[info.data_offset..];
    let actual_len = pcm_data.len().min(info.data_len);
    let bytes_per_frame = info.channels as usize * (info.bits_per_sample as usize / 8);
    let chunk_size = ((info.sample_rate as usize * bytes_per_frame) / 10).max(4096);

    let mut offset = 0;
    let mut fade_start: Option<std::time::Instant> = None;

    while offset < actual_len {
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

        let end = (offset + chunk_size).min(actual_len);
        let chunk = &pcm_data[offset..end];

        let vol = ctrl.map_or(1000, |c| c.volume.load(Ordering::Relaxed));
        if vol < 1000 && info.bits_per_sample == 16 {
            let mut scaled = chunk.to_vec();
            for pair in scaled.chunks_exact_mut(2) {
                let sample = i16::from_le_bytes([pair[0], pair[1]]);
                let scaled_sample = (sample as i32 * vol as i32 / 1000) as i16;
                let bytes = scaled_sample.to_le_bytes();
                pair[0] = bytes[0];
                pair[1] = bytes[1];
            }
            write_all(&mut file, &scaled)?;
        } else {
            write_all(&mut file, chunk)?;
        }

        offset = end;
    }

    Ok(false)
}

fn write_all(file: &mut std::fs::File, data: &[u8]) -> Result<(), String> {
    use std::io::Write;
    file.write_all(data).map_err(|e| format!("audio write: {}", e))
}

// ---- OSS backend (/dev/dsp) ----

/// OSS ioctl constants.
/// _SIOWR('P', n, int) = (3 << 30) | (4 << 16) | (0x50 << 8) | n
const SNDCTL_DSP_SPEED: libc::c_int = ioctl_code(2); // set sample rate
const SNDCTL_DSP_SETFMT: libc::c_int = ioctl_code(5); // set format
const SNDCTL_DSP_CHANNELS: libc::c_int = ioctl_code(6); // set channels
const AFMT_S16_LE: i32 = 0x10;

const fn ioctl_code(nr: u32) -> libc::c_int {
    // _SIOWR('P', nr, int) = direction(3) << 30 | size(4) << 16 | type('P'=0x50) << 8 | nr
    ((3u32 << 30) | (4u32 << 16) | (0x50u32 << 8) | nr) as i32
}

fn open_and_configure_oss(info: &WavInfo) -> Result<std::fs::File, String> {
    use std::os::unix::io::AsRawFd;

    let file = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/dsp")
        .map_err(|e| format!("open /dev/dsp: {}", e))?;

    let fd = file.as_raw_fd();

    // Set format (S16_LE)
    let mut fmt = AFMT_S16_LE;
    let ret = unsafe { libc::ioctl(fd, SNDCTL_DSP_SETFMT, &mut fmt) };
    if ret < 0 {
        eprintln!("[audio] OSS SETFMT failed: {}", std::io::Error::last_os_error());
    }

    // Set channels
    let mut channels = info.channels as i32;
    let ret = unsafe { libc::ioctl(fd, SNDCTL_DSP_CHANNELS, &mut channels) };
    if ret < 0 {
        eprintln!("[audio] OSS CHANNELS failed: {}", std::io::Error::last_os_error());
    }

    // Set sample rate
    let mut rate = info.sample_rate as i32;
    let ret = unsafe { libc::ioctl(fd, SNDCTL_DSP_SPEED, &mut rate) };
    if ret < 0 {
        eprintln!("[audio] OSS SPEED failed: {}", std::io::Error::last_os_error());
    }

    eprintln!("[audio] OSS configured: {}Hz {}ch", rate, channels);
    Ok(file)
}

// ---- ALSA backend (/dev/snd/pcmC0D0p) ----

fn open_and_configure_alsa(info: &WavInfo) -> Result<std::fs::File, String> {
    use std::os::unix::io::AsRawFd;

    let file = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/snd/pcmC0D0p")
        .map_err(|e| format!("open pcm: {}", e))?;

    let fd = file.as_raw_fd();

    // ALSA ioctl numbers
    const SNDRV_PCM_IOCTL_HW_PARAMS: libc::c_int = 0xc2604111u32 as i32;
    const SNDRV_PCM_IOCTL_PREPARE: libc::c_int = 0x00004140;

    let format: u32 = match info.bits_per_sample {
        16 => 2, // S16_LE
        24 => 6, // S24_LE
        32 => 10, // S32_LE
        _ => 2,
    };

    #[repr(C)]
    struct SndPcmHwParams {
        flags: u32,
        masks: [[u32; 8]; 3],
        _reserved_masks: [[u32; 8]; 2],
        intervals: [SndInterval; 12],
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

    let mut params: SndPcmHwParams = unsafe { std::mem::zeroed() };

    // Masks: set all bits, then refine
    for mask in &mut params.masks {
        for word in mask.iter_mut() { *word = 0xFFFFFFFF; }
    }
    for mask in &mut params._reserved_masks {
        for word in mask.iter_mut() { *word = 0xFFFFFFFF; }
    }
    for interval in &mut params.intervals {
        interval.min = 0;
        interval.max = u32::MAX;
        interval.flags = 0;
    }

    // Access: RW_INTERLEAVED (bit 3)
    params.masks[0] = [0; 8];
    params.masks[0][0] = (1 << 0) | (1 << 3);

    // Format
    params.masks[1] = [0; 8];
    params.masks[1][(format / 32) as usize] = 1 << (format % 32);

    // Subformat: standard
    params.masks[2] = [0; 8];
    params.masks[2][0] = 1;

    let sample_bits = info.bits_per_sample as u32;
    let frame_bits = sample_bits * info.channels as u32;
    params.intervals[0] = SndInterval { min: sample_bits, max: sample_bits, flags: 0 };
    params.intervals[1] = SndInterval { min: frame_bits, max: frame_bits, flags: 0 };
    params.intervals[2] = SndInterval { min: info.channels as u32, max: info.channels as u32, flags: 0 };
    params.intervals[3] = SndInterval { min: info.sample_rate, max: info.sample_rate, flags: 0 };
    params.intervals[5] = SndInterval { min: 256, max: 16384, flags: 0 };
    params.intervals[7] = SndInterval { min: 2, max: 8, flags: 0 };
    params.rmask = 0xFFFFFFFF;

    let ret = unsafe {
        libc::ioctl(fd, SNDRV_PCM_IOCTL_HW_PARAMS, &mut params as *mut SndPcmHwParams)
    };
    if ret < 0 {
        eprintln!("[audio] ALSA HW_PARAMS failed: {}", std::io::Error::last_os_error());
    } else {
        eprintln!("[audio] ALSA configured: {}Hz {}ch {}bit", info.sample_rate, info.channels, info.bits_per_sample);
    }

    let ret = unsafe { libc::ioctl(fd, SNDRV_PCM_IOCTL_PREPARE) };
    if ret < 0 {
        eprintln!("[audio] ALSA PREPARE failed: {}", std::io::Error::last_os_error());
    }

    Ok(file)
}

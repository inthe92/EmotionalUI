//! Small in-process PCM adapters. No decoder or external player in the frontend.
use super::assets::RATE;
#[cfg(target_os = "macos")]
use super::mixer::BLOCK;
use std::time::Duration;
pub trait Output {
    fn write(&mut self, pcm: &[i16]) -> Result<(), String>;
    fn drain(&mut self) {}
}
#[derive(Clone, Copy, Debug)]
pub enum Backend {
    Native,
    Probe,
    Unavailable,
    #[cfg(test)]
    WriteFailure,
}
pub fn open(backend: Backend) -> Result<Box<dyn Output>, String> {
    match backend {
        Backend::Probe => Ok(Box::new(Probe)),
        Backend::Unavailable => Err("No audio device in this test environment".into()),
        Backend::Native => native(),
        #[cfg(test)]
        Backend::WriteFailure => Ok(Box::new(BrokenOutput)),
    }
}
struct Probe;
#[cfg(test)]
struct BrokenOutput;
#[cfg(test)]
impl Output for BrokenOutput {
    fn write(&mut self, _: &[i16]) -> Result<(), String> {
        Err("Simulated device disconnected during write".into())
    }
}
impl Output for Probe {
    fn write(&mut self, pcm: &[i16]) -> Result<(), String> {
        std::thread::sleep(Duration::from_secs_f64(pcm.len() as f64 / f64::from(RATE)));
        Ok(())
    }
}
#[cfg(target_os = "linux")]
fn native() -> Result<Box<dyn Output>, String> {
    Ok(Box::new(linux::Alsa::open()?))
}
#[cfg(target_os = "macos")]
fn native() -> Result<Box<dyn Output>, String> {
    Ok(Box::new(mac::Queue::open()?))
}
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn native() -> Result<Box<dyn Output>, String> {
    Err("Audio backend not implemented on this platform".into())
}
#[cfg(any(target_os = "linux", test))]
fn double_rate(input: &[i16], output: &mut Vec<i16>, previous: &mut i16) {
    output.clear();
    for &sample in input {
        output.push(((i32::from(*previous) + i32::from(sample)) / 2) as i16);
        output.push(sample);
        *previous = sample;
    }
}
#[cfg(test)]
mod rate_tests {
    use super::double_rate;
    #[test]
    fn fallback_preserves_duration_extremes_and_block_continuity() {
        let samples = [i16::MIN, 0, i16::MAX, 1200, -1200];
        let mut whole = Vec::new();
        double_rate(&samples, &mut whole, &mut 0);
        let mut previous = 0;
        let mut first = Vec::new();
        double_rate(&samples[..2], &mut first, &mut previous);
        let mut second = Vec::new();
        double_rate(&samples[2..], &mut second, &mut previous);
        first.extend(second);
        assert_eq!(first, whole);
        assert_eq!(whole.len(), 2 * samples.len());
        assert_eq!(whole.iter().step_by(2).count(), samples.len());
        assert_eq!(
            whole.iter().skip(1).step_by(2).copied().collect::<Vec<_>>(),
            samples
        );
    }
}
#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::{
        ffi::{c_char, c_int, c_uint, c_void},
        time::Instant,
    };
    type Open = unsafe extern "C" fn(*mut *mut c_void, *const c_char, c_int, c_int) -> c_int;
    type Set =
        unsafe extern "C" fn(*mut c_void, c_int, c_int, c_uint, c_uint, c_int, c_uint) -> c_int;
    type Write = unsafe extern "C" fn(*mut c_void, *const c_void, libc::c_ulong) -> libc::c_long;
    type Recover = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> c_int;
    type Wait = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
    type Close = unsafe extern "C" fn(*mut c_void) -> c_int;
    pub struct Alsa {
        library: *mut c_void,
        pcm: *mut c_void,
        write: Write,
        recover: Recover,
        wait: Wait,
        drop_pcm: Close,
        close: Close,
        doubled: bool,
        converted: Vec<i16>,
        previous: i16,
    }
    impl Alsa {
        pub fn open() -> Result<Self, String> {
            unsafe {
                let lib = libc::dlopen(
                    c"libasound.so.2".as_ptr(),
                    libc::RTLD_NOW | libc::RTLD_LOCAL,
                );
                if lib.is_null() {
                    return Err("ALSA runtime library is not available".into());
                }
                // Symbols are checked before transmuting exact signatures from the ALSA API.
                macro_rules! symbol {
                    ($name:literal,$ty:ty) => {{
                        let p = libc::dlsym(lib, concat!($name, "\0").as_ptr().cast());
                        if p.is_null() {
                            libc::dlclose(lib);
                            return Err(format!("Missing ALSA symbol {}", $name));
                        }
                        std::mem::transmute::<*mut c_void, $ty>(p)
                    }};
                }
                let open = symbol!("snd_pcm_open", Open);
                let set = symbol!("snd_pcm_set_params", Set);
                let write = symbol!("snd_pcm_writei", Write);
                let recover = symbol!("snd_pcm_recover", Recover);
                let wait = symbol!("snd_pcm_wait", Wait);
                let drop_pcm = symbol!("snd_pcm_drop", Close);
                let close = symbol!("snd_pcm_close", Close);
                let mut pcm = std::ptr::null_mut();
                let code = open(&mut pcm, c"default".as_ptr(), 0, 1);
                if code < 0 {
                    libc::dlclose(lib);
                    return Err(format!("ALSA output unavailable ({code})"));
                }
                // S16_LE=2, RW_INTERLEAVED=3; mono with a 60ms ALSA buffer target.
                let mut code = set(pcm, 2, 3, 1, RATE, 1, 60_000);
                let mut doubled = false;
                // The verified dArkOS softvol/dmix chain rejects 22050 Hz periods.
                // Reopen before a bounded 44100 Hz fallback; no mixer/system changes.
                if code == -libc::EINVAL {
                    close(pcm);
                    code = open(&mut pcm, c"default".as_ptr(), 0, 1);
                    if code >= 0 {
                        code = set(pcm, 2, 3, 1, RATE * 2, 1, 100_000);
                        doubled = true;
                    } else {
                        libc::dlclose(lib);
                        return Err(format!("ALSA fallback open failed ({code})"));
                    }
                }
                if code < 0 {
                    close(pcm);
                    libc::dlclose(lib);
                    return Err(format!("ALSA PCM configuration failed ({code})"));
                }
                Ok(Self {
                    library: lib,
                    pcm,
                    write,
                    recover,
                    wait,
                    drop_pcm,
                    close,
                    doubled,
                    converted: Vec::with_capacity(1024),
                    previous: 0,
                })
            }
        }
    }
    impl Output for Alsa {
        fn write(&mut self, pcm: &[i16]) -> Result<(), String> {
            let pcm = if self.doubled {
                double_rate(pcm, &mut self.converted, &mut self.previous);
                self.converted.as_slice()
            } else {
                pcm
            };
            let start = Instant::now();
            let mut at = 0;
            while at < pcm.len() {
                if start.elapsed() > Duration::from_millis(200) {
                    return Err("ALSA write deadline exceeded".into());
                }
                let n = unsafe {
                    (self.write)(
                        self.pcm,
                        pcm[at..].as_ptr().cast(),
                        (pcm.len() - at) as libc::c_ulong,
                    )
                };
                if n > 0 {
                    at += n as usize;
                } else if n == -(libc::EAGAIN as libc::c_long) {
                    unsafe {
                        (self.wait)(self.pcm, 10);
                    }
                } else if n < 0 {
                    if unsafe { (self.recover)(self.pcm, n as c_int, 1) } < 0 {
                        return Err(format!("ALSA stream error ({n})"));
                    }
                } else {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            Ok(())
        }
        fn drain(&mut self) {
            std::thread::sleep(Duration::from_millis(if self.doubled { 105 } else { 65 }));
        }
    }
    impl Drop for Alsa {
        fn drop(&mut self) {
            unsafe {
                (self.drop_pcm)(self.pcm);
                (self.close)(self.pcm);
                libc::dlclose(self.library);
            }
        }
    }
    // Avoid importing/linking ALSA headers and development packages into the toolchain.
}
#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use std::{
        ffi::c_void,
        sync::mpsc::{self, Receiver, SyncSender},
        time::Instant,
    };
    #[repr(C)]
    struct Format {
        rate: f64,
        id: u32,
        flags: u32,
        bytes_packet: u32,
        frames_packet: u32,
        bytes_frame: u32,
        channels: u32,
        bits: u32,
        reserved: u32,
    }
    // Prefix of AudioQueueBuffer: storage is allocated/owned by AudioToolbox, never by Rust.
    #[repr(C)]
    struct Buffer {
        capacity: u32,
        data: *mut c_void,
        size: u32,
    }
    #[link(name = "AudioToolbox", kind = "framework")]
    unsafe extern "C" {
        fn AudioQueueNewOutput(
            format: *const Format,
            callback: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut Buffer),
            data: *mut c_void,
            runloop: *mut c_void,
            mode: *mut c_void,
            flags: u32,
            out: *mut *mut c_void,
        ) -> i32;
        fn AudioQueueAllocateBuffer(queue: *mut c_void, bytes: u32, out: *mut *mut Buffer) -> i32;
        fn AudioQueueEnqueueBuffer(
            queue: *mut c_void,
            buffer: *mut Buffer,
            n: u32,
            descriptions: *mut c_void,
        ) -> i32;
        fn AudioQueueStart(queue: *mut c_void, when: *const c_void) -> i32;
        fn AudioQueueStop(queue: *mut c_void, immediate: u8) -> i32;
        fn AudioQueueDispose(queue: *mut c_void, immediate: u8) -> i32;
        fn AudioQueueGetProperty(
            queue: *mut c_void,
            id: u32,
            data: *mut c_void,
            size: *mut u32,
        ) -> i32;
    }
    unsafe extern "C" fn returned(data: *mut c_void, _queue: *mut c_void, buffer: *mut Buffer) {
        // The callback only returns a pointer to a bounded queue; no allocation or decoding.
        let tx = unsafe { &*(data as *const SyncSender<usize>) };
        let _ = tx.try_send(buffer as usize);
    }
    pub struct Queue {
        queue: *mut c_void,
        context: *mut SyncSender<usize>,
        free: Receiver<usize>,
        queued: u8,
        started: bool,
    }
    impl Queue {
        pub fn open() -> Result<Self, String> {
            unsafe {
                let (tx, rx) = mpsc::sync_channel(3);
                let context = Box::into_raw(Box::new(tx));
                let mut queue = std::ptr::null_mut();
                let f = Format {
                    rate: f64::from(RATE),
                    id: u32::from_be_bytes(*b"lpcm"),
                    flags: 12,
                    bytes_packet: 2,
                    frames_packet: 1,
                    bytes_frame: 2,
                    channels: 1,
                    bits: 16,
                    reserved: 0,
                };
                let status = AudioQueueNewOutput(
                    &f,
                    returned,
                    context.cast(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    0,
                    &mut queue,
                );
                if status != 0 {
                    drop(Box::from_raw(context));
                    return Err(format!("CoreAudio output unavailable ({status})"));
                }
                let owner = Self {
                    queue,
                    context,
                    free: rx,
                    queued: 0,
                    started: false,
                };
                for _ in 0..3 {
                    let mut buffer = std::ptr::null_mut();
                    let status = AudioQueueAllocateBuffer(queue, (BLOCK * 2) as u32, &mut buffer);
                    if status != 0 {
                        return Err(format!("CoreAudio buffer allocation failed ({status})"));
                    }
                    (*context)
                        .try_send(buffer as usize)
                        .map_err(|e| e.to_string())?;
                }
                Ok(owner)
            }
        }
    }
    impl Output for Queue {
        fn write(&mut self, pcm: &[i16]) -> Result<(), String> {
            let buffer = self
                .free
                .recv_timeout(Duration::from_millis(200))
                .map_err(|_| "CoreAudio buffer deadline exceeded")?
                as *mut Buffer;
            unsafe {
                if pcm.len() * 2 > (*buffer).capacity as usize {
                    return Err("PCM block exceeds CoreAudio buffer".into());
                }
                std::ptr::copy_nonoverlapping(pcm.as_ptr(), (*buffer).data.cast(), pcm.len());
                (*buffer).size = (pcm.len() * 2) as u32;
                let status = AudioQueueEnqueueBuffer(self.queue, buffer, 0, std::ptr::null_mut());
                if status != 0 {
                    return Err(format!("CoreAudio enqueue failed ({status})"));
                }
                self.queued += u8::from(self.queued < 3);
                if !self.started && self.queued == 3 {
                    let status = AudioQueueStart(self.queue, std::ptr::null());
                    if status != 0 {
                        return Err(format!("CoreAudio start failed ({status})"));
                    }
                    self.started = true;
                }
            }
            Ok(())
        }
        fn drain(&mut self) {
            unsafe {
                if !self.started && self.queued > 0 {
                    let _ = AudioQueueStart(self.queue, std::ptr::null());
                    self.started = true;
                }
                let _ = AudioQueueStop(self.queue, 0);
                let start = Instant::now();
                loop {
                    let mut running = 0u32;
                    let mut size = 4;
                    let code = AudioQueueGetProperty(
                        self.queue,
                        u32::from_be_bytes(*b"aqrn"),
                        (&mut running as *mut u32).cast(),
                        &mut size,
                    );
                    if code != 0 || running == 0 || start.elapsed() > Duration::from_millis(200) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(4));
                }
            }
        }
    }
    impl Drop for Queue {
        fn drop(&mut self) {
            unsafe {
                let _ = AudioQueueStop(self.queue, 1);
                let _ = AudioQueueDispose(self.queue, 1);
                drop(Box::from_raw(self.context));
            }
        }
    }
}

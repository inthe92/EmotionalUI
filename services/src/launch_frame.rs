//! Optional, bounded fbdev fallback during a process-exit graphics handoff.
//! No DRM master, modeset, pan, driver changes or background drawing process.
#[cfg(any(target_os = "linux", test))]
use std::io;
pub const FRAME_BYTES: usize = 640 * 480 * 4;

#[derive(Clone, Debug, PartialEq)]
#[cfg(any(target_os = "linux", test))]
struct Layout {
    stride: usize,
    virtual_height: usize,
    offsets: [u32; 3],
    alpha: bool,
}
#[cfg(any(target_os = "linux", test))]
impl Layout {
    fn encode(&self, rgba: &[u8], target: &mut [u8]) -> io::Result<()> {
        if rgba.len() != FRAME_BYTES
            || self.stride < 640 * 4
            || self.stride > 4096
            || ![480, 960].contains(&self.virtual_height)
            || target.len() != self.stride * self.virtual_height
            || !matches!(self.offsets, [16, 8, 0] | [0, 8, 16])
        {
            return Err(io::Error::other("Unsupported bounded launch framebuffer"));
        }
        for y in 0..self.virtual_height {
            let row = &rgba[(y % 480) * 640 * 4..(y % 480 + 1) * 640 * 4];
            let out = &mut target[y * self.stride..y * self.stride + 640 * 4];
            for (pixel, destination) in row.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
                let value = (u32::from(pixel[0]) << self.offsets[0])
                    | (u32::from(pixel[1]) << self.offsets[1])
                    | (u32::from(pixel[2]) << self.offsets[2])
                    | if self.alpha { 0xff00_0000 } else { 0 };
                destination.copy_from_slice(&value.to_ne_bytes());
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod native {
    use super::*;
    use std::{
        fs::{File, OpenOptions},
        io::{Read, Seek, SeekFrom, Write},
        os::{
            fd::AsRawFd,
            raw::c_ulong,
            unix::fs::{FileTypeExt, OpenOptionsExt},
        },
    };
    // Linux UAPI fb.h, GET requests only. ABI layouts, no kernel implementation.
    #[repr(C)]
    #[derive(Default)]
    struct Fixed {
        id: [u8; 16],
        smem_start: c_ulong,
        smem_len: u32,
        kind: u32,
        kind_aux: u32,
        visual: u32,
        xpanstep: u16,
        ypanstep: u16,
        ywrapstep: u16,
        line_length: u32,
        mmio_start: c_ulong,
        mmio_len: u32,
        accel: u32,
        capabilities: u16,
        reserved: [u16; 2],
    }
    fn layout(file: &File) -> io::Result<Layout> {
        let mut fixed = Fixed::default();
        // fb_var_screeninfo is forty u32 fields, including its four reserves.
        let mut variable = [0_u32; 40];
        // SAFETY: writable, aligned UAPI-size buffers; fd remains open throughout.
        if unsafe { libc::ioctl(file.as_raw_fd(), 0x4602, &mut fixed) } != 0
            || unsafe { libc::ioctl(file.as_raw_fd(), 0x4600, variable.as_mut_ptr()) } != 0
        {
            return Err(io::Error::last_os_error());
        }
        if fixed.kind != 0
            || fixed.visual != 2
            || variable[0] != 640
            || variable[1] != 480
            || variable[2] != 640
            || ![480, 960].contains(&variable[3])
            || variable[4] != 0
            || ![0, 480].contains(&variable[5])
            || variable[5] + 480 > variable[3]
            || variable[6] != 32
            || variable[7] != 0
            || variable[9] != 8
            || variable[12] != 8
            || variable[15] != 8
            || variable[10] != 0
            || variable[13] != 0
            || variable[16] != 0
            || ![0, 8].contains(&variable[18])
            || variable[19] != 0
            || (variable[18] == 8 && variable[17] != 24)
            || variable[20] != 0
            || variable[34] != 0
            || fixed.line_length < 2560
            || fixed.line_length > 4096
            || fixed.line_length * variable[3] > fixed.smem_len
        {
            return Err(io::Error::other(
                "Unverified framebuffer layout; original path retained",
            ));
        }
        let result = Layout {
            stride: fixed.line_length as usize,
            virtual_height: variable[3] as usize,
            offsets: [variable[8], variable[11], variable[14]],
            alpha: variable[18] == 8,
        };
        if !matches!(result.offsets, [16, 8, 0] | [0, 8, 16]) {
            return Err(io::Error::other("Unverified framebuffer channel order"));
        }
        Ok(result)
    }
    pub struct Held {
        file: File,
        layout: Layout,
        previous: Vec<u8>,
    }
    impl Held {
        pub fn show(rgba: &[u8]) -> io::Result<Self> {
            if rgba.len() != FRAME_BYTES {
                return Err(io::Error::other("Invalid launch frame"));
            }
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open("/dev/fb0")?;
            if !file.metadata()?.file_type().is_char_device() {
                return Err(io::Error::other("Expected framebuffer character device"));
            }
            let layout = layout(&file)?;
            let mut previous = vec![0; layout.stride * layout.virtual_height];
            file.read_exact(&mut previous)?;
            let mut pixels = previous.clone();
            layout.encode(rgba, &mut pixels)?;
            let mut held = Self {
                file,
                layout,
                previous,
            };
            held.file.seek(SeekFrom::Start(0))?;
            held.file.write_all(&pixels)?;
            Ok(held)
        }
    }
    impl Drop for Held {
        fn drop(&mut self) {
            // Called only after a confirmed child exit (or before a failed spawn).
            // Do not write stale bytes into a framebuffer whose mode has changed.
            if layout(&self.file).is_ok_and(|current| current == self.layout) {
                let _ = self
                    .file
                    .seek(SeekFrom::Start(0))
                    .and_then(|_| self.file.write_all(&self.previous));
            }
        }
    }
}
#[cfg(target_os = "linux")]
pub use native::Held;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_virtual_pages_show_the_same_frame_without_touching_stride_padding() {
        let layout = Layout {
            stride: 2568,
            virtual_height: 960,
            offsets: [16, 8, 0],
            alpha: true,
        };
        let mut source = vec![0; FRAME_BYTES];
        source[..4].copy_from_slice(&[10, 20, 30, 255]);
        let mut target = vec![77; layout.stride * layout.virtual_height];
        layout.encode(&source, &mut target).unwrap();
        let expected = 0xff0a141e_u32.to_ne_bytes();
        assert_eq!(&target[..4], &expected);
        assert_eq!(
            &target[480 * layout.stride..480 * layout.stride + 4],
            &expected
        );
        assert_eq!(&target[2560..2568], &[77; 8]);
    }
    #[test]
    fn unsupported_geometry_or_truncated_data_is_rejected_before_writing() {
        let mut target = vec![77; 2560 * 480];
        let before = target.clone();
        for layout in [
            Layout {
                stride: 2560,
                virtual_height: 480,
                offsets: [16, 0, 8],
                alpha: false,
            },
            Layout {
                stride: 2560,
                virtual_height: 1024,
                offsets: [16, 8, 0],
                alpha: false,
            },
        ] {
            assert!(layout.encode(&vec![0; FRAME_BYTES], &mut target).is_err());
            assert_eq!(target, before);
        }
        let layout = Layout {
            stride: 2560,
            virtual_height: 480,
            offsets: [16, 8, 0],
            alpha: false,
        };
        assert!(layout.encode(&[], &mut target).is_err());
        assert_eq!(target, before);
    }
}

use std::fmt;
use std::fs::File;
use std::io::{Error, ErrorKind, Result};
use std::os::fd::FromRawFd;
use std::os::raw::{c_int, c_ulong};
use std::os::unix::io::AsRawFd;
use std::path::Path;

const FS_IOC_FIEMAP: c_ulong = 0xC020660B;
const PAGESIZE: usize = 8;

unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

#[derive(Debug)]
pub struct Fiemap {
    _file: File,
    fd: c_int,
    fiemap: C_fiemap,
    cur_idx: usize,
    size: u32,
    ended: bool,
}

/// Get fiemap for the path and return an iterator of extents.
///
/// Same as [`Fiemap::new_from_path`].
pub fn fiemap<P: AsRef<Path>>(filepath: P) -> Result<Fiemap> {
    Fiemap::new_from_path(filepath)
}

impl Fiemap {
    /// Creates a new [`Self`] from any type that implements [`AsFd`].
    ///
    /// The lifetime of the underlying file is tied to the lifetime of
    /// the [`Self`] instance, as the file descriptor will be closed
    /// after the instance of [`Self`] is dropped.
    pub fn new(fd: impl AsRawFd) -> Self {
        let raw_fd = fd.as_raw_fd();
        let file = unsafe { File::from_raw_fd(raw_fd) };

        Self {
            _file: file,
            fd: raw_fd,
            fiemap: C_fiemap::new(),
            cur_idx: 0,
            size: 0,
            ended: false,
        }
    }

    /// Creates a new [`Self`] from a file path, opening the file in
    /// read-only mode. See [`std::fs::File::open`] and [`Self::new`].
    pub fn new_from_path(filepath: impl AsRef<Path>) -> Result<Fiemap> {
        let file = File::open(filepath)?;

        Ok(Self::new(file))
    }

    fn get_extents(&mut self) -> Result<()> {
        let req = &mut self.fiemap;
        if self.size != 0 {
            let last = req.fm_extents[self.size as usize - 1];
            req.fm_start = last.fe_logical + last.fe_length;
        }

        let rc = unsafe { ioctl(self.fd, FS_IOC_FIEMAP, req as *mut _) };
        if rc != 0 {
            Err(Error::last_os_error())
        } else {
            self.cur_idx = 0;
            self.size = req.fm_mapped_extents;
            if req.fm_mapped_extents == 0
                || req.fm_extents[req.fm_mapped_extents as usize - 1]
                    .fe_flags
                    .contains(FiemapExtentFlags::LAST)
            {
                self.ended = true;
            }
            Ok(())
        }
    }
}

impl Iterator for Fiemap {
    type Item = Result<FiemapExtent>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur_idx >= self.size as usize {
            if self.ended {
                return None;
            }

            while let Err(e) = self.get_extents() {
                if e.kind() == ErrorKind::Interrupted {
                    continue;
                }
                self.ended = true;
                return Some(Err(e));
            }

            if self.size == 0 {
                // we didn't get any more extents
                return None;
            }
        }

        let idx = self.cur_idx;
        self.cur_idx += 1;
        Some(Ok(self.fiemap.fm_extents[idx]))
    }
}

#[derive(Debug)]
#[repr(C)]
struct C_fiemap {
    fm_start: u64,
    fm_length: u64,
    fm_flags: u32,
    fm_mapped_extents: u32,
    fm_extent_count: u32,
    fm_reserved: u32,
    fm_extents: [FiemapExtent; PAGESIZE],
}

impl C_fiemap {
    fn new() -> Self {
        Self {
            fm_start: 0,
            fm_length: u64::MAX,
            fm_flags: 0,
            fm_mapped_extents: 0,
            fm_extent_count: PAGESIZE as u32,
            fm_reserved: 0,
            fm_extents: [FiemapExtent::new(); PAGESIZE],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FiemapExtent {
    pub fe_logical: u64,
    pub fe_physical: u64,
    pub fe_length: u64,
    fe_reserved64: [u64; 2],
    pub fe_flags: FiemapExtentFlags,
    fe_reserved: [u32; 3],
}

impl FiemapExtent {
    fn new() -> Self {
        Self {
            fe_logical: 0,
            fe_physical: 0,
            fe_length: 0,
            fe_reserved64: [0; 2],
            fe_flags: FiemapExtentFlags::empty(),
            fe_reserved: [0; 3],
        }
    }
}

impl fmt::Debug for FiemapExtent {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        f.debug_struct("FiemapExtent")
            .field("fe_logical", &self.fe_logical)
            .field("fe_physical", &self.fe_physical)
            .field("fe_length", &self.fe_length)
            .field("fe_flags", &self.fe_flags)
            .finish()
    }
}

bitflags::bitflags! {
  #[derive(Copy, Clone, Debug)]
  pub struct FiemapExtentFlags: u32 {
    #[doc = "Last extent in file."]
    const LAST           = 0x00000001;
    #[doc = "Data location unknown."]
    const UNKNOWN        = 0x00000002;
    #[doc = "Location still pending. Sets EXTENT_UNKNOWN."]
    const DELALLOC       = 0x00000004;
    #[doc = "Data can not be read while fs is unmounted"]
    const ENCODED        = 0x00000008;
    #[doc = "Data is encrypted by fs. Sets EXTENT_NO_BYPASS."]
    const DATA_ENCRYPTED = 0x00000080;
    #[doc = "Extent offsets may not be block aligned."]
    const NOT_ALIGNED    = 0x00000100;
    #[doc = "Data mixed with metadata. Sets EXTENT_NOT_ALIGNED."]
    const DATA_INLINE    = 0x00000200;
    #[doc = "Multiple files in block. Sets EXTENT_NOT_ALIGNED."]
    const DATA_TAIL      = 0x00000400;
    #[doc = "Space allocated, but no data (i.e. zero)."]
    const UNWRITTEN      = 0x00000800;
    #[doc = "File does not natively support extents. Result merged for efficiency."]
    const MERGED         = 0x00001000;
    #[doc = "Space shared with other files."]
    const SHARED         = 0x00002000;
  }
}

#![no_std]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/sevki/jetstream/main/logo/JetStream.png"
)]

extern crate kani;

//! Platform-specific libc bindings for jetstream
//!
//! This crate provides a unified interface to libc constants across different platforms,
//! including wasm32 targets where some constants may not be available.

extern crate libc;

// Common types
#[cfg(not(target_arch = "wasm32"))]
pub use libc::{c_int, mode_t};

#[cfg(target_arch = "wasm32")]
pub use core::ffi::c_int;

#[cfg(target_arch = "wasm32")]
#[allow(non_camel_case_types)]
pub type mode_t = u32;

// Platform-specific re-exports and definitions
#[cfg(not(target_arch = "wasm32"))]
mod platform {

    pub use libc::{
        // Stat structure
        stat64,
        EADDRINUSE,
        EADDRNOTAVAIL,
        ECONNABORTED,
        ECONNREFUSED,
        ECONNRESET,
        EEXIST,
        EINTR,
        // Error constants
        EINVAL,
        EIO,
        ENOENT,
        ENOTCONN,
        EPERM,
        EPIPE,
        ETIMEDOUT,
        EWOULDBLOCK,

        O_APPEND,
        O_CREAT,
        O_DSYNC,
        O_EXCL,
        O_NOCTTY,
        O_NONBLOCK,
        // Open flags
        O_RDONLY,
        O_RDWR,
        O_SYNC,

        O_TRUNC,
        O_WRONLY,
        S_IFDIR,
        S_IFLNK,

        // File mode constants
        S_IFMT,
        S_IFREG,
    };

    // Linux-specific flags
    #[cfg(target_os = "linux")]
    pub use libc::{O_DIRECT, O_DIRECTORY, O_LARGEFILE, O_NOATIME, O_NOFOLLOW};

    // Provide default values for non-Linux platforms
    #[cfg(not(target_os = "linux"))]
    pub const O_DIRECT: c_int = 0o40000;
    #[cfg(not(target_os = "linux"))]
    pub const O_LARGEFILE: c_int = 0o100000;
    #[cfg(not(target_os = "linux"))]
    pub const O_DIRECTORY: c_int = 0o200000;
    #[cfg(not(target_os = "linux"))]
    pub const O_NOFOLLOW: c_int = 0o400000;
    #[cfg(not(target_os = "linux"))]
    pub const O_NOATIME: c_int = 0o1000000;
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use super::c_int;

    // WASI error constants - these match the standard POSIX values
    pub const EPERM: c_int = 1;
    pub const ENOENT: c_int = 2;
    pub const EIO: c_int = 5;
    pub const EEXIST: c_int = 17;
    pub const EINVAL: c_int = 22;
    pub const EPIPE: c_int = 32;
    pub const EWOULDBLOCK: c_int = 35;
    pub const EADDRINUSE: c_int = 98;
    pub const EADDRNOTAVAIL: c_int = 99;
    pub const ECONNABORTED: c_int = 103;
    pub const ECONNRESET: c_int = 104;
    pub const ENOTCONN: c_int = 107;
    pub const ETIMEDOUT: c_int = 110;
    pub const ECONNREFUSED: c_int = 111;
    pub const EINTR: c_int = 4;

    // File mode constants
    pub const S_IFMT: u32 = 0o170000;
    pub const S_IFDIR: u32 = 0o040000;
    pub const S_IFREG: u32 = 0o100000;
    pub const S_IFLNK: u32 = 0o120000;

    // Open flags - WASI standard values
    pub const O_RDONLY: c_int = 0;
    pub const O_WRONLY: c_int = 1;
    pub const O_RDWR: c_int = 2;
    pub const O_CREAT: c_int = 0o100;
    pub const O_EXCL: c_int = 0o200;
    pub const O_NOCTTY: c_int = 0o400;
    pub const O_TRUNC: c_int = 0o1000;
    pub const O_APPEND: c_int = 0o2000;
    pub const O_NONBLOCK: c_int = 0o4000;
    pub const O_DSYNC: c_int = 0o10000;
    pub const O_SYNC: c_int = 0o4010000;

    // Linux-specific flags (provide as constants for compatibility)
    pub const O_DIRECT: c_int = 0o40000;
    pub const O_LARGEFILE: c_int = 0o100000;
    pub const O_DIRECTORY: c_int = 0o200000;
    pub const O_NOFOLLOW: c_int = 0o400000;
    pub const O_NOATIME: c_int = 0o1000000;

    // Minimal stat structure for wasm32
    #[repr(C)]
    pub struct stat64 {
        pub st_dev: u64,
        pub st_ino: u64,
        pub st_mode: u32,
        pub st_nlink: u32,
        pub st_uid: u32,
        pub st_gid: u32,
        pub st_rdev: u64,
        pub st_size: i64,
        pub st_blksize: i64,
        pub st_blocks: i64,
        pub st_atime: i64,
        pub st_atime_nsec: i64,
        pub st_mtime: i64,
        pub st_mtime_nsec: i64,
        pub st_ctime: i64,
        pub st_ctime_nsec: i64,
    }
}

// Re-export everything from the platform module
pub use platform::*;

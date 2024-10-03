// Copyright 2018 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub mod messages;
pub mod ninep_2000_l;

use std::io;

pub use self::messages::*;

fn error_to_rmessage(err: &io::Error) -> Rlerror {
    let errno = if let Some(errno) = err.raw_os_error() {
        errno
    } else {
        // Make a best-effort guess based on the kind.
        match err.kind() {
            io::ErrorKind::NotFound => libc::ENOENT,
            io::ErrorKind::PermissionDenied => libc::EPERM,
            io::ErrorKind::ConnectionRefused => libc::ECONNREFUSED,
            io::ErrorKind::ConnectionReset => libc::ECONNRESET,
            io::ErrorKind::ConnectionAborted => libc::ECONNABORTED,
            io::ErrorKind::NotConnected => libc::ENOTCONN,
            io::ErrorKind::AddrInUse => libc::EADDRINUSE,
            io::ErrorKind::AddrNotAvailable => libc::EADDRNOTAVAIL,
            io::ErrorKind::BrokenPipe => libc::EPIPE,
            io::ErrorKind::AlreadyExists => libc::EEXIST,
            io::ErrorKind::WouldBlock => libc::EWOULDBLOCK,
            io::ErrorKind::InvalidInput => libc::EINVAL,
            io::ErrorKind::InvalidData => libc::EINVAL,
            io::ErrorKind::TimedOut => libc::ETIMEDOUT,
            io::ErrorKind::WriteZero => libc::EIO,
            io::ErrorKind::Interrupted => libc::EINTR,
            io::ErrorKind::Other => libc::EIO,
            io::ErrorKind::UnexpectedEof => libc::EIO,
            _ => libc::EIO,
        }
    };

    Rlerror {
        ecode: errno as u32,
    }
}

// Tlopen and Tlcreate flags.  Taken from "include/net/9p/9p.h" in the linux tree.
pub const P9_RDONLY: u32 = 0o00000000;
pub const P9_WRONLY: u32 = 0o00000001;
pub const P9_RDWR: u32 = 0o00000002;
pub const P9_NOACCESS: u32 = 0o00000003;
pub const P9_CREATE: u32 = 0o00000100;
pub const P9_EXCL: u32 = 0o00000200;
pub const P9_NOCTTY: u32 = 0o00000400;
pub const P9_TRUNC: u32 = 0o00001000;
pub const P9_APPEND: u32 = 0o00002000;
pub const P9_NONBLOCK: u32 = 0o00004000;
pub const P9_DSYNC: u32 = 0o00010000;
pub const P9_FASYNC: u32 = 0o00020000;
pub const P9_DIRECT: u32 = 0o00040000;
pub const P9_LARGEFILE: u32 = 0o00100000;
pub const P9_DIRECTORY: u32 = 0o00200000;
pub const P9_NOFOLLOW: u32 = 0o00400000;
pub const P9_NOATIME: u32 = 0o01000000;
pub const _P9_CLOEXEC: u32 = 0o02000000;
pub const P9_SYNC: u32 = 0o04000000;

// Mapping from 9P flags to libc flags.
pub const MAPPED_FLAGS: [(u32, i32); 16] = [
    (P9_WRONLY, libc::O_WRONLY),
    (P9_RDWR, libc::O_RDWR),
    (P9_CREATE, libc::O_CREAT),
    (P9_EXCL, libc::O_EXCL),
    (P9_NOCTTY, libc::O_NOCTTY),
    (P9_TRUNC, libc::O_TRUNC),
    (P9_APPEND, libc::O_APPEND),
    (P9_NONBLOCK, libc::O_NONBLOCK),
    (P9_DSYNC, libc::O_DSYNC),
    (P9_FASYNC, 0), // Unsupported
    (P9_DIRECT, libc::O_DIRECT),
    (P9_LARGEFILE, libc::O_LARGEFILE),
    (P9_DIRECTORY, libc::O_DIRECTORY),
    (P9_NOFOLLOW, libc::O_NOFOLLOW),
    (P9_NOATIME, libc::O_NOATIME),
    (P9_SYNC, libc::O_SYNC),
];

// 9P Qid types.  Taken from "include/net/9p/9p.h" in the linux tree.
pub const P9_QTDIR: u8 = 0x80;
pub const _P9_QTAPPEND: u8 = 0x40;
pub const _P9_QTEXCL: u8 = 0x20;
pub const _P9_QTMOUNT: u8 = 0x10;
pub const _P9_QTAUTH: u8 = 0x08;
pub const _P9_QTTMP: u8 = 0x04;
pub const P9_QTSYMLINK: u8 = 0x02;
pub const _P9_QTLINK: u8 = 0x01;
pub const P9_QTFILE: u8 = 0x00;

// Bitmask values for the getattr request.
pub const _P9_GETATTR_MODE: u64 = 0x00000001;
pub const _P9_GETATTR_NLINK: u64 = 0x00000002;
pub const _P9_GETATTR_UID: u64 = 0x00000004;
pub const _P9_GETATTR_GID: u64 = 0x00000008;
pub const _P9_GETATTR_RDEV: u64 = 0x00000010;
pub const _P9_GETATTR_ATIME: u64 = 0x00000020;
pub const _P9_GETATTR_MTIME: u64 = 0x00000040;
pub const _P9_GETATTR_CTIME: u64 = 0x00000080;
pub const _P9_GETATTR_INO: u64 = 0x00000100;
pub const _P9_GETATTR_SIZE: u64 = 0x00000200;
pub const _P9_GETATTR_BLOCKS: u64 = 0x00000400;

pub const _P9_GETATTR_BTIME: u64 = 0x00000800;
pub const _P9_GETATTR_GEN: u64 = 0x00001000;
pub const _P9_GETATTR_DATA_VERSION: u64 = 0x00002000;

pub const P9_GETATTR_BASIC: u64 = 0x000007ff; /* Mask for fields up to BLOCKS */
pub const _P9_GETATTR_ALL: u64 = 0x00003fff; /* Mask for All fields above */

// Bitmask values for the setattr request.
pub const P9_SETATTR_MODE: u32 = 0x00000001;
pub const P9_SETATTR_UID: u32 = 0x00000002;
pub const P9_SETATTR_GID: u32 = 0x00000004;
pub const P9_SETATTR_SIZE: u32 = 0x00000008;
pub const P9_SETATTR_ATIME: u32 = 0x00000010;
pub const P9_SETATTR_MTIME: u32 = 0x00000020;
pub const P9_SETATTR_CTIME: u32 = 0x00000040;
pub const P9_SETATTR_ATIME_SET: u32 = 0x00000080;
pub const P9_SETATTR_MTIME_SET: u32 = 0x00000100;

// 9p lock constants. Taken from "include/net/9p/9p.h" in the linux kernel.
pub const _P9_LOCK_TYPE_RDLCK: u8 = 0;
pub const _P9_LOCK_TYPE_WRLCK: u8 = 1;
pub const P9_LOCK_TYPE_UNLCK: u8 = 2;
pub const _P9_LOCK_FLAGS_BLOCK: u8 = 1;
pub const _P9_LOCK_FLAGS_RECLAIM: u8 = 2;
pub const P9_LOCK_SUCCESS: u8 = 0;
pub const _P9_LOCK_BLOCKED: u8 = 1;
pub const _P9_LOCK_ERROR: u8 = 2;
pub const _P9_LOCK_GRACE: u8 = 3;

// Minimum and maximum message size that we'll expect from the client.
pub const MIN_MESSAGE_SIZE: u32 = 256;
pub const MAX_MESSAGE_SIZE: u32 = 64 * 1024 + 24; // 64 KiB of payload plus some extra for the header

pub enum Version {
    V9P2000 = 0,
    V9P2000U = 1,
    V9P2000L = 2,
    V9P2000Lu = 3,
    V9P2024q9p = 4,
}

impl From<&str> for Version {
    fn from(version: &str) -> Self {
        match version {
            "9P2000" => Version::V9P2000,
            "9P2000.u" => Version::V9P2000U,
            "9P2000.L" => Version::V9P2000L,
            "9P2000.Lu" => Version::V9P2000Lu,
            "9P2024.q9p" => Version::V9P2024q9p,
            _ => panic!("Invalid 9P version: {}", version),
        }
    }
}

impl From<String> for Version {
    fn from(version: String) -> Self {
        match version.as_str() {
            "9P2000" => Version::V9P2000,
            "9P2000.u" => Version::V9P2000U,
            "9P2000.L" => Version::V9P2000L,
            "9P2000.Lu" => Version::V9P2000Lu,
            "9P2024.q9p" => Version::V9P2024q9p,
            _ => panic!("Invalid 9P version: {}", version),
        }
    }
}

impl From<Version> for String {
    fn from(val: Version) -> Self {
        match val {
            Version::V9P2000 => "9P2000".to_string(),
            Version::V9P2000U => "9P2000.u".to_string(),
            Version::V9P2000L => "9P2000.L".to_string(),
            Version::V9P2000Lu => "9P2000.Lu".to_string(),
            Version::V9P2024q9p => "9P2024.q9p".to_string(),
        }
    }
}

impl From<Version> for &str {
    fn from(val: Version) -> Self {
        match val {
            Version::V9P2000 => "9P2000",
            Version::V9P2000U => "9P2000.u",
            Version::V9P2000L => "9P2000.L",
            Version::V9P2000Lu => "9P2000.Lu",
            Version::V9P2024q9p => "9P2024.q9p",
        }
    }
}

pub(crate) const DEFAULT_MSIZE: u32 = 8192;

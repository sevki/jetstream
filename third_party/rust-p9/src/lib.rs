// Copyright 2018 The ChromiumOS Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#![cfg(unix)]

extern crate libc;

#[macro_use]
extern crate jetstream_p9_wire_format_derive;

pub mod protocol;
pub mod server;

pub mod fuzzing;

pub use server::*;
pub use protocol::*;

#[macro_export]
macro_rules! syscall {
    ($e:expr) => {{
        let res = $e;
        if res < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
}

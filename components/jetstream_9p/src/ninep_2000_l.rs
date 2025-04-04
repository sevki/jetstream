// Copyright (c) 2024, Sevki <s@sevki.io>
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.
use std::io;

use super::*;

/// 9p
#[trait_variant::make(Send+Sync)]
pub trait NineP200L {
    /// The version message is the first message sent on a connection. It is used to negotiate the
    /// 9P protocol version and maximum message size.
    async fn version(
        &mut self,
        tag: u16,
        version: &Tversion,
    ) -> io::Result<Rversion>;

    /// The auth message is used to authenticate a user to the server. It is sent after the version
    /// message and before any other messages.
    /// The auth message is optional and may be ignored by the server.
    async fn auth(&mut self, tag: u16, auth: &Tauth) -> io::Result<Rauth>;

    /// The flush message is used to flush pending I/O requests.
    async fn flush(&mut self, tag: u16, flush: &Tflush) -> io::Result<()>;

    /// The walk message is used to traverse the file system hierarchy. It is sent by the client and
    /// responded to by the server.
    async fn walk(&mut self, tag: u16, walk: &Twalk) -> io::Result<Rwalk>;

    /// The read message is used to read data from a file.
    async fn read(&mut self, tag: u16, read: &Tread) -> io::Result<Rread>;

    /// The write message is used to write data to a file.
    async fn write(&mut self, tag: u16, write: &Twrite) -> io::Result<Rwrite>;

    /// The clunk message is used to release a fid.
    async fn clunk(&mut self, tag: u16, clunk: &Tclunk) -> io::Result<()>;

    /// The remove message is used to remove a file.
    async fn remove(&mut self, tag: u16, remove: &Tremove) -> io::Result<()>;

    /// The attach message is used to associate a fid with a file.
    async fn attach(
        &mut self,
        tag: u16,
        attach: &Tattach,
    ) -> io::Result<Rattach>;

    /// The statfs message is used to retrieve file system information.
    async fn statfs(
        &mut self,
        tag: u16,
        statfs: &Tstatfs,
    ) -> io::Result<Rstatfs>;

    /// The lopen message is used to open a file.
    async fn lopen(&mut self, tag: u16, lopen: &Tlopen) -> io::Result<Rlopen>;

    /// The lcreate message is used to create a file.
    async fn lcreate(
        &mut self,
        tag: u16,
        lcreate: &Tlcreate,
    ) -> io::Result<Rlcreate>;

    /// The symlink message is used to create a symbolic link.
    async fn symlink(
        &mut self,
        tag: u16,
        symlink: &Tsymlink,
    ) -> io::Result<Rsymlink>;

    /// The mknod message is used to create a device file.
    async fn mknod(&mut self, tag: u16, mknod: &Tmknod) -> io::Result<Rmknod>;

    /// The rename message is used to rename a file.
    async fn rename(&mut self, tag: u16, rename: &Trename) -> io::Result<()>;

    /// The readlink message is used to read the target of a symbolic link.
    async fn readlink(
        &mut self,
        tag: u16,
        readlink: &Treadlink,
    ) -> io::Result<Rreadlink>;

    /// The getattr message is used to retrieve file attributes.
    async fn get_attr(
        &mut self,
        tag: u16,
        get_attr: &Tgetattr,
    ) -> io::Result<Rgetattr>;

    /// The setattr message is used to set file attributes.
    async fn set_attr(
        &mut self,
        tag: u16,
        set_attr: &Tsetattr,
    ) -> io::Result<()>;

    /// The xattrwalk message is used to traverse extended attributes.
    async fn xattr_walk(
        &mut self,
        tag: u16,
        xattr_walk: &Txattrwalk,
    ) -> io::Result<Rxattrwalk>;

    /// The xattrcreate message is used to create an extended attribute.
    async fn xattr_create(
        &mut self,
        tag: u16,
        xattr_create: &Txattrcreate,
    ) -> io::Result<()>;

    /// The readdir message is used to read a directory.
    async fn readdir(
        &mut self,
        tag: u16,
        readdir: &Treaddir,
    ) -> io::Result<Rreaddir>;

    /// The fsync message is used to synchronize a file's data and metadata.
    async fn fsync(&mut self, tag: u16, fsync: &Tfsync) -> io::Result<()>;

    /// The lock message is used to lock a file.
    async fn lock(&mut self, tag: u16, lock: &Tlock) -> io::Result<Rlock>;

    /// The getlock message is used to retrieve a file's locks.
    async fn get_lock(
        &mut self,
        tag: u16,
        get_lock: &Tgetlock,
    ) -> io::Result<Rgetlock>;

    /// The link message is used to create a hard link.
    async fn link(&mut self, tag: u16, link: &Tlink) -> io::Result<()>;

    /// The mkdir message is used to create a directory.
    async fn mkdir(&mut self, tag: u16, mkdir: &Tmkdir) -> io::Result<Rmkdir>;

    /// The renameat message is used to rename a file.
    async fn rename_at(
        &mut self,
        tag: u16,
        rename_at: &Trenameat,
    ) -> io::Result<()>;

    /// The unlinkat message is used to remove a file.
    async fn unlink_at(
        &mut self,
        tag: u16,
        unlink_at: &Tunlinkat,
    ) -> io::Result<()>;
}

use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    io::{Read, Write},
};

use async_trait::async_trait;
use futures::{AsyncReadExt, AsyncWriteExt};
use p9::{wire_format, Tmessage, WireFormat};
use s2n_quic::stream::{BidirectionalStream, ReceiveStream, SendStream};
use tokio::{runtime::Runtime, sync::Mutex};

use crate::versions::{self, DEFAULT_MSIZE};

pub struct NinePClient<'a> {
    msize: usize,
    connection: s2n_quic::Connection,

    rt: &'a Runtime,
    // Tag map
    tags: Mutex<BTreeMap<u16, NinePClientConnection<'a>>>,
}

pub struct Name(Vec<String>);

pub struct Owned(u16, Name);

pub struct File {
    pub name: Name,
}

pub struct Dir {
    pub name: Name,
}

pub struct DirEntry(Name);

impl genfs::DirEntry for DirEntry {
    type Path = Name;

    type PathOwned = Owned;

    type Metadata = std::fs::Metadata;

    type FileType = std::fs::FileType;

    type Error = std::io::Error;

    fn path(&self) -> Self::PathOwned {
        todo!()
    }

    fn metadata(
        &self,
    ) -> std::prelude::v1::Result<Self::Metadata, Self::Error> {
        todo!()
    }

    fn file_type(
        &self,
    ) -> std::prelude::v1::Result<Self::FileType, Self::Error> {
        todo!()
    }

    fn file_name(&self) -> &Self::Path {
        todo!()
    }
}

impl Iterator for Dir {
    type Item = Result<DirEntry, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

impl genfs::Dir<DirEntry, std::io::Error> for Dir {}

impl genfs::File for File {
    type Error = std::io::Error;

    fn read(
        &self,
        buf: &mut [u8],
    ) -> std::prelude::v1::Result<usize, Self::Error> {
        todo!()
    }

    fn write(
        &mut self,
        buf: &[u8],
    ) -> std::prelude::v1::Result<usize, Self::Error> {
        todo!()
    }

    fn flush(&mut self) -> std::prelude::v1::Result<(), Self::Error> {
        todo!()
    }

    fn seek(
        &mut self,
        pos: genfs::SeekFrom,
    ) -> std::prelude::v1::Result<u64, Self::Error> {
        todo!()
    }
}

/// The borrowed path slice that represents a relative or absolute path on
/// the filesystem.
impl<'a> genfs::Fs for NinePClient<'a> {
    type Path = Name;

    type PathOwned = Owned;

    type File = File;

    type Dir = Dir;

    type DirEntry = DirEntry;

    type Metadata = std::fs::Metadata;

    type Permissions = std::fs::Permissions;

    type Error = std::io::Error;

    fn open(
        &self,
        path: &Self::Path,
        options: &genfs::OpenOptions<Self::Permissions>,
    ) -> std::prelude::v1::Result<Self::File, Self::Error> {
        todo!()
    }

    fn remove_file(
        &mut self,
        path: &Self::Path,
    ) -> std::prelude::v1::Result<(), Self::Error> {
        todo!()
    }

    fn metadata(
        &self,
        path: &Self::Path,
    ) -> std::prelude::v1::Result<Self::Metadata, Self::Error> {
        todo!()
    }

    fn symlink_metadata(
        &self,
        path: &Self::Path,
    ) -> std::prelude::v1::Result<Self::Metadata, Self::Error> {
        todo!()
    }

    fn rename(
        &mut self,
        from: &Self::Path,
        to: &Self::Path,
    ) -> std::prelude::v1::Result<(), Self::Error> {
        todo!()
    }

    fn copy(
        &mut self,
        from: &Self::Path,
        to: &Self::Path,
    ) -> std::prelude::v1::Result<u64, Self::Error> {
        todo!()
    }

    fn hard_link(
        &mut self,
        src: &Self::Path,
        dst: &Self::Path,
    ) -> std::prelude::v1::Result<(), Self::Error> {
        todo!()
    }

    fn symlink(
        &mut self,
        src: &Self::Path,
        dst: &Self::Path,
    ) -> std::prelude::v1::Result<(), Self::Error> {
        todo!()
    }

    fn read_link(
        &self,
        path: &Self::Path,
    ) -> std::prelude::v1::Result<Self::PathOwned, Self::Error> {
        todo!()
    }

    fn canonicalize(
        &self,
        path: &Self::Path,
    ) -> std::prelude::v1::Result<Self::PathOwned, Self::Error> {
        todo!()
    }

    fn create_dir(
        &mut self,
        path: &Self::Path,
        options: &genfs::DirOptions<Self::Permissions>,
    ) -> std::prelude::v1::Result<(), Self::Error> {
        todo!()
    }

    fn remove_dir(
        &mut self,
        path: &Self::Path,
    ) -> std::prelude::v1::Result<(), Self::Error> {
        todo!()
    }

    fn remove_dir_all(
        &mut self,
        path: &Self::Path,
    ) -> std::prelude::v1::Result<(), Self::Error> {
        todo!()
    }

    fn read_dir(
        &self,
        path: &Self::Path,
    ) -> std::prelude::v1::Result<Self::Dir, Self::Error> {
        todo!()
    }

    fn set_permissions(
        &mut self,
        path: &Self::Path,
        perm: Self::Permissions,
    ) -> std::prelude::v1::Result<(), Self::Error> {
        todo!()
    }
}

pub struct TStream<'a> {
    rt: &'a tokio::runtime::Runtime,
    stream: SendStream,
}

impl<'a> TStream<'a> {
    pub fn new(rt: &'a Runtime, stream: SendStream) -> Self {
        Self { rt, stream }
    }
}

impl<'a> std::io::Write for TStream<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _guard = self.rt.enter();
        self.rt.block_on(async {
            match self.stream.write(buf).await {
                std::io::Result::Ok(n) => Ok(n),
                Err(err) => std::io::Result::Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("failed to write to stream: {}", err),
                )),
            }
        })
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub struct RStream<'a> {
    rt: &'a tokio::runtime::Runtime,
    stream: ReceiveStream,
}

impl<'a> RStream<'a> {
    pub fn new(rt: &'a Runtime, stream: ReceiveStream) -> Self {
        Self { rt, stream }
    }
}

impl<'a> std::io::Read for RStream<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let _guard = self.rt.enter();
        self.rt.block_on(async {
            match self.stream.read(buf).await {
                Ok(n) => match n {
                    0 => Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "stream closed",
                    )),
                    n => Ok(n),
                },
                Err(err) => Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("failed to read from stream: {}", err),
                )),
            }
        })
    }
}

impl<'a> NinePClient<'a> {
    pub fn new(connection: s2n_quic::Connection, rt: &'a Runtime) -> Self {
        Self {
            rt,
            msize: DEFAULT_MSIZE,
            tags: Mutex::new(BTreeMap::new()),
            connection,
        }
    }
}

pub struct NinePClientConnection<'a> {
    fids: Mutex<HashMap<u32, u64>>,
    tx: TStream<'a>,
    rx: RStream<'a>,
    turn: Turn,
}

impl<'a> NinePClient<'a> {
        pub async fn attach(
            &mut self,
            tag: u16,
            aname: &str,
        ) -> NinePClientConnection<'a> {
            let bi_stream = self.connection.open_bidirectional_stream().await.unwrap();
            let (recv, send) = bi_stream.split();
            let mut conn = NinePClientConnection {
                fids: Mutex::new(HashMap::new()),
                tx: TStream::new(self.rt, send),
                rx: RStream::new(self.rt, recv),
                turn: Turn::Client,
            };
            let _ = conn.version(0, DEFAULT_MSIZE, versions::Version::V9P2024q9p.into()).await;
            conn
        }
}

enum Turn {
    Client,
    Server,
}

impl<'a> NinePClientConnection<'a> {
    pub async fn attach(
        &mut self,
        tag: u16,
        aname: &str,
        uname: &str,
    ) -> std::prelude::v1::Result<(), std::io::Error> {
        todo!()
    }

    pub async fn version(
        &mut self,
        tag: u16,
        msize: usize,
        version: &str,
    ) -> std::prelude::v1::Result<(), std::io::Error> {
        todo!()
    }

    async fn write_message(
        &mut self,
        tag: u16,
        msg: &Tmessage,
        tx: &mut TStream<'a>,
    ) -> std::prelude::v1::Result<(), std::io::Error> {
        let mut _guard = self.fids.lock().await;
        let _ = match msg {
            Tmessage::Version(version) => {
                wire_format::WireFormat::encode(version, tx)
            }
            Tmessage::Flush(flush) => {
                wire_format::WireFormat::encode(flush, tx)
            }
            Tmessage::Read(read) => wire_format::WireFormat::encode(read, tx),
            Tmessage::Write(write) => {
                wire_format::WireFormat::encode(write, tx)
            }
            Tmessage::Clunk(clunk) => {
                wire_format::WireFormat::encode(clunk, tx)
            }
            Tmessage::Remove(remove) => {
                wire_format::WireFormat::encode(remove, tx)
            }
            Tmessage::Attach(attach) => {
                wire_format::WireFormat::encode(attach, tx)
            }
            Tmessage::Auth(auth) => wire_format::WireFormat::encode(auth, tx),
            Tmessage::Statfs(statfs) => {
                wire_format::WireFormat::encode(statfs, tx)
            }
            Tmessage::Lopen(lopen) => {
                wire_format::WireFormat::encode(lopen, tx)
            }
            Tmessage::Lcreate(lcreate) => {
                wire_format::WireFormat::encode(lcreate, tx)
            }
            Tmessage::Symlink(symlink) => {
                wire_format::WireFormat::encode(symlink, tx)
            }
            Tmessage::Mknod(mknod) => {
                wire_format::WireFormat::encode(mknod, tx)
            }
            Tmessage::Rename(rename) => {
                wire_format::WireFormat::encode(rename, tx)
            }
            Tmessage::Readlink(readlink) => {
                wire_format::WireFormat::encode(readlink, tx)
            }
            Tmessage::GetAttr(getattr) => {
                wire_format::WireFormat::encode(getattr, tx)
            }
            Tmessage::SetAttr(setattr) => {
                wire_format::WireFormat::encode(setattr, tx)
            }
            Tmessage::XattrWalk(xattrwalk) => {
                wire_format::WireFormat::encode(xattrwalk, tx)
            }
            Tmessage::XattrCreate(xattrcreate) => {
                wire_format::WireFormat::encode(xattrcreate, tx)
            }
            Tmessage::Readdir(readdir) => {
                wire_format::WireFormat::encode(readdir, tx)
            }
            Tmessage::Fsync(fsync) => {
                wire_format::WireFormat::encode(fsync, tx)
            }
            Tmessage::Lock(lock) => wire_format::WireFormat::encode(lock, tx),
            Tmessage::GetLock(getlock) => {
                wire_format::WireFormat::encode(getlock, tx)
            }
            Tmessage::Link(link) => wire_format::WireFormat::encode(link, tx),
            Tmessage::Mkdir(mkdir) => {
                wire_format::WireFormat::encode(mkdir, tx)
            }
            Tmessage::RenameAt(renameat) => {
                wire_format::WireFormat::encode(renameat, tx)
            }
            Tmessage::UnlinkAt(unlinkat) => {
                wire_format::WireFormat::encode(unlinkat, tx)
            }
            Tmessage::Walk(walk) => wire_format::WireFormat::encode(walk, tx),
        };

        Ok(())
    }
}

// A reference implementation written in go.
/*
package client // import "9fans.net/go/plan9/client"

import (
    "fmt"
    "io"
    "sync"
    "sync/atomic"

    "9fans.net/go/plan9"
)

type Error string

func (e Error) Error() string { return string(e) }

type Conn struct {
    // We wrap the underlying conn type so that
    // there's a clear distinction between Close,
    // which forces a close of the underlying rwc,
    // and Release, which lets the Fids take control
    // of when the conn is actually closed.
    mu       sync.Mutex
    _c       *conn
    released bool
}

var errClosed = fmt.Errorf("connection has been closed")

// Close forces a close of the connection and all Fids derived
// from it.
func (c *Conn) Close() error {
    c.mu.Lock()
    defer c.mu.Unlock()
    if c._c == nil {
        if c.released {
            return fmt.Errorf("cannot close connection after it's been released")
        }
        return nil
    }
    rwc := c._c.rwc
    c._c = nil
    // TODO perhaps we shouldn't hold the mutex while closing?
    return rwc.Close()
}

func (c *Conn) conn() (*conn, error) {
    c.mu.Lock()
    defer c.mu.Unlock()
    if c._c == nil {
        return nil, errClosed
    }
    return c._c, nil
}

// Release marks the connection so that it will
// close automatically when the last Fid derived
// from it is closed.
//
// If there are no current Fids, it closes immediately.
// After calling Release, c.Attach, c.Auth and c.Close will return
// an error.
func (c *Conn) Release() error {
    c.mu.Lock()
    defer c.mu.Unlock()
    if c._c == nil {
        return nil
    }
    conn := c._c
    c._c = nil
    c.released = true
    return conn.release()
}

type conn struct {
    rwc      io.ReadWriteCloser
    err      error
    tagmap   map[uint16]chan *plan9.Fcall
    freetag  map[uint16]bool
    freefid  map[uint32]bool
    nexttag  uint16
    nextfid  uint32
    msize    uint32
    version  string
    w, x     sync.Mutex
    muxer    bool
    refCount int32 // atomic
}

func NewConn(rwc io.ReadWriteCloser) (*Conn, error) {
    c := &conn{
        rwc:      rwc,
        tagmap:   make(map[uint16]chan *plan9.Fcall),
        freetag:  make(map[uint16]bool),
        freefid:  make(map[uint32]bool),
        nexttag:  1,
        nextfid:  1,
        msize:    131072,
        version:  "9P2000",
        refCount: 1,
    }

    //	XXX raw messages, not c.rpc
    tx := &plan9.Fcall{Type: plan9.Tversion, Tag: plan9.NOTAG, Msize: c.msize, Version: c.version}
    err := c.write(tx)
    if err != nil {
        return nil, err
    }
    rx, err := c.read()
    if err != nil {
        return nil, err
    }
    if rx.Type != plan9.Rversion || rx.Tag != plan9.NOTAG {
        return nil, plan9.ProtocolError(fmt.Sprintf("invalid type/tag in Tversion exchange: %v %v", rx.Type, rx.Tag))
    }

    if rx.Msize > c.msize {
        return nil, plan9.ProtocolError(fmt.Sprintf("invalid msize %d in Rversion", rx.Msize))
    }
    c.msize = rx.Msize
    if rx.Version != "9P2000" {
        return nil, plan9.ProtocolError(fmt.Sprintf("invalid version %s in Rversion", rx.Version))
    }
    return &Conn{
        _c: c,
    }, nil
}

func (c *conn) newFid(fid uint32, qid plan9.Qid) *Fid {
    c.acquire()
    return &Fid{
        _c:  c,
        fid: fid,
        qid: qid,
    }
}

func (c *conn) newfidnum() (uint32, error) {
    c.x.Lock()
    defer c.x.Unlock()
    for fidnum, _ := range c.freefid {
        delete(c.freefid, fidnum)
        return fidnum, nil
    }
    fidnum := c.nextfid
    if c.nextfid == plan9.NOFID {
        return 0, plan9.ProtocolError("out of fids")
    }
    c.nextfid++
    return fidnum, nil
}

func (c *conn) putfidnum(fid uint32) {
    c.x.Lock()
    defer c.x.Unlock()
    c.freefid[fid] = true
}

func (c *conn) newtag(ch chan *plan9.Fcall) (uint16, error) {
    c.x.Lock()
    defer c.x.Unlock()
    var tagnum uint16
    for tagnum, _ = range c.freetag {
        delete(c.freetag, tagnum)
        goto found
    }
    tagnum = c.nexttag
    if c.nexttag == plan9.NOTAG {
        return 0, plan9.ProtocolError("out of tags")
    }
    c.nexttag++
found:
    c.tagmap[tagnum] = ch
    if !c.muxer {
        c.muxer = true
        ch <- &yourTurn
    }
    return tagnum, nil
}

func (c *conn) puttag(tag uint16) chan *plan9.Fcall {
    c.x.Lock()
    defer c.x.Unlock()
    ch := c.tagmap[tag]
    delete(c.tagmap, tag)
    c.freetag[tag] = true
    return ch
}

func (c *conn) mux(rx *plan9.Fcall) {
    c.x.Lock()
    defer c.x.Unlock()

    ch := c.tagmap[rx.Tag]
    delete(c.tagmap, rx.Tag)
    c.freetag[rx.Tag] = true
    c.muxer = false
    for _, ch2 := range c.tagmap {
        c.muxer = true
        ch2 <- &yourTurn
        break
    }
    ch <- rx
}

func (c *conn) read() (*plan9.Fcall, error) {
    if err := c.getErr(); err != nil {
        return nil, err
    }
    f, err := plan9.ReadFcall(c.rwc)
    if err != nil {
        c.setErr(err)
        return nil, err
    }
    return f, nil
}

func (c *conn) write(f *plan9.Fcall) error {
    if err := c.getErr(); err != nil {
        return err
    }
    err := plan9.WriteFcall(c.rwc, f)
    if err != nil {
        c.setErr(err)
    }
    return err
}

var yourTurn plan9.Fcall

func (c *conn) rpc(tx *plan9.Fcall, clunkFid *Fid) (rx *plan9.Fcall, err error) {
    ch := make(chan *plan9.Fcall, 1)
    tx.Tag, err = c.newtag(ch)
    if err != nil {
        return nil, err
    }
    c.w.Lock()
    err = c.write(tx)
    // Mark the fid as clunked inside the write lock so that we're
    // sure that we don't reuse it after the sending the message
    // that will clunk it, even in the presence of concurrent method
    // calls on Fid.
    if clunkFid != nil {
        // Closing the Fid might release the conn, which
        // would close the underlying rwc connection,
        // which would prevent us from being able to receive the
        // reply, so make sure that doesn't happen until the end
        // by acquiring a reference for the duration of the call.
        c.acquire()
        defer c.release()
        if err := clunkFid.clunked(); err != nil {
            // This can happen if two clunking operations
            // (e.g. Close and Remove) are invoked concurrently
            c.w.Unlock()
            return nil, err
        }
    }
    c.w.Unlock()
    if err != nil {
        return nil, err
    }

    for rx = range ch {
        if rx != &yourTurn {
            break
        }
        rx, err = c.read()
        if err != nil {
            break
        }
        c.mux(rx)
    }

    if rx == nil {
        return nil, c.getErr()
    }
    if rx.Type == plan9.Rerror {
        return nil, Error(rx.Ename)
    }
    if rx.Type != tx.Type+1 {
        return nil, plan9.ProtocolError("packet type mismatch")
    }
    return rx, nil
}

func (c *conn) acquire() {
    atomic.AddInt32(&c.refCount, 1)
}

func (c *conn) release() error {
    if atomic.AddInt32(&c.refCount, -1) != 0 {
        return nil
    }
    err := c.rwc.Close()
    c.setErr(errClosed)
    return err
}

func (c *conn) getErr() error {
    c.x.Lock()
    defer c.x.Unlock()
    return c.err
}

func (c *conn) setErr(err error) {
    c.x.Lock()
    defer c.x.Unlock()
    c.err = err
}
 */

mod client_tests;

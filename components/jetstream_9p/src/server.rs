use jetstream_rpc::{context::Context, Frame, Framer, Protocol};
use jetstream_wireformat::WireFormat;

use crate::{ninep_2000_l::NineP200L, *};

// Message type constants.  Taken from "include/net/9p/9p.h" in the linux kernel
// tree.  The protocol specifies each R* message to be the corresponding T*
// message plus one.
const TLERROR: u8 = 6;
const RLERROR: u8 = TLERROR + 1;
const TSTATFS: u8 = 8;
const RSTATFS: u8 = TSTATFS + 1;
const TLOPEN: u8 = 12;
const RLOPEN: u8 = TLOPEN + 1;
const TLCREATE: u8 = 14;
const RLCREATE: u8 = TLCREATE + 1;
const TSYMLINK: u8 = 16;
const RSYMLINK: u8 = TSYMLINK + 1;
const TMKNOD: u8 = 18;
const RMKNOD: u8 = TMKNOD + 1;
const TRENAME: u8 = 20;
const RRENAME: u8 = TRENAME + 1;
const TREADLINK: u8 = 22;
const RREADLINK: u8 = TREADLINK + 1;
const TGETATTR: u8 = 24;
const RGETATTR: u8 = TGETATTR + 1;
const TSETATTR: u8 = 26;
const RSETATTR: u8 = TSETATTR + 1;
const TXATTRWALK: u8 = 30;
const RXATTRWALK: u8 = TXATTRWALK + 1;
const TXATTRCREATE: u8 = 32;
const RXATTRCREATE: u8 = TXATTRCREATE + 1;
const TREADDIR: u8 = 40;
const RREADDIR: u8 = TREADDIR + 1;
const TFSYNC: u8 = 50;
const RFSYNC: u8 = TFSYNC + 1;
const TLOCK: u8 = 52;
const RLOCK: u8 = TLOCK + 1;
const TGETLOCK: u8 = 54;
const RGETLOCK: u8 = TGETLOCK + 1;
const TLINK: u8 = 70;
const RLINK: u8 = TLINK + 1;
const TMKDIR: u8 = 72;
const RMKDIR: u8 = TMKDIR + 1;
const TRENAMEAT: u8 = 74;
const RRENAMEAT: u8 = TRENAMEAT + 1;
const TUNLINKAT: u8 = 76;
const RUNLINKAT: u8 = TUNLINKAT + 1;
const TVERSION: u8 = 100;
const RVERSION: u8 = TVERSION + 1;
const TAUTH: u8 = 102;
const RAUTH: u8 = TAUTH + 1;
const TATTACH: u8 = 104;
const RATTACH: u8 = TATTACH + 1;
const _TERROR: u8 = 106;
const _RERROR: u8 = _TERROR + 1;
const TFLUSH: u8 = 108;
const RFLUSH: u8 = TFLUSH + 1;
const TWALK: u8 = 110;
const RWALK: u8 = TWALK + 1;
const _TOPEN: u8 = 112;
const _ROPEN: u8 = _TOPEN + 1;
const _TCREATE: u8 = 114;
const _RCREATE: u8 = _TCREATE + 1;
const TREAD: u8 = 116;
const RREAD: u8 = TREAD + 1;
const TWRITE: u8 = 118;
const RWRITE: u8 = TWRITE + 1;
const TCLUNK: u8 = 120;
const RCLUNK: u8 = TCLUNK + 1;
const TREMOVE: u8 = 122;
const RREMOVE: u8 = TREMOVE + 1;
const _TSTAT: u8 = 124;
const _RSTAT: u8 = _TSTAT + 1;
const _TWSTAT: u8 = 126;
const _RWSTAT: u8 = _TWSTAT + 1;

/// A message sent from a 9P client to a 9P server.
#[derive(Debug)]
#[repr(u8)]
pub enum Tmessage {
    Version(Tversion) = TVERSION,
    Flush(Tflush) = TFLUSH,
    Walk(Twalk) = TWALK,
    Read(Tread) = TREAD,
    Write(Twrite) = TWRITE,
    Clunk(Tclunk) = TCLUNK,
    Remove(Tremove) = TREMOVE,
    Attach(Tattach) = TATTACH,
    Auth(Tauth) = TAUTH,
    Statfs(Tstatfs) = TSTATFS,
    Lopen(Tlopen) = TLOPEN,
    Lcreate(Tlcreate) = TLCREATE,
    Symlink(Tsymlink) = TSYMLINK,
    Mknod(Tmknod) = TMKNOD,
    Rename(Trename) = TRENAME,
    Readlink(Treadlink) = TREADLINK,
    GetAttr(Tgetattr) = TGETATTR,
    SetAttr(Tsetattr) = TSETATTR,
    XattrWalk(Txattrwalk) = TXATTRWALK,
    XattrCreate(Txattrcreate) = TXATTRCREATE,
    Readdir(Treaddir) = TREADDIR,
    Fsync(Tfsync) = TFSYNC,
    Lock(Tlock) = TLOCK,
    GetLock(Tgetlock) = TGETLOCK,
    Link(Tlink) = TLINK,
    Mkdir(Tmkdir) = TMKDIR,
    RenameAt(Trenameat) = TRENAMEAT,
    UnlinkAt(Tunlinkat) = TUNLINKAT,
}

impl Framer for Tmessage {
    fn message_type(&self) -> u8 {
        match self {
            Tmessage::Version(_) => TVERSION,
            Tmessage::Flush(_) => TFLUSH,
            Tmessage::Walk(_) => TWALK,
            Tmessage::Read(_) => TREAD,
            Tmessage::Write(_) => TWRITE,
            Tmessage::Clunk(_) => TCLUNK,
            Tmessage::Remove(_) => TREMOVE,
            Tmessage::Attach(_) => TATTACH,
            Tmessage::Auth(_) => TAUTH,
            Tmessage::Statfs(_) => TSTATFS,
            Tmessage::Lopen(_) => TLOPEN,
            Tmessage::Lcreate(_) => TLCREATE,
            Tmessage::Symlink(_) => TSYMLINK,
            Tmessage::Mknod(_) => TMKNOD,
            Tmessage::Rename(_) => TRENAME,
            Tmessage::Readlink(_) => TREADLINK,
            Tmessage::GetAttr(_) => TGETATTR,
            Tmessage::SetAttr(_) => TSETATTR,
            Tmessage::XattrWalk(_) => TXATTRWALK,
            Tmessage::XattrCreate(_) => TXATTRCREATE,
            Tmessage::Readdir(_) => TREADDIR,
            Tmessage::Fsync(_) => TFSYNC,
            Tmessage::Lock(_) => TLOCK,
            Tmessage::GetLock(_) => TGETLOCK,
            Tmessage::Link(_) => TLINK,
            Tmessage::Mkdir(_) => TMKDIR,
            Tmessage::RenameAt(_) => TRENAMEAT,
            Tmessage::UnlinkAt(_) => TUNLINKAT,
        }
    }

    fn byte_size(&self) -> u32 {
        match self {
            Tmessage::Version(msg) => msg.byte_size(),
            Tmessage::Flush(msg) => msg.byte_size(),
            Tmessage::Walk(msg) => msg.byte_size(),
            Tmessage::Read(msg) => msg.byte_size(),
            Tmessage::Write(msg) => msg.byte_size(),
            Tmessage::Clunk(msg) => msg.byte_size(),
            Tmessage::Remove(msg) => msg.byte_size(),
            Tmessage::Attach(msg) => msg.byte_size(),
            Tmessage::Auth(msg) => msg.byte_size(),
            Tmessage::Statfs(msg) => msg.byte_size(),
            Tmessage::Lopen(msg) => msg.byte_size(),
            Tmessage::Lcreate(msg) => msg.byte_size(),
            Tmessage::Symlink(msg) => msg.byte_size(),
            Tmessage::Mknod(msg) => msg.byte_size(),
            Tmessage::Rename(msg) => msg.byte_size(),
            Tmessage::Readlink(msg) => msg.byte_size(),
            Tmessage::GetAttr(msg) => msg.byte_size(),
            Tmessage::SetAttr(msg) => msg.byte_size(),
            Tmessage::XattrWalk(msg) => msg.byte_size(),
            Tmessage::XattrCreate(msg) => msg.byte_size(),
            Tmessage::Readdir(msg) => msg.byte_size(),
            Tmessage::Fsync(msg) => msg.byte_size(),
            Tmessage::Lock(msg) => msg.byte_size(),
            Tmessage::GetLock(msg) => msg.byte_size(),
            Tmessage::Link(msg) => msg.byte_size(),
            Tmessage::Mkdir(msg) => msg.byte_size(),
            Tmessage::RenameAt(msg) => msg.byte_size(),
            Tmessage::UnlinkAt(msg) => msg.byte_size(),
        }
    }

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            Tmessage::Version(msg) => msg.encode(writer),
            Tmessage::Flush(msg) => msg.encode(writer),
            Tmessage::Walk(msg) => msg.encode(writer),
            Tmessage::Read(msg) => msg.encode(writer),
            Tmessage::Write(msg) => msg.encode(writer),
            Tmessage::Clunk(msg) => msg.encode(writer),
            Tmessage::Remove(msg) => msg.encode(writer),
            Tmessage::Attach(msg) => msg.encode(writer),
            Tmessage::Auth(msg) => msg.encode(writer),
            Tmessage::Statfs(msg) => msg.encode(writer),
            Tmessage::Lopen(msg) => msg.encode(writer),
            Tmessage::Lcreate(msg) => msg.encode(writer),
            Tmessage::Symlink(msg) => msg.encode(writer),
            Tmessage::Mknod(msg) => msg.encode(writer),
            Tmessage::Rename(msg) => msg.encode(writer),
            Tmessage::Readlink(msg) => msg.encode(writer),
            Tmessage::GetAttr(msg) => msg.encode(writer),
            Tmessage::SetAttr(msg) => msg.encode(writer),
            Tmessage::XattrWalk(msg) => msg.encode(writer),
            Tmessage::XattrCreate(msg) => msg.encode(writer),
            Tmessage::Readdir(msg) => msg.encode(writer),
            Tmessage::Fsync(msg) => msg.encode(writer),
            Tmessage::Lock(msg) => msg.encode(writer),
            Tmessage::GetLock(msg) => msg.encode(writer),
            Tmessage::Link(msg) => msg.encode(writer),
            Tmessage::Mkdir(msg) => msg.encode(writer),
            Tmessage::RenameAt(msg) => msg.encode(writer),
            Tmessage::UnlinkAt(msg) => msg.encode(writer),
        }
    }

    fn decode<R: std::io::Read>(
        reader: &mut R,
        ty: u8,
    ) -> std::io::Result<Self> {
        match ty {
            TVERSION => Ok(Tmessage::Version(Tversion::decode(reader)?)),
            TFLUSH => Ok(Tmessage::Flush(Tflush::decode(reader)?)),
            TWALK => Ok(Tmessage::Walk(Twalk::decode(reader)?)),
            TREAD => Ok(Tmessage::Read(Tread::decode(reader)?)),
            TWRITE => Ok(Tmessage::Write(Twrite::decode(reader)?)),
            TCLUNK => Ok(Tmessage::Clunk(Tclunk::decode(reader)?)),
            TREMOVE => Ok(Tmessage::Remove(Tremove::decode(reader)?)),
            TATTACH => Ok(Tmessage::Attach(Tattach::decode(reader)?)),
            TAUTH => Ok(Tmessage::Auth(Tauth::decode(reader)?)),
            TSTATFS => Ok(Tmessage::Statfs(Tstatfs::decode(reader)?)),
            TLOPEN => Ok(Tmessage::Lopen(Tlopen::decode(reader)?)),
            TLCREATE => Ok(Tmessage::Lcreate(Tlcreate::decode(reader)?)),
            TSYMLINK => Ok(Tmessage::Symlink(Tsymlink::decode(reader)?)),
            TMKNOD => Ok(Tmessage::Mknod(Tmknod::decode(reader)?)),
            TRENAME => Ok(Tmessage::Rename(Trename::decode(reader)?)),
            TREADLINK => Ok(Tmessage::Readlink(Treadlink::decode(reader)?)),
            TGETATTR => Ok(Tmessage::GetAttr(Tgetattr::decode(reader)?)),
            TSETATTR => Ok(Tmessage::SetAttr(Tsetattr::decode(reader)?)),
            TXATTRWALK => Ok(Tmessage::XattrWalk(Txattrwalk::decode(reader)?)),
            TXATTRCREATE => {
                Ok(Tmessage::XattrCreate(Txattrcreate::decode(reader)?))
            }
            TREADDIR => Ok(Tmessage::Readdir(Treaddir::decode(reader)?)),
            TFSYNC => Ok(Tmessage::Fsync(Tfsync::decode(reader)?)),
            TLOCK => Ok(Tmessage::Lock(Tlock::decode(reader)?)),
            TGETLOCK => Ok(Tmessage::GetLock(Tgetlock::decode(reader)?)),
            TLINK => Ok(Tmessage::Link(Tlink::decode(reader)?)),
            TMKDIR => Ok(Tmessage::Mkdir(Tmkdir::decode(reader)?)),
            TRENAMEAT => Ok(Tmessage::RenameAt(Trenameat::decode(reader)?)),
            TUNLINKAT => Ok(Tmessage::UnlinkAt(Tunlinkat::decode(reader)?)),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid message type",
            )),
        }
    }
}

/// A message sent from a 9P server to a 9P client in response to a request from
/// that client.  Encapsulates a full frame.
#[derive(Debug)]
#[repr(u8)]
pub enum Rmessage {
    Version(Rversion) = RVERSION,
    Flush = RFLUSH,
    Walk(Rwalk) = RWALK,
    Read(Rread) = RREAD,
    Write(Rwrite) = RWRITE,
    Clunk = RCLUNK,
    Remove = RREMOVE,
    Attach(Rattach) = RATTACH,
    Auth(Rauth) = RAUTH,
    Statfs(Rstatfs) = RSTATFS,
    Lopen(Rlopen) = RLOPEN,
    Lcreate(Rlcreate) = RLCREATE,
    Symlink(Rsymlink) = RSYMLINK,
    Mknod(Rmknod) = RMKNOD,
    Rename = RRENAME,
    Readlink(Rreadlink) = RREADLINK,
    GetAttr(Rgetattr) = RGETATTR,
    SetAttr = RSETATTR,
    XattrWalk(Rxattrwalk) = RXATTRWALK,
    XattrCreate = RXATTRCREATE,
    Readdir(Rreaddir) = RREADDIR,
    Fsync = RFSYNC,
    Lock(Rlock) = RLOCK,
    GetLock(Rgetlock) = RGETLOCK,
    Link = RLINK,
    Mkdir(Rmkdir) = RMKDIR,
    RenameAt = RRENAMEAT,
    UnlinkAt = RUNLINKAT,
    Lerror(Rlerror) = RLERROR,
}

impl Framer for Rmessage {
    fn message_type(&self) -> u8 {
        match self {
            Rmessage::Version(_) => RVERSION,
            Rmessage::Flush => RFLUSH,
            Rmessage::Walk(_) => RWALK,
            Rmessage::Read(_) => RREAD,
            Rmessage::Write(_) => RWRITE,
            Rmessage::Clunk => RCLUNK,
            Rmessage::Remove => RREMOVE,
            Rmessage::Attach(_) => RATTACH,
            Rmessage::Auth(_) => RAUTH,
            Rmessage::Statfs(_) => RSTATFS,
            Rmessage::Lopen(_) => RLOPEN,
            Rmessage::Lcreate(_) => RLCREATE,
            Rmessage::Symlink(_) => RSYMLINK,
            Rmessage::Mknod(_) => RMKNOD,
            Rmessage::Rename => RRENAME,
            Rmessage::Readlink(_) => RREADLINK,
            Rmessage::GetAttr(_) => RGETATTR,
            Rmessage::SetAttr => RSETATTR,
            Rmessage::XattrWalk(_) => RXATTRWALK,
            Rmessage::XattrCreate => RXATTRCREATE,
            Rmessage::Readdir(_) => RREADDIR,
            Rmessage::Fsync => RFSYNC,
            Rmessage::Lock(_) => RLOCK,
            Rmessage::GetLock(_) => RGETLOCK,
            Rmessage::Link => RLINK,
            Rmessage::Mkdir(_) => RMKDIR,
            Rmessage::RenameAt => RRENAMEAT,
            Rmessage::UnlinkAt => RUNLINKAT,
            Rmessage::Lerror(_) => RLERROR,
        }
    }

    fn byte_size(&self) -> u32 {
        match self {
            Rmessage::Version(msg) => msg.byte_size(),
            Rmessage::Flush => 0,
            Rmessage::Walk(msg) => msg.byte_size(),
            Rmessage::Read(msg) => msg.byte_size(),
            Rmessage::Write(msg) => msg.byte_size(),
            Rmessage::Clunk => 0,
            Rmessage::Remove => 0,
            Rmessage::Attach(msg) => msg.byte_size(),
            Rmessage::Auth(msg) => msg.byte_size(),
            Rmessage::Statfs(msg) => msg.byte_size(),
            Rmessage::Lopen(msg) => msg.byte_size(),
            Rmessage::Lcreate(msg) => msg.byte_size(),
            Rmessage::Symlink(msg) => msg.byte_size(),
            Rmessage::Mknod(msg) => msg.byte_size(),
            Rmessage::Rename => 0,
            Rmessage::Readlink(msg) => msg.byte_size(),
            Rmessage::GetAttr(msg) => msg.byte_size(),
            Rmessage::SetAttr => 0,
            Rmessage::XattrWalk(msg) => msg.byte_size(),
            Rmessage::XattrCreate => 0,
            Rmessage::Readdir(msg) => msg.byte_size(),
            Rmessage::Fsync => 0,
            Rmessage::Lock(msg) => msg.byte_size(),
            Rmessage::GetLock(msg) => msg.byte_size(),
            Rmessage::Link => 0,
            Rmessage::Mkdir(msg) => msg.byte_size(),
            Rmessage::RenameAt => 0,
            Rmessage::UnlinkAt => 0,
            Rmessage::Lerror(msg) => msg.byte_size(),
        }
    }

    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match self {
            Rmessage::Version(msg) => msg.encode(writer),
            Rmessage::Flush => Ok(()),
            Rmessage::Walk(msg) => msg.encode(writer),
            Rmessage::Read(msg) => msg.encode(writer),
            Rmessage::Write(msg) => msg.encode(writer),
            Rmessage::Clunk => Ok(()),
            Rmessage::Remove => Ok(()),
            Rmessage::Attach(msg) => msg.encode(writer),
            Rmessage::Auth(msg) => msg.encode(writer),
            Rmessage::Statfs(msg) => msg.encode(writer),
            Rmessage::Lopen(msg) => msg.encode(writer),
            Rmessage::Lcreate(msg) => msg.encode(writer),
            Rmessage::Symlink(msg) => msg.encode(writer),
            Rmessage::Mknod(msg) => msg.encode(writer),
            Rmessage::Rename => Ok(()),
            Rmessage::Readlink(msg) => msg.encode(writer),
            Rmessage::GetAttr(msg) => msg.encode(writer),
            Rmessage::SetAttr => Ok(()),
            Rmessage::XattrWalk(msg) => msg.encode(writer),
            Rmessage::XattrCreate => Ok(()),
            Rmessage::Readdir(msg) => msg.encode(writer),
            Rmessage::Fsync => Ok(()),
            Rmessage::Lock(msg) => msg.encode(writer),
            Rmessage::GetLock(msg) => msg.encode(writer),
            Rmessage::Link => Ok(()),
            Rmessage::Mkdir(msg) => msg.encode(writer),
            Rmessage::RenameAt => Ok(()),
            Rmessage::UnlinkAt => Ok(()),
            Rmessage::Lerror(msg) => msg.encode(writer),
        }
    }

    fn decode<R: std::io::Read>(
        reader: &mut R,
        ty: u8,
    ) -> std::io::Result<Self> {
        match ty {
            RVERSION => Ok(Rmessage::Version(Rversion::decode(reader)?)),
            RFLUSH => Ok(Rmessage::Flush),
            RWALK => Ok(Rmessage::Walk(Rwalk::decode(reader)?)),
            RREAD => Ok(Rmessage::Read(Rread::decode(reader)?)),
            RWRITE => Ok(Rmessage::Write(Rwrite::decode(reader)?)),
            RCLUNK => Ok(Rmessage::Clunk),
            RREMOVE => Ok(Rmessage::Remove),
            RATTACH => Ok(Rmessage::Attach(Rattach::decode(reader)?)),
            RAUTH => Ok(Rmessage::Auth(Rauth::decode(reader)?)),
            RSTATFS => Ok(Rmessage::Statfs(Rstatfs::decode(reader)?)),
            RLOPEN => Ok(Rmessage::Lopen(Rlopen::decode(reader)?)),
            RLCREATE => Ok(Rmessage::Lcreate(Rlcreate::decode(reader)?)),
            RSYMLINK => Ok(Rmessage::Symlink(Rsymlink::decode(reader)?)),
            RMKNOD => Ok(Rmessage::Mknod(Rmknod::decode(reader)?)),
            RRENAME => Ok(Rmessage::Rename),
            RREADLINK => Ok(Rmessage::Readlink(Rreadlink::decode(reader)?)),
            RGETATTR => Ok(Rmessage::GetAttr(Rgetattr::decode(reader)?)),
            RSETATTR => Ok(Rmessage::SetAttr),
            RXATTRWALK => Ok(Rmessage::XattrWalk(Rxattrwalk::decode(reader)?)),
            RXATTRCREATE => Ok(Rmessage::XattrCreate),
            RREADDIR => Ok(Rmessage::Readdir(Rreaddir::decode(reader)?)),
            RFSYNC => Ok(Rmessage::Fsync),
            RLOCK => Ok(Rmessage::Lock(Rlock::decode(reader)?)),
            RGETLOCK => Ok(Rmessage::GetLock(Rgetlock::decode(reader)?)),
            RLINK => Ok(Rmessage::Link),
            RMKDIR => Ok(Rmessage::Mkdir(Rmkdir::decode(reader)?)),
            RRENAMEAT => Ok(Rmessage::RenameAt),
            RUNLINKAT => Ok(Rmessage::UnlinkAt),
            RLERROR => Ok(Rmessage::Lerror(Rlerror::decode(reader)?)),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid message type",
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Server<T>
where
    T: Clone,
{
    inner: T,
}

impl<T> Server<T>
where
    T: Clone,
{
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T> Protocol for Server<T>
where
    T: NineP200L + Clone,
{
    type Error = jetstream_rpc::Error;
    type Request = Tmessage;
    type Response = Rmessage;

    const VERSION: &'static str = "9P2000.L";

    const NAME: &'static str = "9p";
}

impl<T> jetstream_rpc::server::Server for Server<T>
where
    T: NineP200L + Clone,
{
    async fn rpc(
        &mut self,
        _context: Context,
        frame: Frame<Self::Request>,
    ) -> Result<Frame<Self::Response>, Self::Error> {
        let Frame { msg, tag } = frame;
        let rmsg = match msg {
            Tmessage::Version(ref version) => self
                .inner
                .version(tag, version)
                .await
                .map(Rmessage::Version),
            Tmessage::Flush(ref flush) => {
                self.inner.flush(tag, flush).await.and(Ok(Rmessage::Flush))
            }
            Tmessage::Walk(ref walk) => {
                self.inner.walk(tag, walk).await.map(Rmessage::Walk)
            }
            Tmessage::Read(ref read) => {
                self.inner.read(tag, read).await.map(Rmessage::Read)
            }
            Tmessage::Write(ref write) => {
                self.inner.write(tag, write).await.map(Rmessage::Write)
            }
            Tmessage::Clunk(ref clunk) => {
                self.inner.clunk(tag, clunk).await.and(Ok(Rmessage::Clunk))
            }
            Tmessage::Remove(ref remove) => self
                .inner
                .remove(tag, remove)
                .await
                .and(Ok(Rmessage::Remove)),
            Tmessage::Attach(ref attach) => {
                self.inner.attach(tag, attach).await.map(Rmessage::Attach)
            }
            Tmessage::Auth(ref auth) => {
                self.inner.auth(tag, auth).await.map(Rmessage::Auth)
            }
            Tmessage::Statfs(ref statfs) => {
                self.inner.statfs(tag, statfs).await.map(Rmessage::Statfs)
            }
            Tmessage::Lopen(ref lopen) => {
                self.inner.lopen(tag, lopen).await.map(Rmessage::Lopen)
            }
            Tmessage::Lcreate(ref lcreate) => self
                .inner
                .lcreate(tag, lcreate)
                .await
                .map(Rmessage::Lcreate),
            Tmessage::Symlink(ref symlink) => self
                .inner
                .symlink(tag, symlink)
                .await
                .map(Rmessage::Symlink),
            Tmessage::Mknod(ref mknod) => {
                self.inner.mknod(tag, mknod).await.map(Rmessage::Mknod)
            }
            Tmessage::Rename(ref rename) => self
                .inner
                .rename(tag, rename)
                .await
                .and(Ok(Rmessage::Rename)),
            Tmessage::Readlink(ref readlink) => self
                .inner
                .readlink(tag, readlink)
                .await
                .map(Rmessage::Readlink),
            Tmessage::GetAttr(ref get_attr) => self
                .inner
                .get_attr(tag, get_attr)
                .await
                .map(Rmessage::GetAttr),
            Tmessage::SetAttr(ref set_attr) => self
                .inner
                .set_attr(tag, set_attr)
                .await
                .and(Ok(Rmessage::SetAttr)),
            Tmessage::XattrWalk(ref xattr_walk) => self
                .inner
                .xattr_walk(tag, xattr_walk)
                .await
                .map(Rmessage::XattrWalk),
            Tmessage::XattrCreate(ref xattr_create) => self
                .inner
                .xattr_create(tag, xattr_create)
                .await
                .and(Ok(Rmessage::XattrCreate)),
            Tmessage::Readdir(ref readdir) => self
                .inner
                .readdir(tag, readdir)
                .await
                .map(Rmessage::Readdir),
            Tmessage::Fsync(ref fsync) => {
                self.inner.fsync(tag, fsync).await.and(Ok(Rmessage::Fsync))
            }
            Tmessage::Lock(ref lock) => {
                self.inner.lock(tag, lock).await.map(Rmessage::Lock)
            }
            Tmessage::GetLock(ref get_lock) => self
                .inner
                .get_lock(tag, get_lock)
                .await
                .map(Rmessage::GetLock),
            Tmessage::Link(ref link) => {
                self.inner.link(tag, link).await.and(Ok(Rmessage::Link))
            }
            Tmessage::Mkdir(ref mkdir) => {
                self.inner.mkdir(tag, mkdir).await.map(Rmessage::Mkdir)
            }
            Tmessage::RenameAt(ref rename_at) => self
                .inner
                .rename_at(tag, rename_at)
                .await
                .and(Ok(Rmessage::RenameAt)),
            Tmessage::UnlinkAt(ref unlink_at) => self
                .inner
                .unlink_at(tag, unlink_at)
                .await
                .and(Ok(Rmessage::UnlinkAt)),
        };
        match rmsg {
            Ok(msg) => Ok(Frame { tag, msg }),
            Err(e) => Ok(Frame {
                tag,
                msg: Rmessage::Lerror(error_to_rmessage(&e)),
            }),
        }
    }
}

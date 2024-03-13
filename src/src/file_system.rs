use std::{
    collections::btree_map,
    io::{self},
    marker::PhantomData,
    pin::Pin,
};

use futures_util::Future;
use p9::{Data, Rframe, WireFormat};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::async_wire_format::{AsyncWireFormat, AsyncWireFormatExt};

use super::ninep_2000_l::NineP200L;

#[async_trait::async_trait]
trait Stat {
    async fn stat(&mut self) -> p9::Rgetattr;
}

#[async_trait::async_trait]
trait File: AsyncRead + AsyncWrite + Stat {
    async fn read(&mut self, msg: &p9::Tread) -> io::Result<p9::Rread>;
    async fn write(&mut self, msg: &p9::Twrite) -> io::Result<p9::Rwrite>;
    async fn flush(&mut self, _msg: &p9::Tflush) -> io::Result<()>;
    async fn stat(&mut self, msg: &p9::Tgetattr) -> io::Result<p9::Rgetattr>;
}

#[async_trait::async_trait]
trait FileExt: File
where
    Self: Sized + Send + Sync + Unpin,
{
    async fn read(&mut self, msg: &p9::Tread) -> io::Result<p9::Rread> {
        let mut buf = vec![0; msg.count as usize];
        let _n = self.read_exact(buf.as_mut_slice()).await?;
        Ok(p9::Rread { data: Data(buf) })
    }

    async fn write(&mut self, msg: &p9::Twrite) -> io::Result<p9::Rwrite> {
        self.write_all(&msg.data.0).await?;
        Ok(p9::Rwrite {
            count: msg.data.0.len() as u32,
        })
    }

    async fn flush(&mut self, _msg: &p9::Tflush) -> io::Result<()> {
        AsyncWriteExt::flush(&mut self).await
    }

    async fn stat(&mut self, _msg: &p9::Tgetattr) -> io::Result<p9::Rgetattr> {
        Ok(p9::Rgetattr {
            valid: 0,
            qid: p9::Qid {
                ty: 0,
                version: 0,
                path: 0,
            },
            mode: 0,
            uid: 0,
            gid: 0,
            nlink: 0,
            rdev: 0,
            size: 0,
            blksize: 0,
            blocks: 0,
            atime_sec: 0,
            atime_nsec: 0,
            mtime_sec: 0,
            mtime_nsec: 0,
            ctime_sec: 0,
            ctime_nsec: 0,
            btime_sec: 0,
            btime_nsec: 0,
            gen: 0,
            data_version: 0,
        })
    }
}

#[async_trait::async_trait]
trait Dir {
    async fn open(&mut self, msg: &p9::Tlopen) -> io::Result<p9::Rlopen>;
    async fn create(&mut self, msg: &p9::Tlcreate) -> io::Result<p9::Rlcreate>;
    async fn remove(&mut self, msg: &p9::Tremove) -> io::Result<()>;
    async fn stat(&mut self, msg: &p9::Tgetattr) -> io::Result<p9::Rgetattr>;
}

#[async_trait::async_trait]
trait DirExt: Dir
where
    Self: Sized + Send + Sync + Unpin,
{
    async fn open(&mut self, _msg: &p9::Tlopen) -> io::Result<p9::Rlopen> {
        Ok(p9::Rlopen {
            qid: p9::Qid {
                ty: 0,
                version: 0,
                path: 0,
            },
            iounit: 0,
        })
    }

    async fn create(
        &mut self,
        _msg: &p9::Tlcreate,
    ) -> io::Result<p9::Rlcreate> {
        Ok(p9::Rlcreate {
            qid: p9::Qid {
                ty: 0,
                version: 0,
                path: 0,
            },
            iounit: 0,
        })
    }

    async fn remove(&mut self, _msg: &p9::Tremove) -> io::Result<()> {
        Ok(())
    }

    async fn stat(&mut self, _msg: &p9::Tgetattr) -> io::Result<p9::Rgetattr> {
        Ok(p9::Rgetattr {
            valid: 0,
            qid: p9::Qid {
                ty: 0,
                version: 0,
                path: 0,
            },
            mode: 0,
            uid: 0,
            gid: 0,
            nlink: 0,
            rdev: 0,
            size: 0,
            blksize: 0,
            blocks: 0,
            atime_sec: 0,
            atime_nsec: 0,
            mtime_sec: 0,
            mtime_nsec: 0,
            ctime_sec: 0,
            ctime_nsec: 0,
            btime_sec: 0,
            btime_nsec: 0,
            gen: 0,
            data_version: 0,
        })
    }
}

enum Node<F: File, D: Dir> {
    File(F),
    Dir(D),
    Empty
}
#[derive(Eq, Clone)]
struct Fid {
    inner: u32,
    _phantom: PhantomData<()>,
}

impl PartialEq for Fid {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl  std::hash::Hash for Fid {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl std::cmp::PartialOrd for Fid {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl std::cmp::Ord for Fid {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

struct FileSystem<F: File, D: Dir> {
    fids: btree_map::BTreeMap<Fid, String>,
    nodes: btree_map::BTreeMap<Fid, Node<F, D>>,
}

impl <F: File, D: Dir> FileSystem<F, D> {
    fn new() -> Self {
        Self {
            fids: btree_map::BTreeMap::new(),
            nodes: btree_map::BTreeMap::new(),
        }
    }
}

impl<F: File, D: Dir> FileSystem<F, D> {
    async fn attach(&mut self, msg: &p9::Tattach) -> io::Result<p9::Rattach> {
        let fid = Fid {
            inner: msg.fid,
            _phantom: PhantomData,
        };
        self.fids.insert(fid.clone(), msg.uname.clone());
        self.nodes.insert(fid, Node::Empty);
        Ok(p9::Rattach {
            qid: p9::Qid {
                ty: 0,
                version: 0,
                path: 0,
            },
        })
    }
}
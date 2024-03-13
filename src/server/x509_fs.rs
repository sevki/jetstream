use std::io;

use jetstream_p9::messages::*;

use crate::server::ninep_2000_l::NineP200L;

struct X509Fs;

impl NineP200L for X509Fs {
    /// The version message is the first message sent on a connection. It is used to negotiate the
    /// 9P protocol version and maximum message size.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn version<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _version: &'life1 Tversion,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rversion>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The auth message is used to authenticate a user to the server. It is sent after the version
    /// message and before any other messages.
    /// The auth message is optional and may be ignored by the server.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn auth<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _auth: &'life1 Tauth,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rauth>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The flush message is used to flush pending I/O requests.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn flush<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _flush: &'life1 Tflush,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<()>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The walk message is used to traverse the file system hierarchy. It is sent by the client and
    /// responded to by the server.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn walk<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _walk: &'life1 Twalk,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rwalk>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The read message is used to read data from a file.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn read<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _read: &'life1 Tread,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rread>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The write message is used to write data to a file.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn write<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _write: &'life1 Twrite,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rwrite>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The clunk message is used to release a fid.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn clunk<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _clunk: &'life1 Tclunk,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<()>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The remove message is used to remove a file.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn remove<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _remove: &'life1 Tremove,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<()>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The attach message is used to associate a fid with a file.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn attach<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _attach: &'life1 Tattach,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rattach>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The statfs message is used to retrieve file system information.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn statfs<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _statfs: &'life1 Tstatfs,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rstatfs>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The lopen message is used to open a file.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn lopen<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _lopen: &'life1 Tlopen,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rlopen>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The lcreate message is used to create a file.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn lcreate<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _lcreate: &'life1 Tlcreate,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rlcreate>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The symlink message is used to create a symbolic link.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn symlink<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _symlink: &'life1 Tsymlink,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rsymlink>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The mknod message is used to create a device file.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn mknod<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _mknod: &'life1 Tmknod,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rmknod>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The rename message is used to rename a file.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn rename<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _rename: &'life1 Trename,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<()>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The readlink message is used to read the target of a symbolic link.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn readlink<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _readlink: &'life1 Treadlink,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rreadlink>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The getattr message is used to retrieve file attributes.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn get_attr<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _get_attr: &'life1 Tgetattr,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rgetattr>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The setattr message is used to set file attributes.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn set_attr<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _set_attr: &'life1 Tsetattr,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<()>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The xattrwalk message is used to traverse extended attributes.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn xattr_walk<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _xattr_walk: &'life1 Txattrwalk,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rxattrwalk>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The xattrcreate message is used to create an extended attribute.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn xattr_create<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _xattr_create: &'life1 Txattrcreate,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<()>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The readdir message is used to read a directory.   
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn readdir<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _readdir: &'life1 Treaddir,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rreaddir>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The fsync message is used to synchronize a file\'s data and metadata.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn fsync<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _fsync: &'life1 Tfsync,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<()>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The lock message is used to lock a file.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn lock<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _lock: &'life1 Tlock,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rlock>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The getlock message is used to retrieve a file\'s locks.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn get_lock<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _get_lock: &'life1 Tgetlock,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rgetlock>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The link message is used to create a hard link.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn link<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _link: &'life1 Tlink,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<()>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The mkdir message is used to create a directory.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn mkdir<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _mkdir: &'life1 Tmkdir,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<Rmkdir>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The renameat message is used to rename a file.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn rename_at<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _rename_at: &'life1 Trenameat,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<()>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }

    /// The unlinkat message is used to remove a file.
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn unlink_at<'life0, 'life1, 'async_trait>(
        self: &'life0 mut Self,
        _tag: u16,
        _unlink_at: &'life1 Tunlinkat,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = io::Result<()>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        todo!()
    }
}

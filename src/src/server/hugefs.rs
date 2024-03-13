// hugefs is a very FAST and efficient file system designed to scale to very large file systems.

// The current schema uses the build the dumbest version and see where it fails, apply learnings and iterate.
//
// Storage (we are only concerned with storage, transport is already very fast)
// -------
// The dumbest and probably the slowest path to store a data is this,
//  storefile(path, data):
//    for i, chunk in fastCDC(data).await {
//     storechunk(path, i chunk)
//    }
//
// storechunk(path, n, chunk):
//   hash = blake3(chunk)
//   s3.put(hash, chunk)
//   db.exec(
//     if !path in db:
//       db[path] = []
//     db[path].insert_at(n, hash)
//  )
//
// Something, roughly like this, and the corresponding reads.

use p9::WireFormat;

use crate::P9WireFormat;

#[derive(Debug, P9WireFormat)]
pub struct Id {
    data: p9::Data,
}

impl PartialEq for Id {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

// tests
#[cfg(test)]
mod tests {

    use clap::builder::ValueParserFactory;

    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_id() {
        // sha256("hello")

        let id = Id {
            data: p9::Data(vec![
                0x9b, 0x71, 0x7c, 0x3c, 0x7b, 0x8b, 0x7e, 0x5f, 0x7f, 0x8b,
            ]),
        };
    }
}

#[cfg(test)]
mod tests {
    use jetstream_9p::*;
    use jetstream_rpc::*;
    use std::io;

    #[test]
    fn test_jetstream_9p_input_validation() {
        // Test for invalid input in Tversion
        let invalid_version = Tversion {
            msize: 0,
            version: String::from(""),
        };
        let result = invalid_version.encode(&mut Vec::new());
        assert!(result.is_err());

        // Test for invalid input in Tflush
        let invalid_flush = Tflush { oldtag: 0 };
        let result = invalid_flush.encode(&mut Vec::new());
        assert!(result.is_err());

        // Test for invalid input in Twalk
        let invalid_walk = Twalk {
            fid: 0,
            newfid: 0,
            wnames: vec![String::from("")],
        };
        let result = invalid_walk.encode(&mut Vec::new());
        assert!(result.is_err());

        // Test for invalid input in Tread
        let invalid_read = Tread {
            fid: 0,
            offset: 0,
            count: 0,
        };
        let result = invalid_read.encode(&mut Vec::new());
        assert!(result.is_err());

        // Test for invalid input in Twrite
        let invalid_write = Twrite {
            fid: 0,
            offset: 0,
            data: Data(Vec::new()),
        };
        let result = invalid_write.encode(&mut Vec::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_jetstream_rpc_input_validation() {
        // Test for invalid input in Message trait
        struct InvalidMessage;
        impl Message for InvalidMessage {}
        impl WireFormat for InvalidMessage {
            fn byte_size(&self) -> u32 {
                0
            }
            fn encode<W: io::Write>(&self, _writer: &mut W) -> io::Result<()> {
                Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid message"))
            }
            fn decode<R: io::Read>(_reader: &mut R) -> io::Result<Self> {
                Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid message"))
            }
        }

        let invalid_message = InvalidMessage;
        let result = invalid_message.encode(&mut Vec::new());
        assert!(result.is_err());
    }
}

/// Creates a newtype wrapper around a prost Message type that implements WireFormat.
///
/// # Examples
///
/// ```rust
/// use jetstream::prelude::*;
/// use jetstream::prost_wireformat;
/// use prost::Message;
///
/// // Define a simple prost message for testing
/// #[derive(Clone, PartialEq, Message)]
/// pub struct TestMessage {
///     #[prost(uint64, tag = "1")]
///     pub id: u64,
///     #[prost(string, tag = "2")]
///     pub name: String,
///     #[prost(bytes = "vec", tag = "3")]
///     pub data: Vec<u8>,
/// }
///
/// // Public struct, private inner field and accessors
/// prost_wireformat!(pub TestMessage as MyMessage);
///
/// // Public struct with public inner field and accessors
/// prost_wireformat!(pub TestMessage as pub YourMessage);
///
/// // Crate-visible struct with public inner access
/// prost_wireformat!(pub(crate) TestMessage as pub OurMessage);
///
/// // With derives - adds Clone and Debug to the wrapper
/// prost_wireformat!(pub TestMessage as CloneableMessage, derive(Clone, Debug));
/// ```
#[macro_export]
macro_rules! prost_wireformat {
    // With derives: prost_wireformat!(pub TestMessage as MyMessage, derives(Clone, Debug));
    ($wrapper_vis:vis $wrapped_type:ty as $vis:vis $new_type:ident, derive($($derives:path),* $(,)?)) => {
        #[derive($($derives),*)]
        $wrapper_vis struct $new_type($vis $wrapped_type);

        $crate::prost_wireformat!(@impl $wrapped_type, $vis, $new_type);
    };

    // Without derives: prost_wireformat!(pub TestMessage as MyMessage);
    ($wrapper_vis:vis $wrapped_type:ty as $vis:vis $new_type:ident) => {
        $wrapper_vis struct $new_type($vis $wrapped_type);

        $crate::prost_wireformat!(@impl $wrapped_type, $vis, $new_type);
    };

    // Internal: shared implementation
    (@impl $wrapped_type:ty, $vis:vis, $new_type:ident) => {
        impl $new_type {
            $vis fn new(inner: $wrapped_type) -> Self {
                Self(inner)
            }

            $vis fn into_inner(self) -> $wrapped_type {
                self.0
            }

            $vis fn inner(&self) -> &$wrapped_type {
                &self.0
            }
        }

        impl $crate::prelude::WireFormat for $new_type {
            fn byte_size(&self) -> u32 {
                std::mem::size_of::<usize>() as u32
                    + self.0.encoded_len() as u32
            }

            fn encode<W: std::io::Write>(
                &self,
                writer: &mut W,
            ) -> std::io::Result<()> {
                let len = self.0.encoded_len();
                len.encode(writer)?;
                let mut buf = Vec::with_capacity(len);
                self.0
                    .encode(&mut buf)
                    .map_err(std::io::Error::other)?;
                writer.write_all(&buf)
            }

            fn decode<R: std::io::Read>(
                reader: &mut R,
            ) -> std::io::Result<Self> {
                let len = usize::decode(reader)?;
                let mut bytes = vec![0; len];
                reader.read_exact(&mut bytes)?;
                <$wrapped_type as prost::Message>::decode(bytes.as_slice())
                    .map($new_type)
                    .map_err(std::io::Error::other)
            }
        }
    };
}

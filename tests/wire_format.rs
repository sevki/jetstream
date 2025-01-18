use jetstream_9p::*;
use jetstream_macros::JetStreamWireFormat;
use jetstream_wireformat::*;
use std::io::{self, Cursor};
use std::mem;
use std::string::String;
use std::{pin::Pin, time::Duration};
use tokio::io::{AsyncRead, AsyncWrite};
#[allow(unused_imports)]
use tokio::time::sleep;
use wire_format_extensions::tokio::AsyncWireFormatExt;

use wire_format_extensions::ConvertWireFormat;

#[test]
fn integer_byte_size() {
    assert_eq!(1, 0u8.byte_size());
    assert_eq!(2, 0u16.byte_size());
    assert_eq!(4, 0u32.byte_size());
    assert_eq!(8, 0u64.byte_size());
}

#[test]
fn integer_decode() {
    let buf: [u8; 8] = [0xef, 0xbe, 0xad, 0xde, 0x0d, 0xf0, 0xad, 0x8b];

    assert_eq!(0xef_u8, u8::decode(&mut Cursor::new(&buf)).unwrap());
    assert_eq!(0xbeef_u16, u16::decode(&mut Cursor::new(&buf)).unwrap());
    assert_eq!(0xdeadbeef_u32, u32::decode(&mut Cursor::new(&buf)).unwrap());
    assert_eq!(
        0x8bad_f00d_dead_beef_u64,
        u64::decode(&mut Cursor::new(&buf)).unwrap()
    );
}

#[test]
fn integer_encode() {
    let value: u64 = 0x8bad_f00d_dead_beef;
    let expected: [u8; 8] = [0xef, 0xbe, 0xad, 0xde, 0x0d, 0xf0, 0xad, 0x8b];

    let mut buf = vec![0; 8];

    (value as u8).encode(&mut Cursor::new(&mut *buf)).unwrap();
    assert_eq!(expected[0..1], buf[0..1]);

    (value as u16).encode(&mut Cursor::new(&mut *buf)).unwrap();
    assert_eq!(expected[0..2], buf[0..2]);

    (value as u32).encode(&mut Cursor::new(&mut *buf)).unwrap();
    assert_eq!(expected[0..4], buf[0..4]);

    value.encode(&mut Cursor::new(&mut *buf)).unwrap();
    assert_eq!(expected[0..8], buf[0..8]);
}

#[test]
fn string_byte_size() {
    let values = [
        String::from("Google Video"),
        String::from("网页 图片 资讯更多 »"),
        String::from("Παγκόσμιος Ιστός"),
        String::from("Поиск страниц на русском"),
        String::from("전체서비스"),
    ];

    let exp = values
        .iter()
        .map(|v| (mem::size_of::<u16>() + v.len()) as u32);

    for (value, expected) in values.iter().zip(exp) {
        assert_eq!(expected, value.byte_size());
    }
}

#[test]
fn zero_length_string() {
    let s = String::from("");
    assert_eq!(s.byte_size(), mem::size_of::<u16>() as u32);

    let mut buf = [0xffu8; 4];

    s.encode(&mut Cursor::new(&mut buf[..]))
        .expect("failed to encode empty string");
    assert_eq!(&[0, 0, 0xff, 0xff], &buf);

    assert_eq!(
        s,
        <String as WireFormat>::decode(&mut Cursor::new(
            &[0, 0, 0x61, 0x61][..]
        ))
        .expect("failed to decode empty string")
    );
}

#[test]
fn string_encode() {
    let values = [
        String::from("Google Video"),
        String::from("网页 图片 资讯更多 »"),
        String::from("Παγκόσμιος Ιστός"),
        String::from("Поиск страниц на русском"),
        String::from("전체서비스"),
    ];

    let expected = values.iter().map(|v| {
        let len = v.len();
        let mut buf = Vec::with_capacity(len + mem::size_of::<u16>());

        buf.push(len as u8);
        buf.push((len >> 8) as u8);

        buf.extend_from_slice(v.as_bytes());

        buf
    });

    for (val, exp) in values.iter().zip(expected) {
        let mut buf = vec![0; exp.len()];

        WireFormat::encode(val, &mut Cursor::new(&mut *buf)).unwrap();
        assert_eq!(exp, buf);
    }
}

#[test]
fn string_decode() {
    assert_eq!(
        String::from("Google Video"),
        <String as WireFormat>::decode(&mut Cursor::new(
            &[
                0x0c, 0x00, 0x47, 0x6F, 0x6F, 0x67, 0x6C, 0x65, 0x20, 0x56,
                0x69, 0x64, 0x65, 0x6F,
            ][..]
        ))
        .unwrap()
    );
    assert_eq!(
        String::from("网页 图片 资讯更多 »"),
        <String as WireFormat>::decode(&mut Cursor::new(
            &[
                0x1d, 0x00, 0xE7, 0xBD, 0x91, 0xE9, 0xA1, 0xB5, 0x20, 0xE5,
                0x9B, 0xBE, 0xE7, 0x89, 0x87, 0x20, 0xE8, 0xB5, 0x84, 0xE8,
                0xAE, 0xAF, 0xE6, 0x9B, 0xB4, 0xE5, 0xA4, 0x9A, 0x20, 0xC2,
                0xBB,
            ][..]
        ))
        .unwrap()
    );
    assert_eq!(
        String::from("Παγκόσμιος Ιστός"),
        <String as WireFormat>::decode(&mut Cursor::new(
            &[
                0x1f, 0x00, 0xCE, 0xA0, 0xCE, 0xB1, 0xCE, 0xB3, 0xCE, 0xBA,
                0xCF, 0x8C, 0xCF, 0x83, 0xCE, 0xBC, 0xCE, 0xB9, 0xCE, 0xBF,
                0xCF, 0x82, 0x20, 0xCE, 0x99, 0xCF, 0x83, 0xCF, 0x84, 0xCF,
                0x8C, 0xCF, 0x82,
            ][..]
        ))
        .unwrap()
    );
    assert_eq!(
        String::from("Поиск страниц на русском"),
        <String as WireFormat>::decode(&mut Cursor::new(
            &[
                0x2d, 0x00, 0xD0, 0x9F, 0xD0, 0xBE, 0xD0, 0xB8, 0xD1, 0x81,
                0xD0, 0xBA, 0x20, 0xD1, 0x81, 0xD1, 0x82, 0xD1, 0x80, 0xD0,
                0xB0, 0xD0, 0xBD, 0xD0, 0xB8, 0xD1, 0x86, 0x20, 0xD0, 0xBD,
                0xD0, 0xB0, 0x20, 0xD1, 0x80, 0xD1, 0x83, 0xD1, 0x81, 0xD1,
                0x81, 0xD0, 0xBA, 0xD0, 0xBE, 0xD0, 0xBC,
            ][..]
        ))
        .unwrap()
    );
    assert_eq!(
        String::from("전체서비스"),
        <String as WireFormat>::decode(&mut Cursor::new(
            &[
                0x0f, 0x00, 0xEC, 0xA0, 0x84, 0xEC, 0xB2, 0xB4, 0xEC, 0x84,
                0x9C, 0xEB, 0xB9, 0x84, 0xEC, 0x8A, 0xA4,
            ][..]
        ))
        .unwrap()
    );
}

#[test]
fn invalid_string_decode() {
    let _ = <String as WireFormat>::decode(&mut Cursor::new(&[
        0x06, 0x00, 0xed, 0xa0, 0x80, 0xed, 0xbf, 0xbf,
    ]))
    .expect_err("surrogate code point");

    let _ = <String as WireFormat>::decode(&mut Cursor::new(&[
        0x05, 0x00, 0xf8, 0x80, 0x80, 0x80, 0xbf,
    ]))
    .expect_err("overlong sequence");

    let _ = <String as WireFormat>::decode(&mut Cursor::new(&[
        0x04, 0x00, 0xf4, 0x90, 0x80, 0x80,
    ]))
    .expect_err("out of range");

    let _ = <String as WireFormat>::decode(&mut Cursor::new(&[
        0x04, 0x00, 0x63, 0x61, 0x66, 0xe9,
    ]))
    .expect_err("ISO-8859-1");

    let _ = <String as WireFormat>::decode(&mut Cursor::new(&[
        0x04, 0x00, 0xb0, 0xa1, 0xb0, 0xa2,
    ]))
    .expect_err("EUC-KR");
}

#[test]
fn vector_encode() {
    let values: Vec<u32> =
        vec![291, 18_916, 2_497, 22, 797_162, 2_119_732, 3_213_929_716];
    let mut expected: Vec<u8> = Vec::with_capacity(
        values.len() * mem::size_of::<u32>() + mem::size_of::<u16>(),
    );
    expected.push(values.len() as u8);
    expected.push((values.len() >> 8) as u8);

    const MASK: u32 = 0xff;
    for val in &values {
        expected.push((val & MASK) as u8);
        expected.push(((val >> 8) & MASK) as u8);
        expected.push(((val >> 16) & MASK) as u8);
        expected.push(((val >> 24) & MASK) as u8);
    }

    let mut actual: Vec<u8> = vec![0; expected.len()];

    WireFormat::encode(&values, &mut Cursor::new(&mut *actual))
        .expect("failed to encode vector");
    assert_eq!(expected, actual);
}

#[test]
fn vector_decode() {
    let expected: Vec<u32> = vec![
        2_498,
        24,
        897,
        4_097_789_579,
        8_498_119,
        684_279,
        961_189_198,
        7,
    ];
    let mut input: Vec<u8> = Vec::with_capacity(
        expected.len() * mem::size_of::<u32>() + mem::size_of::<u16>(),
    );
    input.push(expected.len() as u8);
    input.push((expected.len() >> 8) as u8);

    const MASK: u32 = 0xff;
    for val in &expected {
        input.push((val & MASK) as u8);
        input.push(((val >> 8) & MASK) as u8);
        input.push(((val >> 16) & MASK) as u8);
        input.push(((val >> 24) & MASK) as u8);
    }

    assert_eq!(
        expected,
        <Vec<u32> as WireFormat>::decode(&mut Cursor::new(&*input))
            .expect("failed to decode vector")
    );
}

#[test]
fn data_encode() {
    let values = Data(vec![169, 155, 79, 67, 182, 199, 25, 73, 129, 200]);
    let mut expected: Vec<u8> = Vec::with_capacity(
        values.len() * mem::size_of::<u8>() + mem::size_of::<u32>(),
    );
    expected.push(values.len() as u8);
    expected.push((values.len() >> 8) as u8);
    expected.push((values.len() >> 16) as u8);
    expected.push((values.len() >> 24) as u8);
    expected.extend_from_slice(&values);

    let mut actual: Vec<u8> = vec![0; expected.len()];

    WireFormat::encode(&values, &mut Cursor::new(&mut *actual))
        .expect("failed to encode datar");
    assert_eq!(expected, actual);
}

#[test]
fn data_decode() {
    let expected = Data(vec![219, 15, 8, 155, 194, 129, 79, 91, 46, 53, 173]);
    let mut input: Vec<u8> = Vec::with_capacity(
        expected.len() * mem::size_of::<u8>() + mem::size_of::<u32>(),
    );
    input.push(expected.len() as u8);
    input.push((expected.len() >> 8) as u8);
    input.push((expected.len() >> 16) as u8);
    input.push((expected.len() >> 24) as u8);
    input.extend_from_slice(&expected);

    assert_eq!(
        expected,
        <Data as WireFormat>::decode(&mut Cursor::new(&mut *input))
            .expect("failed to decode data")
    );
}

#[test]
fn error_cases() {
    // string is too long.
    let mut long_str = String::with_capacity(u16::MAX as usize);
    while long_str.len() < u16::MAX as usize {
        long_str.push_str("long");
    }
    long_str.push('!');

    let count = long_str.len() + mem::size_of::<u16>();
    let mut buf = vec![0; count];

    long_str
        .encode(&mut Cursor::new(&mut *buf))
        .expect_err("long string");

    // vector is too long.
    let mut long_vec: Vec<u32> = Vec::with_capacity(u16::MAX as usize);
    while long_vec.len() < u16::MAX as usize {
        long_vec.push(0x8bad_f00d);
    }
    long_vec.push(0x00ba_b10c);

    let count = long_vec.len() * mem::size_of::<u32>();
    let mut buf = vec![0; count];

    WireFormat::encode(&long_vec, &mut Cursor::new(&mut *buf))
        .expect_err("long vector");
}

#[derive(Debug, PartialEq, JetStreamWireFormat)]
struct Item {
    a: u64,
    b: String,
    c: Vec<u16>,
    buf: Data,
}

#[test]
fn struct_encode() {
    let item = Item {
        a: 0xdead_10cc_00ba_b10c,
        b: String::from("冻住，不许走!"),
        c: vec![359, 492, 8891],
        buf: Data(vec![254, 129, 0, 62, 49, 172]),
    };

    let mut expected: Vec<u8> =
        vec![0x0c, 0xb1, 0xba, 0x00, 0xcc, 0x10, 0xad, 0xde];
    let strlen = item.b.len() as u16;
    expected.push(strlen as u8);
    expected.push((strlen >> 8) as u8);
    expected.extend_from_slice(item.b.as_bytes());

    let veclen = item.c.len() as u16;
    expected.push(veclen as u8);
    expected.push((veclen >> 8) as u8);
    for val in &item.c {
        expected.push(*val as u8);
        expected.push((val >> 8) as u8);
    }

    let buflen = item.buf.len() as u32;
    expected.push(buflen as u8);
    expected.push((buflen >> 8) as u8);
    expected.push((buflen >> 16) as u8);
    expected.push((buflen >> 24) as u8);
    expected.extend_from_slice(&item.buf);

    let mut actual = vec![0; expected.len()];

    WireFormat::encode(&item, &mut Cursor::new(&mut *actual))
        .expect("failed to encode item");

    assert_eq!(expected, actual);
}

#[test]
fn struct_decode() {
    let expected = Item {
        a: 0xface_b00c_0404_4b1d,
        b: String::from("Огонь по готовности!"),
        c: vec![20067, 32449, 549, 4972, 77, 1987],
        buf: Data(vec![126, 236, 79, 59, 6, 159]),
    };

    let mut input: Vec<u8> =
        vec![0x1d, 0x4b, 0x04, 0x04, 0x0c, 0xb0, 0xce, 0xfa];
    let strlen = expected.b.len() as u16;
    input.push(strlen as u8);
    input.push((strlen >> 8) as u8);
    input.extend_from_slice(expected.b.as_bytes());

    let veclen = expected.c.len() as u16;
    input.push(veclen as u8);
    input.push((veclen >> 8) as u8);
    for val in &expected.c {
        input.push(*val as u8);
        input.push((val >> 8) as u8);
    }

    let buflen = expected.buf.len() as u32;
    input.push(buflen as u8);
    input.push((buflen >> 8) as u8);
    input.push((buflen >> 16) as u8);
    input.push((buflen >> 24) as u8);
    input.extend_from_slice(&expected.buf);

    let actual: Item = WireFormat::decode(&mut Cursor::new(input))
        .expect("failed to decode item");

    assert_eq!(expected, actual);
}

#[derive(Debug, PartialEq, JetStreamWireFormat)]
struct Nested {
    item: Item,
    val: Vec<u64>,
}

#[allow(clippy::vec_init_then_push)]
fn build_encoded_buffer(value: &Nested) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();

    // encode a
    result.push(value.item.a as u8);
    result.push((value.item.a >> 8) as u8);
    result.push((value.item.a >> 16) as u8);
    result.push((value.item.a >> 24) as u8);
    result.push((value.item.a >> 32) as u8);
    result.push((value.item.a >> 40) as u8);
    result.push((value.item.a >> 48) as u8);
    result.push((value.item.a >> 56) as u8);

    // encode b
    result.push(value.item.b.len() as u8);
    result.push((value.item.b.len() >> 8) as u8);
    result.extend_from_slice(value.item.b.as_bytes());

    // encode c
    result.push(value.item.c.len() as u8);
    result.push((value.item.c.len() >> 8) as u8);
    for val in &value.item.c {
        result.push((val & 0xffu16) as u8);
        result.push(((val >> 8) & 0xffu16) as u8);
    }

    // encode buf
    result.push(value.item.buf.len() as u8);
    result.push((value.item.buf.len() >> 8) as u8);
    result.push((value.item.buf.len() >> 16) as u8);
    result.push((value.item.buf.len() >> 24) as u8);
    result.extend_from_slice(&value.item.buf);

    // encode val
    result.push(value.val.len() as u8);
    result.push((value.val.len() >> 8) as u8);
    for val in &value.val {
        result.push(*val as u8);
        result.push((val >> 8) as u8);
        result.push((val >> 16) as u8);
        result.push((val >> 24) as u8);
        result.push((val >> 32) as u8);
        result.push((val >> 40) as u8);
        result.push((val >> 48) as u8);
        result.push((val >> 56) as u8);
    }

    result
}

#[test]
fn nested_encode() {
    let value = Nested {
        item: Item {
            a: 0xcafe_d00d_8bad_f00d,
            b: String::from("龍が我が敵を喰らう!"),
            c: vec![2679, 55_919, 44, 38_819, 792],
            buf: Data(vec![129, 55, 200, 93, 7, 68]),
        },
        val: vec![1954978, 59, 4519, 15679],
    };

    let expected = build_encoded_buffer(&value);

    let mut actual = vec![0; expected.len()];

    WireFormat::encode(&value, &mut Cursor::new(&mut *actual))
        .expect("failed to encode value");
    assert_eq!(expected, actual);
}

#[test]
fn nested_decode() {
    let expected = Nested {
        item: Item {
            a: 0x0ff1ce,
            b: String::from("龍神の剣を喰らえ!"),
            c: vec![21687, 159, 55, 9217, 192],
            buf: Data(vec![189, 22, 7, 59, 235]),
        },
        val: vec![15679, 8619196, 319746, 123957, 77, 0, 492],
    };

    let input = build_encoded_buffer(&expected);

    assert_eq!(
        expected,
        <Nested as WireFormat>::decode(&mut Cursor::new(&*input))
            .expect("failed to decode value")
    );
}

#[test]
fn test_io_read_for_data() {
    let mut data = Data(vec![0x8b, 0xad, 0xf0, 0x0d, 0x0d, 0xf0, 0xad, 0x8b]);
    let mut buf = [0; 8];

    let _ = io::Read::read_exact(&mut data, &mut buf);

    assert_eq!(buf, [0x8b, 0xad, 0xf0, 0x0d, 0x0d, 0xf0, 0xad, 0x8b]);
}

struct BlockingIO<T: Sized + Unpin> {
    delay: Duration,
    inner: T,
}

impl BlockingIO<tokio::io::DuplexStream> {
    #[allow(dead_code)]
    fn new(delay: Duration, inner: tokio::io::DuplexStream) -> Self {
        Self { delay, inner }
    }
}

impl<T> AsyncRead for BlockingIO<T>
where
    T: AsyncRead + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        let delay = self.delay;

        // If there's a delay, we schedule a sleep before proceeding with the read.
        if delay > Duration::ZERO {
            // This future will complete after the specified delay.
            tokio::spawn(async move {
                sleep(delay).await;
            });
        }
        let this = self.get_mut();
        // Poll the inner AsyncRead.
        Pin::new(&mut this.inner).poll_read(cx, buf)
    }
}

impl<T> AsyncWrite for BlockingIO<T>
where
    T: AsyncWrite + Unpin,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        let delay = self.delay;

        // If there's a delay, we schedule a sleep before proceeding with the write.
        if delay > Duration::ZERO {
            // This future will complete after the specified delay.
            tokio::spawn(async move {
                sleep(delay).await;
            });
        }
        let this = self.get_mut();
        // Poll the inner AsyncWrite.
        Pin::new(&mut this.inner).poll_write(cx, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        let this = self.get_mut();
        Pin::new(&mut this.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        let this = self.get_mut();
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_async_wire_format() {
    let test = Tframe {
        tag: 0,
        msg: Ok(Tmessage::Version(Tversion {
            msize: 8192,
            version: "9P2000.L".to_string(),
        })),
    };

    let mut buf = Vec::new();
    test.encode_async(&mut buf).await.unwrap();
    let mut cursor = Cursor::new(buf);
    let decoded = Tframe::decode_async(&mut cursor).await.unwrap();
    assert_eq!(decoded.tag, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_async_wire_format_delayed() {
    let test = Tframe {
        tag: 0,
        msg: Ok(Tmessage::Version(Tversion {
            msize: 8192,
            version: "9P2000.L".to_string(),
        })),
    };

    let (upstream, downstream) = tokio::io::duplex(1024);
    let writer = BlockingIO::new(Duration::from_millis(1), upstream);
    let reader = BlockingIO::new(Duration::from_millis(1), downstream);

    test.encode_async(writer).await.unwrap();
    let decoded = Tframe::decode_async(reader).await.unwrap();
    assert_eq!(decoded.tag, 0);
}

#[test]
fn test_convert_wire_format() {
    let test = Tframe {
        tag: 0,
        msg: Ok(Tmessage::Version(Tversion {
            msize: 8192,
            version: "9P2000.L".to_string(),
        })),
    };

    let buf = test.to_bytes();
    let decoded = Tframe::from_bytes(&buf).unwrap();
    assert_eq!(decoded.tag, 0);
}
#[test]
fn test_option_none() {
    let value: Option<u32> = None;

    // Test byte size
    assert_eq!(value.byte_size(), 1);

    // Test encode
    let mut buf = Vec::new();
    value.encode(&mut buf).unwrap();
    assert_eq!(buf.len(), 1);
    assert_eq!(buf[0], 0);

    // Test decode
    let mut cursor = Cursor::new(buf);
    let decoded: Option<u32> = WireFormat::decode(&mut cursor).unwrap();
    assert_eq!(value, decoded);
}

#[test]
fn test_option_some() {
    let value = Some(42u32);

    // Test byte size
    assert_eq!(value.byte_size(), 5); // 1 byte tag + 4 bytes for u32

    // Test encode
    let mut buf = Vec::new();
    value.encode(&mut buf).unwrap();
    assert_eq!(buf.len(), 5);
    assert_eq!(buf[0], 1);

    // Test decode
    let mut cursor = Cursor::new(buf);
    let decoded: Option<u32> = WireFormat::decode(&mut cursor).unwrap();
    assert_eq!(value, decoded);
}

#[test]
fn test_option_invalid_tag() {
    let buf = vec![2u8]; // Invalid tag
    let mut cursor = Cursor::new(buf);
    let result: Result<Option<u32>, _> = WireFormat::decode(&mut cursor);
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!(e.kind(), io::ErrorKind::InvalidData);
    }
}

#[test]
fn test_nested_option() {
    let value = Some(Some(42u32));

    // Test byte size
    assert_eq!(value.byte_size(), 6); // outer tag + inner tag + u32

    // Test encode/decode
    let mut buf = Vec::new();
    value.encode(&mut buf).unwrap();
    let mut cursor = Cursor::new(buf);
    let decoded: Option<Option<u32>> = WireFormat::decode(&mut cursor).unwrap();
    assert_eq!(value, decoded);
}

#[test]
fn test_bool_encode() {
    let mut buf = Vec::new();
    true.encode(&mut buf).unwrap();
    assert_eq!(buf, vec![1]);

    buf.clear();
    false.encode(&mut buf).unwrap();
    assert_eq!(buf, vec![0]);
}

#[test]
fn test_bool_decode() {
    let mut cursor = Cursor::new(vec![1]);
    let decoded: bool = WireFormat::decode(&mut cursor).unwrap();
    assert!(decoded);

    cursor = Cursor::new(vec![0]);
    let decoded: bool = WireFormat::decode(&mut cursor).unwrap();
    assert!(!decoded);
}

#[test]
fn test_bool_invalid_decode() {
    let mut cursor = Cursor::new(vec![2]);
    let result: Result<bool, _> = WireFormat::decode(&mut cursor);
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!(e.kind(), io::ErrorKind::InvalidData);
    }
}

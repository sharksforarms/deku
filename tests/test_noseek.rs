use deku::no_std_io::{Cursor, Read, Seek, SeekFrom, Write};
use deku::noseek::NoSeek;

#[cfg(feature = "std")]
use deku::no_std_io::ErrorKind;

#[test]
#[allow(clippy::seek_from_current)]
fn test_read_position_and_accessors() {
    let input = [0x10, 0x20, 0x30];
    let mut stream = NoSeek::new(&input[..]);

    assert_eq!(stream.get_ref(), &&input[..]);

    let mut first = [0u8; 2];
    assert_eq!(stream.read(&mut first).unwrap(), 2);
    assert_eq!(first, [0x10, 0x20]);
    assert_eq!(stream.stream_position().unwrap(), 2);
    assert_eq!(stream.seek(SeekFrom::Start(2)).unwrap(), 2);
    assert_eq!(stream.seek(SeekFrom::Current(0)).unwrap(), 2);

    let mut last = [0u8; 1];
    stream.read_exact(&mut last).unwrap();
    assert_eq!(last, [0x30]);
    assert_eq!(stream.stream_position().unwrap(), 3);
}

#[test]
fn test_write_position_and_accessors() {
    let mut output = [0u8; 4];
    let mut stream = NoSeek::new(Cursor::new(&mut output[..]));

    stream.get_mut().get_mut()[3] = 0x40;
    assert_eq!(stream.write(&[0x10, 0x20]).unwrap(), 2);
    stream.write_all(&[0x30]).unwrap();
    stream.flush().unwrap();
    assert_eq!(stream.stream_position().unwrap(), 3);

    let inner = stream.into_inner().into_inner();
    assert_eq!(inner, &[0x10, 0x20, 0x30, 0x40]);
}

#[cfg(feature = "std")]
#[test]
fn test_std_read_forwarding_updates_position() {
    let input = [0x10, 0x20, 0x30];

    let mut stream = NoSeek::new(&input[..]);
    let mut first = [0u8; 1];
    let mut second = [0u8; 2];
    let mut bufs = [
        std::io::IoSliceMut::new(&mut first),
        std::io::IoSliceMut::new(&mut second),
    ];
    let read = stream.read_vectored(&mut bufs).unwrap();
    assert_eq!(read, 3);
    assert_eq!(first, [0x10]);
    assert_eq!(second, [0x20, 0x30]);
    assert_eq!(stream.stream_position().unwrap(), 3);

    let mut stream = NoSeek::new(&input[..]);
    let mut output = Vec::new();
    assert_eq!(stream.read_to_end(&mut output).unwrap(), 3);
    assert_eq!(output, input);
    assert_eq!(stream.stream_position().unwrap(), 3);

    let mut stream = NoSeek::new(&b"deku"[..]);
    let mut output = String::new();
    assert_eq!(stream.read_to_string(&mut output).unwrap(), 4);
    assert_eq!(output, "deku");
    assert_eq!(stream.stream_position().unwrap(), 4);
}

#[cfg(feature = "std")]
#[test]
fn test_std_write_vectored_updates_position() {
    let mut output = [0u8; 3];
    {
        let mut stream = NoSeek::new(&mut output[..]);
        let bufs = [
            std::io::IoSlice::new(&[0x10]),
            std::io::IoSlice::new(&[0x20, 0x30]),
        ];

        let written = stream.write_vectored(&bufs).unwrap();
        assert_eq!(written, 3);
        assert_eq!(stream.stream_position().unwrap(), 3);
    }
    assert_eq!(output, [0x10, 0x20, 0x30]);
}

#[cfg(feature = "std")]
#[test]
fn test_unsupported_seek_returns_error() {
    let mut stream = NoSeek::new(&[0x10][..]);
    let error = stream.seek(SeekFrom::Start(1)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Other);
}

#[cfg(not(feature = "std"))]
#[test]
#[should_panic(expected = "seek on unseekable file")]
fn test_unsupported_seek_panics_without_std() {
    let mut stream = NoSeek::new(&[0x10][..]);
    let _ = stream.seek(SeekFrom::Start(1));
}

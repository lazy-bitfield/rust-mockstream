use super::{FailingMockStream, MockStream, SharedMockStream, SyncMockStream};
use std::error::Error;
use std::io::{Cursor, ErrorKind, Read, Result, Write};

#[test]
fn test_mock_stream_read() {
    let mut s = MockStream::new();
    s.push_bytes_to_read("abcd".as_bytes());
    let mut v = [11; 6];
    assert_eq!(s.read(v.as_mut()).unwrap(), 4);
    assert_eq!(v, [97, 98, 99, 100, 11, 11]);
}

#[test]
fn test_mock_stream_pop_again() {
    let mut s = MockStream::new();
    s.write_all(b"abcd").unwrap();
    assert_eq!(s.pop_bytes_written(), b"abcd");
    s.write_all(b"efgh").unwrap();
    assert_eq!(s.pop_bytes_written(), b"efgh");
}

#[test]
fn test_mock_stream_empty_and_fill() {
    let mut s = MockStream::new();
    let mut v = [11; 6];
    assert_eq!(s.read(v.as_mut()).unwrap(), 0);
    s.push_bytes_to_read("abcd".as_bytes());
    assert_eq!(s.read(v.as_mut()).unwrap(), 4);
    assert_eq!(s.read(v.as_mut()).unwrap(), 0);
}

#[test]
fn test_mock_stream_read_lines() {
    let mut s = MockStream::new();
    s.push_bytes_to_read("abcd\r\ndcba\r\n".as_bytes());
    let first_line = s
        .bytes()
        .map(|c| c.unwrap())
        .take_while(|&c| c != b'\n')
        .collect::<Vec<u8>>();
    assert_eq!(first_line, (vec![97, 98, 99, 100, 13]));
}

#[test]
fn test_failing_mock_stream_read() {
    let mut s = FailingMockStream::new(ErrorKind::BrokenPipe, "The dog ate the ethernet cable", 1);
    let mut v = [0; 4];
    let error = s.read(v.as_mut()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::BrokenPipe);
    assert_eq!(error.description(), "The dog ate the ethernet cable");
    // after a single error, it will return Ok(0)
    assert_eq!(s.read(v.as_mut()).unwrap(), 0);
}

#[test]
fn test_failing_mock_stream_chain() {
    let mut s1 = MockStream::new();
    s1.push_bytes_to_read("abcd".as_bytes());
    let s2 = FailingMockStream::new(ErrorKind::Other, "Failing", -1);

    let mut c = s1.chain(s2);
    let mut v = [0; 8];
    assert_eq!(c.read(v.as_mut()).unwrap(), 4);
    assert_eq!(c.read(v.as_mut()).unwrap_err().kind(), ErrorKind::Other);
    assert_eq!(c.read(v.as_mut()).unwrap_err().kind(), ErrorKind::Other);
}

#[test]
fn test_failing_mock_stream_chain_interrupted() {
    let mut c = Cursor::new(&b"abcd"[..])
        .chain(FailingMockStream::new(
            ErrorKind::Interrupted,
            "Interrupted",
            5,
        ))
        .chain(Cursor::new(&b"ABCD"[..]));

    let mut v = [0; 8];
    c.read_exact(v.as_mut()).unwrap();
    assert_eq!(v, [0x61, 0x62, 0x63, 0x64, 0x41, 0x42, 0x43, 0x44]);
    assert_eq!(c.read(v.as_mut()).unwrap(), 0);
}

#[test]
fn test_mock_stream_write() {
    let mut s = MockStream::new();
    assert_eq!(s.write("abcd".as_bytes()).unwrap(), 4);
    assert_eq!(s.pop_bytes_written().as_ref(), [97, 98, 99, 100]);
    assert!(s.pop_bytes_written().is_empty());
}

#[test]
fn test_failing_mock_stream_write() {
    let mut s = FailingMockStream::new(ErrorKind::PermissionDenied, "Access denied", -1);
    let error = s.write("abcd".as_bytes()).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::PermissionDenied);
    assert_eq!(error.description(), "Access denied");
    // it will keep failing
    s.write("abcd".as_bytes()).unwrap_err();
}

// *** Real-world example ***

/// ADT that represents some network stream
enum NetStream {
    Mocked(SharedMockStream),
    //Tcp(TcpStream)
}

impl Read for NetStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match *self {
            NetStream::Mocked(ref mut s) => s.read(buf),
            //NetStream::Tcp(ref mut s) => s.read(buf),
        }
    }
}

impl Write for NetStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match *self {
            NetStream::Mocked(ref mut s) => s.write(buf),
            //NetStream::Tcp(ref mut s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> Result<()> {
        match *self {
            NetStream::Mocked(ref mut s) => s.flush(),
            //NetStream::Tcp(ref mut s) => s.flush(),
        }
    }
}

/// read 4 bytes from network, reverse them and write back
fn reverse4(s: &mut NetStream) -> Result<usize> {
    let mut v = [0; 4];
    let count = try![s.read(v.as_mut())];
    assert_eq!(count, 4);
    v.reverse();
    s.write(v.as_ref())
}

#[test]
fn test_shared_mock_stream() {
    let mut s = SharedMockStream::new();
    let mut e = NetStream::Mocked(s.clone());

    // provide data to mock
    s.push_bytes_to_read([1, 2, 3, 4].as_ref());
    // check if io succeeded
    assert_eq!(reverse4(&mut e).unwrap(), 4);
    // verify the data returned
    assert_eq!(s.pop_bytes_written().as_ref(), [4, 3, 2, 1]);

    // ensure no more bytes in stream
    assert_eq!(e.read(&mut [0; 4]).unwrap(), 0);
}

#[test]
fn test_sync_mock_stream() {
    use std::thread;
    let mut s = SyncMockStream::new();
    let mut s2 = s.clone();

    // thread will write some bytes, and then read some bytes
    s.push_bytes_to_read(&[5, 6, 7, 8]);
    let read = thread::spawn(move || {
        s2.write_all(&[1, 2, 3, 4]).unwrap();
        let mut buf = Vec::new();
        s2.read_to_end(&mut buf).unwrap();
        buf
    })
    .join()
    .unwrap();

    assert_eq!(s.pop_bytes_written(), &[1, 2, 3, 4]);
    assert_eq!(read, &[5, 6, 7, 8]);
}

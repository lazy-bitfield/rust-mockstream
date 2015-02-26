use super::{MockStream, SharedMockStream};
use std::io;

#[test]
fn test_mock_stream_read() {
	use std::io::Read;
	let mut s = MockStream::new();
	s.push_bytes_to_read("abcd".as_bytes());
	let mut v = [11; 6];
	assert_eq!(s.read(v.as_mut_slice()), Ok(4));
	assert_eq!(v, [97, 98, 99, 100, 11, 11]);
}

#[test]
fn test_mock_stream_write() {
	use std::io::Write;
	let mut s = MockStream::new();
	assert_eq!(s.write("abcd".as_bytes()), Ok(4));
	assert_eq!(s.pop_bytes_written().as_slice(), [97, 98, 99, 100]);
	assert!(s.pop_bytes_written().is_empty());
}

// *** Real-world example ***

/// ADT that represents some network stream
enum NetStream {
	Mocked(SharedMockStream),
	//Tcp(TcpStream)
}

impl io::Read for NetStream {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		match *self {
			NetStream::Mocked(ref mut s) => s.read(buf),
			//NetStream::Tcp(ref mut s) => s.read(buf),
		}
	}
}

impl io::Write for NetStream {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		match *self {
			NetStream::Mocked(ref mut s) => s.write(buf),
			//NetStream::Tcp(ref mut s) => s.write(buf),
		}
	}

	fn flush(&mut self) -> io::Result<()> {
		match *self {
			NetStream::Mocked(ref mut s) => s.flush(),
			//NetStream::Tcp(ref mut s) => s.flush(),
		}
	}
}

/// read 4 bytes from network, reverse them and write back
fn reverse4(s: &mut NetStream) -> io::Result<usize> {
	use std::io::{Read, Write};
	
	let mut v = [0; 4];
	let count = try![s.read(v.as_mut_slice())];
	assert_eq!(count, 4);
	v.reverse();
	s.write(v.as_slice())
}

#[test]
fn test_shared_mock_stream() {
	use std::io::Read;

	let mut s = SharedMockStream::new();
	let mut e = NetStream::Mocked(s.clone());

	let source = [1, 2, 3, 4];
	s.push_bytes_to_read(source.as_slice());

	// reverse4 succeeded
	assert_eq!(reverse4(&mut e), Ok(4));
	// ensure no more bytes in stream
	assert_eq!(e.read(&mut [0; 4]), Ok(0));
	// check data written by reverse4
	assert_eq!(s.pop_bytes_written().as_slice(), [4, 3, 2, 1]);
}

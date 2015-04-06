use super::{MockStream, SharedMockStream};
use std::io::{Read,Write,Result};

#[test]
fn test_mock_stream_read() {
	let mut s = MockStream::new();
	s.push_bytes_to_read("abcd".as_bytes());
	let mut v = [11; 6];
	assert_eq!(s.read(v.as_mut()).unwrap(), 4);
	assert_eq!(v, [97, 98, 99, 100, 11, 11]);
}

#[test]
fn test_mock_stream_read_lines() {
	let mut s = MockStream::new();
	s.push_bytes_to_read("abcd\r\ndcba\r\n".as_bytes());
	let first_line = s.bytes().map(|c| c.unwrap()).take_while(|&c| c != b'\n').collect::<Vec<u8>>();
	assert_eq!(first_line, (vec![97, 98, 99, 100, 13]));
	
}


#[test]
fn test_mock_stream_write() {
	let mut s = MockStream::new();
	assert_eq!(s.write("abcd".as_bytes()).unwrap(), 4);
	assert_eq!(s.pop_bytes_written().as_ref(), [97, 98, 99, 100]);
	assert!(s.pop_bytes_written().is_empty());
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

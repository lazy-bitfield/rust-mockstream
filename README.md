# rust-mockstream

Stream (Read+Write traits) implementations to be used to mock real streams in tests.

## Install

Just use Cargo.

## Usage scenario

Wrap the stream you use into ADT and provide Read and Write traits implementation.

```
enum NetStream {
	Mocked(SharedMockStream),
	Tcp(TcpStream)
}

impl io::Read for NetStream {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		match *self {
			NetStream::Mocked(ref mut s) => s.read(buf),
			NetStream::Tcp(ref mut s) => s.read(buf),
		}
	}
}

impl io::Write for NetStream {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		match *self {
			NetStream::Mocked(ref mut s) => s.write(buf),
			NetStream::Tcp(ref mut s) => s.write(buf),
		}
	}

	fn flush(&mut self) -> io::Result<()> {
		match *self {
			NetStream::Mocked(ref mut s) => s.flush(),
			NetStream::Tcp(ref mut s) => s.flush(),
		}
	}
}
```

Then use this ADT instead of stream everywhere.
```
fn reverse4(s: &mut NetStream) -> io::Result<usize> {
	use std::io::{Read, Write};
	
	// read 4 bytes into v
	let mut v = [0; 4];
	let count = try![s.read(v.as_mut_slice())];
	assert_eq!(count, 4);

	// reverse them
	v.reverse();

	// write them back to network
	s.write(v.as_slice())
}
```

In tests, provide data to mock and verify the results after performing operations.

```
	let mut s = SharedMockStream::new();
	let mut e = NetStream::Mocked(s.clone());

	// provide data to mock
	s.push_bytes_to_read([1, 2, 3, 4].as_slice());
	// check if io succeeded
	assert_eq!(reverse4(&mut e), Ok(4));
	// verify the data returned
	assert_eq!(s.pop_bytes_written().as_slice(), [4, 3, 2, 1]);
```

## I/O failures

A mock simulating I/O errors is provided too. Combined with a Chain, this allows
to simulate errors between reading data. See the following example where read_data
performs 3 retries on I/O errors. This is verified in the test function below.

```
struct CountIo {}

impl CountIo {
	fn read_data(&self, r: &mut Read) -> usize {
		let mut count: usize = 0;
		let mut retries = 3;

		loop {
			let mut buffer = [0; 5];
			match r.read(&mut buffer) {
				Err(_) => {
					if retries == 0 { break; }
					retries -= 1;
				},
				Ok(0) => break,
				Ok(n) => count += n,
			}
		}
		count
	}
}

#[test]
fn test_io_retries() {
	let mut c = Cursor::new(&b"1234"[..])
			.chain(FailingMockStream::new(ErrorKind::Other, "Failing", 3))
			.chain(Cursor::new(&b"5678"[..]));

	let sut = CountIo {};
	// this will fail unless read_data performs at least 3 retries on I/O errors
	assert_eq!(8, sut.read_data(&mut c));
}
```

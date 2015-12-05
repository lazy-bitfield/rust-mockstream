#![crate_name="mockstream"]
#![crate_type="lib"]
//! A reader/writer streams to mock real streams in tests.

use std::rc::{Rc};
use std::cell::{RefCell};
use std::io::{Cursor,Read,Write,Result};
use std::mem::swap;

#[cfg(test)]
mod tests;

/// MockStream is Read+Write stream that stores the data written and provides the data to be read.
#[derive(Clone)]
pub struct MockStream {
	reader: Cursor<Vec<u8>>,
	writer: Cursor<Vec<u8>>,
}

fn new_cursor() -> Cursor<Vec<u8>> {
	Cursor::new(Vec::new())
}

impl MockStream {
	/// Create new empty stream
	pub fn new() -> MockStream {
		MockStream { 
			reader: new_cursor(),
			writer: new_cursor(),
		}
	}

	/// Extract all bytes written by Write trait calls.
	pub fn pop_bytes_written(&mut self) -> Vec<u8> {
		let mut result = Vec::new();
		swap(&mut result, self.writer.get_mut());
		result
	}

	/// Provide data to be read by Read trait calls.
	pub fn push_bytes_to_read(&mut self, bytes: &[u8]) {
		let avail = self.reader.get_ref().len();
		if self.reader.position() == avail as u64 {
			self.reader = new_cursor();
		}
		self.reader.get_mut().extend(bytes.iter().map(|c| *c));
	}
}

impl Read for MockStream {
	fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
		self.reader.read(buf)	
	}
}

impl Write for MockStream {
	fn write<'a>(&mut self, buf: &'a [u8]) -> Result<usize> {
		self.writer.write(buf)
	}

	fn flush(&mut self) -> Result<()> {
		self.writer.flush()
	}
}


/// Reference-counted stream.
#[derive(Clone)]
pub struct SharedMockStream {
	pimpl: Rc<RefCell<MockStream>>
}

impl SharedMockStream {
	/// Create empty stream
	pub fn new() -> SharedMockStream {
		SharedMockStream { pimpl: Rc::new(RefCell::new(MockStream::new())) }
	}

	/// Extract all bytes written by Write trait calls.
	pub fn push_bytes_to_read(&mut self, bytes: &[u8]) {
		self.pimpl.borrow_mut().push_bytes_to_read(bytes)
	}

	/// Provide data to be read by Read trait calls.
	pub fn pop_bytes_written(&mut self) -> Vec<u8> {
		self.pimpl.borrow_mut().pop_bytes_written()
	}
}

impl Read for SharedMockStream {
	fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
		self.pimpl.borrow_mut().read(buf)
	}
}

impl Write for SharedMockStream {
	fn write(&mut self, buf: &[u8]) -> Result<usize> {
		self.pimpl.borrow_mut().write(buf)
	}

	fn flush(&mut self) -> Result<()> {
		self.pimpl.borrow_mut().flush()
	}
}

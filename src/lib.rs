#![crate_name="mockstream"]
#![crate_type="lib"]
#![feature(io, collections, core)]

//! A reader/writer streams to mock real streams in tests.

use std::rc::{Rc};
use std::cell::{RefCell};
use std::io;
use std::mem::swap;
use std::slice::bytes::copy_memory;
use std::collections::vec_deque::{RingBuf};

#[cfg(test)]
mod tests;

/// MockStream is Read+Write stream that stores the data written and provides the data to be read.
pub struct MockStream {
	bytes_to_read: RingBuf<Vec<u8>>,
	bytes_written: Vec<u8>,
}

impl MockStream {
	/// Create new empty stream
	pub fn new() -> MockStream {
		MockStream { 
			bytes_to_read: RingBuf::new(),
			bytes_written: Vec::new(),
		}
	}

	/// Extract all bytes written by Write trait calls.
	pub fn pop_bytes_written(&mut self) -> Vec<u8> {
		let mut temp = Vec::new();
		swap(&mut self.bytes_written, &mut temp);
		temp
	}

	/// Provide data to be read by Read trait calls.
	pub fn push_bytes_to_read(&mut self, bytes: &[u8]) {
		self.bytes_to_read.push_back(bytes.to_vec())
	}
}

impl io::Read for MockStream {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		if buf.len() == 0 {
			return Ok(0)
		}

		self.bytes_to_read.pop_front().map_or(Ok(0), |mut v| {
			if v.len() > buf.len() {
				self.bytes_to_read.push_front(v.split_off(buf.len()));
			}
			copy_memory(buf, v.as_slice());
			Ok(v.len())
		})
	}
}

impl io::Write for MockStream {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		self.bytes_written.push_all(buf);
		Ok(buf.len())
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
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

impl io::Read for SharedMockStream {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		self.pimpl.borrow_mut().read(buf)
	}
}

impl io::Write for SharedMockStream {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		self.pimpl.borrow_mut().write(buf)
	}

	fn flush(&mut self) -> io::Result<()> {
		self.pimpl.borrow_mut().flush()
	}
}

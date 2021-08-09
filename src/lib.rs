#![crate_name = "mockstream"]
#![crate_type = "lib"]
//! A reader/writer streams to mock real streams in tests.

use std::cell::RefCell;
use std::io::{Cursor, Error, ErrorKind, Read, Result, Write};
use std::mem::swap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time;

#[cfg(test)]
mod tests;

fn find_subsequence<T>(haystack: &[T], needle: &[T]) -> Option<usize>
where
    for<'a> &'a [T]: PartialEq,
{
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// MockStream is Read+Write stream that stores the data written and provides the data to be read.
#[derive(Clone)]
pub struct MockStream {
    reader: Cursor<Vec<u8>>,
    writer: Cursor<Vec<u8>>,
}

impl Default for MockStream {
    fn default() -> Self {
        MockStream::new()
    }
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
    pub fn peek_bytes_written(&mut self) -> &Vec<u8> {
        self.writer.get_ref()
    }

    /// Extract all bytes written by Write trait calls.
    pub fn pop_bytes_written(&mut self) -> Vec<u8> {
        let mut result = Vec::new();
        swap(&mut result, self.writer.get_mut());
        self.writer.set_position(0);
        result
    }

    /// Provide data to be read by Read trait calls.
    pub fn push_bytes_to_read(&mut self, bytes: &[u8]) {
        let avail = self.reader.get_ref().len();
        if self.reader.position() == avail as u64 {
            self.reader = new_cursor();
        }
        self.reader.get_mut().extend(bytes.iter().copied());
    }
}

impl Read for MockStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.reader.read(buf)
    }
}

impl Write for MockStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        self.writer.flush()
    }
}

/// Reference-counted stream.
#[derive(Clone, Default)]
pub struct SharedMockStream {
    pimpl: Rc<RefCell<MockStream>>,
}

impl SharedMockStream {
    /// Create empty stream
    pub fn new() -> SharedMockStream {
        SharedMockStream::default()
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

/// Thread-safe stream.
#[derive(Clone, Default)]
pub struct SyncMockStream {
    pimpl: Arc<Mutex<MockStream>>,
    pub waiting_for_write: Arc<AtomicBool>,
    pub expected_bytes: Vec<u8>,
}

impl SyncMockStream {
    /// Create empty stream
    pub fn new() -> SyncMockStream {
        SyncMockStream::default()
    }

    /// Block reads until expected bytes are written.
    pub fn wait_for(&mut self, expected_bytes: &[u8]) {
        self.expected_bytes = expected_bytes.to_vec();
        self.waiting_for_write.store(true, Ordering::Relaxed);
    }

    /// Extract all bytes written by Write trait calls.
    pub fn push_bytes_to_read(&mut self, bytes: &[u8]) {
        self.pimpl.lock().unwrap().push_bytes_to_read(bytes)
    }

    /// Provide data to be read by Read trait calls.
    pub fn pop_bytes_written(&mut self) -> Vec<u8> {
        self.pimpl.lock().unwrap().pop_bytes_written()
    }
}

impl Read for SyncMockStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        while self.waiting_for_write.load(Ordering::Relaxed) {
            sleep(time::Duration::from_millis(10));
        }
        self.pimpl.lock().unwrap().read(buf)
    }
}

impl Write for SyncMockStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut x = self.pimpl.lock().unwrap();
        match x.write(buf) {
            Ok(rv) => {
                if self.waiting_for_write.load(Ordering::Relaxed)
                    && find_subsequence(x.peek_bytes_written(), &self.expected_bytes).is_some()
                {
                    self.waiting_for_write.store(false, Ordering::Relaxed);
                }
                Ok(rv)
            }
            Err(rv) => Err(rv),
        }
    }

    fn flush(&mut self) -> Result<()> {
        self.pimpl.lock().unwrap().flush()
    }
}

/// `FailingMockStream` mocks a stream which will fail upon read or write
///
/// # Examples
///
/// ```
/// use std::io::{Cursor, Read};
///
/// struct CountIo {}
///
/// impl CountIo {
///     fn read_data(&self, r: &mut Read) -> usize {
///         let mut count: usize = 0;
///         let mut retries = 3;
///
///         loop {
///             let mut buffer = [0; 5];
///             match r.read(&mut buffer) {
///                 Err(_) => {
///                     if retries == 0 { break; }
///                     retries -= 1;
///                 },
///                 Ok(0) => break,
///                 Ok(n) => count += n,
///             }
///         }
///         count
///     }
/// }
///
/// #[test]
/// fn test_io_retries() {
///     let mut c = Cursor::new(&b"1234"[..])
///             .chain(FailingMockStream::new(ErrorKind::Other, "Failing", 3))
///             .chain(Cursor::new(&b"5678"[..]));
///
///     let sut = CountIo {};
///     // this will fail unless read_data performs at least 3 retries on I/O errors
///     assert_eq!(8, sut.read_data(&mut c));
/// }
/// ```
#[derive(Clone)]
pub struct FailingMockStream {
    kind: ErrorKind,
    message: &'static str,
    repeat_count: i32,
}

impl FailingMockStream {
    /// Creates a FailingMockStream
    ///
    /// When `read` or `write` is called, it will return an error `repeat_count` times.
    /// `kind` and `message` can be specified to define the exact error.
    pub fn new(kind: ErrorKind, message: &'static str, repeat_count: i32) -> FailingMockStream {
        FailingMockStream {
            kind,
            message,
            repeat_count,
        }
    }

    fn error(&mut self) -> Result<usize> {
        if self.repeat_count == 0 {
            Ok(0)
        } else {
            if self.repeat_count > 0 {
                self.repeat_count -= 1;
            }
            Err(Error::new(self.kind, self.message))
        }
    }
}

impl Read for FailingMockStream {
    fn read(&mut self, _: &mut [u8]) -> Result<usize> {
        self.error()
    }
}

impl Write for FailingMockStream {
    fn write(&mut self, _: &[u8]) -> Result<usize> {
        self.error()
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

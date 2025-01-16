// Copyright (c) 2020 rust-mysql-simple contributors
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

use std::{
    fmt, io,
    sync::{Arc, Mutex},
};

use crate::Conn;

pub(crate) type LocalInfileInner =
    Arc<Mutex<dyn for<'a> FnMut(&'a [u8], &'a mut LocalInfile<'_>) -> io::Result<()> + Send>>;

/// Callback to handle requests for local files.
/// Consult [Mysql documentation](https://dev.mysql.com/doc/refman/5.7/en/load-data.html) for the
/// format of local infile data.
///
/// # Support
///
/// Note that older versions of Mysql server may not support this functionality.
///
/// ```rust
/// # mysql::doctest_wrapper!(__result, {
/// use mysql::*;
/// use mysql::prelude::*;
///
/// use std::io::Write;
///
/// let pool = Pool::new(get_opts())?;
/// let mut conn = pool.get_conn().unwrap();
///
/// conn.query_drop("CREATE TEMPORARY TABLE mysql.tbl(a TEXT)").unwrap();
/// conn.set_local_infile_handler(Some(
///     LocalInfileHandler::new(|file_name, writer| {
///         writer.write_all(b"row1: file name is ")?;
///         writer.write_all(file_name)?;
///         writer.write_all(b"\n")?;
///
///         writer.write_all(b"row2: foobar\n")
///     })
/// ));
///
/// match conn.query_drop("LOAD DATA LOCAL INFILE 'file_name' INTO TABLE mysql.tbl") {
///     Ok(_) => (),
///     Err(Error::MySqlError(ref e)) if e.code == 1148 => {
///         // functionality is not supported by the server
///         return Ok(());
///     }
///     err => {
///         err.unwrap();
///     }
/// }
///
/// let mut row_num = 0;
/// let result: Vec<String> = conn.query("SELECT * FROM mysql.tbl").unwrap();
/// assert_eq!(
///     result,
///     vec!["row1: file name is file_name".to_string(), "row2: foobar".to_string()],
/// );
/// # });
/// ```
#[derive(Clone)]
pub struct LocalInfileHandler(pub(crate) LocalInfileInner);

impl LocalInfileHandler {
    pub fn new<F>(f: F) -> Self
    where
        F: for<'a> FnMut(&'a [u8], &'a mut LocalInfile<'_>) -> io::Result<()> + Send + 'static,
    {
        LocalInfileHandler(Arc::new(Mutex::new(f)))
    }
}

impl PartialEq for LocalInfileHandler {
    fn eq(&self, other: &LocalInfileHandler) -> bool {
        std::ptr::eq(&*self.0, &*other.0)
    }
}

impl Eq for LocalInfileHandler {}

impl fmt::Debug for LocalInfileHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "LocalInfileHandler(...)")
    }
}

/// Local in-file stream.
/// The callback will be passed a reference to this stream, which it
/// should use to write the contents of the requested file.
/// See [LocalInfileHandler](struct.LocalInfileHandler.html) documentation for example.
#[derive(Debug)]
pub struct LocalInfile<'a> {
    buffer: io::Cursor<&'a mut [u8]>,
    conn: &'a mut Conn,
}

impl<'a> LocalInfile<'a> {
    pub(crate) const BUFFER_SIZE: usize = 4096;

    pub(crate) fn new(buffer: &'a mut [u8; LocalInfile::BUFFER_SIZE], conn: &'a mut Conn) -> Self {
        Self {
            buffer: io::Cursor::new(buffer),
            conn,
        }
    }
}

impl io::Write for LocalInfile<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.buffer.position() == Self::BUFFER_SIZE as u64 {
            self.flush()?;
        }
        self.buffer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let n = self.buffer.position() as usize;
        if n > 0 {
            let mut range = &self.buffer.get_ref()[..n];
            self.conn
                .write_packet(&mut range)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, Box::new(e)))?;
        }
        self.buffer.set_position(0);
        Ok(())
    }
}

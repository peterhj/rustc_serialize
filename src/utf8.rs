//! Utilities for safely working with UTF-8 byte streams.

use std::io::{Read};
use std::str::{from_utf8};

// from the implementation in core::num (why is it private?)
#[inline]
pub const fn is_utf8_char_boundary(this: u8) -> bool {
    // This is bit magic equivalent to: b < 128 || b >= 192
    (this as i8) >= -0x40
}

pub struct CharBuffer<R> {
    inner: R,
    off: usize,
    buf: [u8; 4],
    bsz: u8,
    eof: bool,
    err: bool,
}

impl<R: Read> CharBuffer<R> {
    pub fn from_reader(inner: R) -> CharBuffer<R> {
        CharBuffer::from_reader_state(0, [0; 4], 0, inner)
    }

    pub fn from_reader_state(offset: usize, buf: [u8; 4], buf_len: usize, inner: R) -> CharBuffer<R> {
        assert!(buf_len <= 4);
        CharBuffer{
            inner,
            off: offset,
            buf,
            bsz: buf_len as u8,
            eof: false,
            err: false,
        }
    }

    pub fn into_inner(self) -> Result<(usize, [u8; 4], usize, R), (usize, [u8; 4], usize, R)> {
        self.into_reader_state()
    }

    pub fn into_reader_state(self) -> Result<(usize, [u8; 4], usize, R), (usize, [u8; 4], usize, R)> {
        let state = (self.off, self.buf, self.bsz as usize, self.inner);
        if self.err {
            Err(state)
        } else {
            Ok(state)
        }
    }
}

impl<R: Read> Iterator for CharBuffer<R> {
    type Item = Result<(char, usize), usize>;

    fn next(&mut self) -> Option<Result<(char, usize), usize>> {
        if self.err {
            return Some(Err(self.off));
        }
        if self.eof && self.bsz == 0 {
            return None;
        }
        if !self.eof && self.bsz < 4 {
            let olen = self.bsz as usize;
            for i in olen .. 4 {
                match self.inner.read(&mut self.buf[i .. (i + 1)]) {
                    Err(_) => {
                        self.err = true;
                        return Some(Err(self.off));
                    }
                    Ok(0) => {
                        self.eof = true;
                        break;
                    }
                    Ok(1) => {
                        self.bsz += 1;
                    }
                    Ok(_) => unreachable!()
                }
            }
        }
        assert!(self.bsz <= 4);
        let len = self.bsz as usize;
        if len == 0 {
            assert!(self.eof);
            return None;
        }
        if !is_utf8_char_boundary(self.buf[0]) {
            self.err = true;
            return Some(Err(self.off));
        }
        let mut i = 1;
        while i < len {
            if is_utf8_char_boundary(self.buf[i]) {
                break;
            }
            i += 1;
        }
        match from_utf8(&self.buf[ .. i]) {
            Err(_) => {
                self.err = true;
                return Some(Err(self.off));
            }
            Ok(s) => {
                let c = s.chars().next().unwrap();
                assert_eq!(c.len_utf8(), i);
                drop(s);
                let off = self.off;
                self.off += i;
                for j in i .. 4 {
                    self.buf[j - i] = self.buf[j];
                }
                self.bsz -= i as u8;
                return Some(Ok((c, off)));
            }
        }
    }
}

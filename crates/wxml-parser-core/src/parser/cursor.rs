use super::Parser;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Cursor {
  pub(crate) idx: usize,
  pub(crate) line: usize,
  pub(crate) col: usize,
}

impl<'a> Parser<'a> {
  #[inline(always)]
  pub(crate) fn eof(&self) -> bool {
    self.i >= self.bytes.len()
  }

  #[inline(always)]
  pub(crate) fn cur(&self) -> Option<u8> {
    unsafe { Some(*self.bytes.get_unchecked(self.i)) }
  }

  #[inline(always)]
  pub(crate) fn peek(&self, offset: usize) -> Option<u8> {
    self.bytes.get(self.i + offset).copied()
  }

  #[inline(always)]
  pub(crate) fn starts_with(&self, s: &[u8]) -> bool {
    let end = self.i + s.len();
    if end > self.bytes.len() {
      return false;
    }
    // SAFETY: we just checked bounds
    unsafe { self.bytes.get_unchecked(self.i..end) == s }
  }

  #[inline(always)]
  pub(crate) fn safe_slice(&self, start: usize, end: usize) -> &'a str {
    if start >= end || end > self.src.len() {
      ""
    } else {
      // SAFETY: parser always advances by valid UTF-8 boundaries
      unsafe { self.src.get_unchecked(start..end) }
    }
  }

  #[inline(always)]
  pub(crate) fn pos(&self) -> Cursor {
    Cursor {
      idx: self.i,
      line: self.line,
      col: self.col,
    }
  }

  /// Advance by exactly one byte. Caller must ensure the byte is ASCII
  /// or that multi-byte handling is not needed (e.g. for known ASCII sequences).
  #[inline(always)]
  fn bump_byte(&mut self, ch: u8) {
    if ch == b'\n' {
      self.line += 1;
      self.col = 1;
    } else {
      self.col += 1;
    }
    self.i += 1;
  }

  /// Advance past one character, handling multi-byte UTF-8.
  #[inline(always)]
  pub(crate) fn bump(&mut self) -> Option<u8> {
    if self.i >= self.bytes.len() {
      return None;
    }
    let ch = unsafe { *self.bytes.get_unchecked(self.i) };
    if ch == b'\n' {
      self.line += 1;
      self.col = 1;
      self.i += 1;
      return Some(ch);
    }
    if ch.is_ascii() {
      self.col += 1;
      self.i += 1;
      return Some(ch);
    }
    // Multi-byte UTF-8: compute width from leading byte
    let width = if ch & 0xF0 == 0xE0 {
      3
    } else if ch & 0xE0 == 0xC0 {
      2
    } else if ch & 0xF8 == 0xF0 {
      4
    } else {
      1
    };
    self.col += 1;
    self.i += width;
    Some(ch)
  }

  /// Bump past a known ASCII sequence (e.g. "<!--", "-->", "{{", "}}").
  #[inline(always)]
  pub(crate) fn bump_ascii(&mut self, s: &[u8]) {
    debug_assert!(s.is_ascii());
    for &ch in s {
      if ch == b'\n' {
        self.line += 1;
        self.col = 1;
      } else {
        self.col += 1;
      }
    }
    self.i += s.len();
  }

  pub(crate) fn bump_n(&mut self, n: usize) {
    for _ in 0..n {
      let _ = self.bump();
    }
  }

  pub(crate) fn skip_ws(&mut self) {
    while self.i < self.bytes.len() {
      match unsafe { *self.bytes.get_unchecked(self.i) } {
        b' ' | b'\t' | b'\r' => {
          self.col += 1;
          self.i += 1;
        }
        b'\n' => {
          self.line += 1;
          self.col = 1;
          self.i += 1;
        }
        _ => break,
      }
    }
  }

  /// Skip while the predicate holds on ASCII bytes. Stops on non-ASCII or EOF.
  #[inline(always)]
  pub(crate) fn skip_ascii_while(&mut self, pred: impl Fn(u8) -> bool) {
    while self.i < self.bytes.len() {
      let ch = unsafe { *self.bytes.get_unchecked(self.i) };
      if !ch.is_ascii() || !pred(ch) {
        break;
      }
      self.bump_byte(ch);
    }
  }
}

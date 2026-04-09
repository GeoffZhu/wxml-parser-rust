pub(crate) fn has_unescaped_quote_ahead(src: &[u8], from: usize, quote: u8) -> bool {
  let mut i = from;
  let mut escaped = false;
  while i < src.len() {
    let ch = src[i];
    if escaped {
      escaped = false;
      i += 1;
      continue;
    }
    if ch == b'\\' {
      escaped = true;
      i += 1;
      continue;
    }
    if ch == quote {
      return true;
    }
    i += 1;
  }
  false
}

pub(crate) fn scan_interpolation_end(src: &str, from: usize, stop_on_close_tag: bool) -> (usize, bool) {
  let b = src.as_bytes();
  let mut i = from;
  let mut nested = 0usize;
  let mut quote_ctx: Option<u8> = None;
  let mut escaped = false;

  while i + 1 < b.len() {
    if let Some(q) = quote_ctx {
      if stop_on_close_tag && nested == 0 && b[i] == b'<' && b[i + 1] == b'/' {
        if !has_unescaped_quote_ahead(b, i, q) {
          return (i, true);
        }
      }
      let ch = b[i];
      if escaped {
        escaped = false;
        i += 1;
        continue;
      }
      if ch == b'\\' {
        escaped = true;
        i += 1;
        continue;
      }
      if ch == q {
        quote_ctx = None;
      }
      i += 1;
      continue;
    }

    if b[i] == b'\'' || b[i] == b'"' {
      quote_ctx = Some(b[i]);
      i += 1;
      continue;
    }

    if b[i] == b'{' && b[i + 1] == b'{' {
      nested += 1;
      i += 2;
      continue;
    }

    if b[i] == b'}' && b[i + 1] == b'}' {
      if nested == 0 {
        return (i, false);
      }
      nested -= 1;
      i += 2;
      continue;
    }

    if stop_on_close_tag && nested == 0 && b[i] == b'<' && b[i + 1] == b'/' {
      return (i, quote_ctx.is_some());
    }

    i += 1;
  }

  (b.len(), quote_ctx.is_some())
}

#[allow(dead_code)]
pub(crate) fn scan_interpolation_end_loose(src: &str, from: usize, stop_on_close_tag: bool) -> usize {
  let b = src.as_bytes();
  let mut i = from;
  let mut nested = 0usize;

  while i + 1 < b.len() {
    if b[i] == b'{' && b[i + 1] == b'{' {
      nested += 1;
      i += 2;
      continue;
    }

    if b[i] == b'}' && b[i + 1] == b'}' {
      if nested == 0 {
        return i;
      }
      nested -= 1;
      i += 2;
      continue;
    }

    if stop_on_close_tag && nested == 0 && b[i] == b'<' && b[i + 1] == b'/' {
      return i;
    }

    i += 1;
  }

  b.len()
}

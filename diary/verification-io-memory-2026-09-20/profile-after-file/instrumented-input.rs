//! Incremental record reader shared by the main input and redirected getline.
use std::io::{self, Read};

pub(crate) type SharedReader = std::rc::Rc<std::cell::RefCell<RecordReader>>;

pub(crate) struct RecordReader {
    reader: Box<dyn Read>,
    buffer: Vec<u8>,
    start: usize,
    eof: bool,
    separator: Option<(Vec<u8>, std::rc::Rc<crate::ere::Ere>)>,
    stream_separator: Option<(Vec<u8>, std::rc::Rc<crate::ere::stream::StreamEre>)>,
}

impl RecordReader {
    pub(crate) fn shared(self) -> SharedReader {
        std::rc::Rc::new(std::cell::RefCell::new(self))
    }
    pub(crate) fn next_csv(&mut self) -> io::Result<Option<(Vec<u8>, Vec<u8>)>> {
        // These offsets are relative to remaining(), so fill's compaction
        // preserves them. Quote state must also survive physical short reads.
        let mut scanned = 0;
        let mut quoted = false;
        loop {
            let mut end = None;
            for (i, b) in self.remaining().iter().enumerate().skip(scanned) {
                crate::io_profile::SCANNED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if *b == b'"' {
                    quoted = !quoted;
                }
                if *b == b'\n' && !quoted {
                    end = Some(i);
                    break;
                }
            }
            if end.is_some() || self.eof {
                if self.remaining().is_empty() {
                    return Ok(None);
                }
                let len = end.unwrap_or(self.remaining().len());
                let mut record = Vec::with_capacity(len);
                for &b in &self.remaining()[..len] {
                    if b == b'\n' && record.last() == Some(&b'\r') {
                        record.pop();
                    }
                    record.push(b);
                }
                let rt = if end.is_some() {
                    if record.last() == Some(&b'\r') {
                        record.pop();
                        b"\r\n".to_vec()
                    } else {
                        b"\n".to_vec()
                    }
                } else {
                    Vec::new()
                };
                self.start += len + usize::from(end.is_some());
                return Ok(Some((record, rt)));
            }
            scanned = self.remaining().len();
            self.fill()?;
        }
    }
    pub(crate) fn new(reader: impl Read + 'static) -> Self {
        Self {
            reader: Box::new(reader),
            buffer: Vec::new(),
            start: 0,
            eof: false,
            separator: None,
            stream_separator: None,
        }
    }

    fn remaining(&self) -> &[u8] {
        &self.buffer[self.start..]
    }

    fn fill(&mut self) -> io::Result<()> {
        // Compact only when more input is needed, not after every record.
        if self.start > 1 {
            // Retain one consumed byte so ^ cannot match again after compaction.
            crate::io_profile::COMPACTED.fetch_add(self.buffer.len() - (self.start - 1), std::sync::atomic::Ordering::Relaxed);
            self.buffer.drain(..self.start - 1);
            self.start = 1;
        }
        let mut bytes = [0; 8192];
        crate::io_profile::READS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let count = self.reader.read(&mut bytes)?;
        crate::io_profile::READ_BYTES.fetch_add(count, std::sync::atomic::Ordering::Relaxed);
        self.eof = count == 0;
        crate::io_profile::COPIED.fetch_add(count, std::sync::atomic::Ordering::Relaxed);
        self.buffer.extend_from_slice(&bytes[..count]);
        crate::io_profile::CAPACITY.fetch_max(self.buffer.capacity(), std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    pub(crate) fn next(&mut self, rs: &[u8]) -> io::Result<Option<(Vec<u8>, Vec<u8>)>> {
        if rs.len() > 1 && crate::text::shift_jis() {
            return self.next_shift_jis(rs);
        }
        let regex = if rs.len() != 1 {
            if self.separator.as_ref().is_none_or(|(key, _)| key != rs) {
                let re = crate::ere::Ere::new(if rs.is_empty() { b"\n\n+" } else { rs })
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
                self.separator = Some((rs.to_vec(), std::rc::Rc::new(re)));
            }
            self.separator.as_ref().map(|(_, re)| re.clone())
        } else {
            None
        };
        // Only single-byte separators can skip a prefix conclusively searched
        // before fill. Regex separators may extend or need earlier context.
        let mut scanned = 0;
        loop {
            if rs.is_empty() {
                let leading = self.remaining().iter().take_while(|b| **b == b'\n').count();
                self.start += leading;
            }
            let subject_end = if regex.is_some() && crate::text::multibyte() && !self.eof {
                crate::text::incomplete_tail(&self.buffer).unwrap_or(self.buffer.len())
            } else {
                self.buffer.len()
            };
            let separator = if let Some(regex) = &regex {
                {
                    let subject = regex.subject(&self.buffer[..subject_end]);
                    let mut offset = self.start;
                    let mut found = None;
                    while offset <= subject_end {
                        let Some(m) = subject.find_at(offset) else {
                            break;
                        };
                        if !m.is_empty() {
                            found =
                                Some((m.start() - self.start, m.end() - self.start, m.can_extend));
                            break;
                        }
                        offset = crate::text::advance(&self.buffer, m.end());
                    }
                    found
                }
            } else {
                crate::io_profile::position(&self.remaining()[scanned..], rs[0])
                    .map(|p| (scanned + p, scanned + p + 1, false))
            };
            if let Some((start, end, can_extend)) = separator {
                // A regex separator ending at the buffer boundary might extend.
                if regex.is_some()
                    && (can_extend || end == subject_end.saturating_sub(self.start))
                    && !self.eof
                {
                    self.fill()?;
                    continue;
                }
                let record = crate::io_profile::copy(&self.remaining()[..start]);
                let rt = crate::io_profile::copy(&self.remaining()[start..end]);
                self.start += end;
                return Ok(Some((record, rt)));
            }
            if self.eof {
                if self.remaining().is_empty() {
                    return Ok(None);
                }
                let mut record = crate::io_profile::copy(&self.remaining());
                self.buffer.clear();
                self.start = 0;
                if rs.is_empty() {
                    while record.last() == Some(&b'\n') {
                        record.pop();
                    }
                }
                return Ok(Some((record, Vec::new())));
            }
            if regex.is_none() {
                scanned = self.remaining().len();
            }
            self.fill()?;
        }
    }

    fn next_shift_jis(&mut self, rs: &[u8]) -> io::Result<Option<(Vec<u8>, Vec<u8>)>> {
        use crate::ere::stream::{Progress, StreamEre};
        if self
            .stream_separator
            .as_ref()
            .is_none_or(|(key, _)| key != rs)
        {
            let re =
                StreamEre::new(rs).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
            self.stream_separator = Some((rs.to_vec(), std::rc::Rc::new(re)));
        }
        let re = self.stream_separator.as_ref().unwrap().1.clone();
        let mut search = re.search(self.start == 0);
        loop {
            match search.resume(self.remaining(), self.eof) {
                Progress::NeedMore => self.fill()?,
                Progress::Found { start, end } => {
                    let record = crate::io_profile::copy(&self.remaining()[..start]);
                    let rt = crate::io_profile::copy(&self.remaining()[start..end]);
                    self.start += end;
                    return Ok(Some((record, rt)));
                }
                Progress::Done => {
                    if self.remaining().is_empty() {
                        return Ok(None);
                    }
                    let record = crate::io_profile::copy(&self.remaining());
                    self.start = self.buffer.len();
                    return Ok(Some((record, Vec::new())));
                }
            }
        }
    }
}

pub(crate) fn csv_fields(record: &[u8]) -> Vec<Vec<u8>> {
    if record.is_empty() {
        return Vec::new();
    }
    let mut fields = Vec::new();
    let mut i = 0;
    loop {
        let mut field = Vec::new();
        if record.get(i) == Some(&b'"') {
            i += 1;
            while i < record.len() {
                if record[i] == b'"' && record.get(i + 1) == Some(&b'"') {
                    field.push(b'"');
                    i += 2;
                } else if record[i] == b'"' && matches!(record.get(i + 1), None | Some(b',')) {
                    i += 1;
                    break;
                } else {
                    field.push(record[i]);
                    i += 1;
                }
            }
        } else {
            while i < record.len() && record[i] != b',' {
                field.push(record[i]);
                i += 1;
            }
        }
        fields.push(field);
        if i >= record.len() {
            break;
        }
        i += 1;
    }
    fields
}

pub(crate) struct MainInput {
    pub(crate) reader: Option<SharedReader>,
    pub(crate) next_arg: usize,
    pub(crate) opened: bool,
}
impl Default for MainInput {
    fn default() -> Self {
        Self {
            reader: None,
            next_arg: 1,
            opened: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Chunks {
        data: std::io::Cursor<Vec<u8>>,
        size: usize,
    }
    impl Read for Chunks {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let n = out.len().min(self.size);
            self.data.read(&mut out[..n])
        }
    }
    #[test]
    fn long_records_compaction_and_separator_changes_share_one_cursor() {
        for size in [1, 7, 8192] {
            let long = vec![b'x'; 32769];
            let mut data = b"head\n".to_vec();
            data.extend_from_slice(&long);
            data.extend_from_slice(b":\0\xff!tail");
            let shared = RecordReader::new(Chunks {
                data: std::io::Cursor::new(data),
                size,
            })
            .shared();
            let alias = shared.clone();
            assert_eq!(shared.borrow_mut().next(b"\n").unwrap().unwrap().0, b"head");
            assert_eq!(alias.borrow_mut().next(b":").unwrap().unwrap(), (long, b":".to_vec()));
            assert_eq!(shared.borrow_mut().next(b"!").unwrap().unwrap().0, b"\0\xff");
            assert_eq!(alias.borrow_mut().next(b"\n").unwrap().unwrap().0, b"tail");
            assert!(shared.borrow_mut().next(b"\n").unwrap().is_none());
        }
    }

    #[test]
    fn long_csv_keeps_quotes_crlf_and_eof_across_short_reads() {
        for size in [1, 2, 7, 8192] {
            let mut record = b"\"".to_vec();
            record.extend(std::iter::repeat_n(b'x', 32767));
            record.extend_from_slice(b"\"\"y\r\nz\",last");
            let mut data = b"head\r\n".to_vec();
            data.extend_from_slice(&record);
            data.extend_from_slice(b"\r\n\"unterminated\nquote");
            let mut reader = RecordReader::new(Chunks {
                data: std::io::Cursor::new(data),
                size,
            });
            assert_eq!(reader.next_csv().unwrap().unwrap(), (b"head".to_vec(), b"\r\n".to_vec()));
            let normalized = record.into_iter().filter(|b| *b != b'\r').collect::<Vec<_>>();
            assert_eq!(reader.next_csv().unwrap().unwrap(), (normalized, b"\r\n".to_vec()));
            assert_eq!(reader.next_csv().unwrap().unwrap(), (b"\"unterminated\nquote".to_vec(), Vec::new()));
            assert!(reader.next_csv().unwrap().is_none());
        }
    }

    #[test]
    fn separators_are_independent_of_read_boundaries() {
        for size in [1, 2, 3, 7, 8192] {
            for (input, rs, expected) in [
                (
                    b"aaa1a2a\n".as_slice(),
                    b"^a".as_slice(),
                    vec![Vec::new(), b"aa1a2a\n".to_vec()],
                ),
                (
                    b"aaab".as_slice(),
                    b"^a+".as_slice(),
                    vec![Vec::new(), b"b".to_vec()],
                ),
                (
                    b"xabzab".as_slice(),
                    b"a|ab".as_slice(),
                    vec![b"x".to_vec(), b"z".to_vec()],
                ),
                (
                    b"xazabz".as_slice(),
                    b"a.*b|a".as_slice(),
                    vec![b"x".to_vec(), b"z".to_vec()],
                ),
                (
                    b"\n\na\n\n\nb\n".as_slice(),
                    b"".as_slice(),
                    vec![b"a".to_vec(), b"b".to_vec()],
                ),
            ] {
                let mut reader = RecordReader::new(Chunks {
                    data: std::io::Cursor::new(input.to_vec()),
                    size,
                });
                let mut actual = Vec::new();
                while let Some((record, _)) = reader.next(rs).unwrap() {
                    actual.push(record);
                }
                assert_eq!(actual, expected, "chunk size {size}");
            }
            let mut reader = RecordReader::new(Chunks {
                data: std::io::Cursor::new(b"a,\"b\r\nc\"\r\nz,\"x\"\"y\"\n".to_vec()),
                size,
            });
            assert_eq!(reader.next_csv().unwrap().unwrap().0, b"a,\"b\nc\"");
            assert_eq!(reader.next_csv().unwrap().unwrap().0, b"z,\"x\"\"y\"");
            assert!(reader.next_csv().unwrap().is_none());
        }
    }
}

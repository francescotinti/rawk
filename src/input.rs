//! Incremental record reader shared by the main input and redirected getline.
use std::io::{self, Read};

pub(crate) struct RecordReader {
    reader: Box<dyn Read>,
    buffer: Vec<u8>,
    eof: bool,
    separator: Option<(Vec<u8>, std::rc::Rc<crate::ere::Ere>)>,
}

impl RecordReader {
    pub(crate) fn next_csv(&mut self) -> io::Result<Option<(Vec<u8>, Vec<u8>)>> {
        loop {
            let mut quoted = false;
            let mut end = None;
            for (i, b) in self.buffer.iter().enumerate() {
                if *b == b'"' {
                    quoted = !quoted;
                }
                if *b == b'\n' && !quoted {
                    end = Some(i);
                    break;
                }
            }
            if end.is_some() || self.eof {
                if self.buffer.is_empty() {
                    return Ok(None);
                }
                let len = end.unwrap_or(self.buffer.len());
                let mut record = Vec::new();
                for &b in &self.buffer[..len] {
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
                self.buffer.drain(..len + usize::from(end.is_some()));
                return Ok(Some((record, rt)));
            }
            self.fill()?;
        }
    }
    pub(crate) fn new(reader: impl Read + 'static) -> Self {
        Self {
            reader: Box::new(reader),
            buffer: Vec::new(),
            eof: false,
            separator: None,
        }
    }

    fn fill(&mut self) -> io::Result<()> {
        let mut bytes = [0; 8192];
        let count = self.reader.read(&mut bytes)?;
        self.eof = count == 0;
        self.buffer.extend_from_slice(&bytes[..count]);
        Ok(())
    }

    pub(crate) fn next(&mut self, rs: &[u8]) -> io::Result<Option<(Vec<u8>, Vec<u8>)>> {
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
        loop {
            if rs.is_empty() {
                let leading = self.buffer.iter().take_while(|b| **b == b'\n').count();
                self.buffer.drain(..leading);
            }
            let separator = if let Some(regex) = &regex {
                {
                    let mut offset = 0;
                    let mut found = None;
                    while offset <= self.buffer.len() {
                        let Some(m) = regex.find_at(&self.buffer, offset) else {
                            break;
                        };
                        if !m.is_empty() {
                            found = Some((m.start(), m.end(), m.can_extend));
                            break;
                        }
                        offset = m.end() + 1;
                    }
                    found
                }
            } else {
                self.buffer
                    .iter()
                    .position(|b| *b == rs[0])
                    .map(|p| (p, p + 1, false))
            };
            if let Some((start, end, can_extend)) = separator {
                // A regex separator ending at the buffer boundary might extend.
                if regex.is_some() && (can_extend || end == self.buffer.len()) && !self.eof {
                    self.fill()?;
                    continue;
                }
                let record = self.buffer[..start].to_vec();
                let rt = self.buffer[start..end].to_vec();
                self.buffer.drain(..end);
                return Ok(Some((record, rt)));
            }
            if self.eof {
                if self.buffer.is_empty() {
                    return Ok(None);
                }
                let mut record = std::mem::take(&mut self.buffer);
                if rs.is_empty() {
                    while record.last() == Some(&b'\n') {
                        record.pop();
                    }
                }
                return Ok(Some((record, Vec::new())));
            }
            self.fill()?;
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
    pub(crate) reader: Option<RecordReader>,
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
    fn separators_are_independent_of_read_boundaries() {
        for size in [1, 2, 3, 7, 8192] {
            for (input, rs, expected) in [
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

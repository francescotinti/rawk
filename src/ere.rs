//! Byte-oriented leftmost-longest matching. The locator finds the earliest
//! starting position; an anchored all-matches DFA selects its longest match.
//! Walking the DFA also tells streaming readers whether a match can extend.
use regex_automata::{
    Anchored, Input, MatchKind,
    dfa::{Automaton, dense},
    util::syntax,
};

#[derive(Clone)]
pub(crate) struct Ere {
    locator: regex::bytes::Regex,
    longest: Option<dense::DFA<Vec<u32>>>,
}
pub(crate) struct Match<'a> {
    bytes: &'a [u8],
    start: usize,
    end: usize,
    pub(crate) can_extend: bool,
}
impl<'a> Match<'a> {
    pub(crate) fn start(&self) -> usize {
        self.start
    }
    pub(crate) fn end(&self) -> usize {
        self.end
    }
    pub(crate) fn len(&self) -> usize {
        self.end - self.start
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.start == self.end
    }
    pub(crate) fn as_bytes(&self) -> &'a [u8] {
        &self.bytes[self.start..self.end]
    }
}
impl Ere {
    pub(crate) fn new(bytes: &[u8]) -> Result<Self, String> {
        validate_repetitions(bytes)?;
        let pattern = crate::types::regex_pattern_from_bytes(&normalize_brackets(bytes)?);
        let locator = regex::bytes::RegexBuilder::new(&pattern)
            .unicode(false)
            .dot_matches_new_line(true)
            .build()
            .map_err(|e| e.to_string())?;
        // Literal patterns have exactly one possible length; constructing the
        // all-matches DFA adds no information and is expensive for long FS.
        if !bytes.iter().any(|b| b"\\.^$|?*+()[]{}".contains(b)) {
            return Ok(Self {
                locator,
                longest: None,
            });
        }
        let longest = dense::Builder::new()
            .configure(
                dense::Config::new()
                    .match_kind(MatchKind::All)
                    .dfa_size_limit(Some(4 * 1024 * 1024)),
            )
            .syntax(
                syntax::Config::new()
                    .unicode(false)
                    .utf8(false)
                    .dot_matches_new_line(true),
            )
            .build(&pattern)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            locator,
            longest: Some(longest),
        })
    }
    pub(crate) fn is_match(&self, bytes: &[u8]) -> bool {
        self.locator.is_match(bytes)
    }
    pub(crate) fn find<'a>(&self, bytes: &'a [u8]) -> Option<Match<'a>> {
        self.find_at(bytes, 0)
    }
    pub(crate) fn find_at<'a>(&self, bytes: &'a [u8], offset: usize) -> Option<Match<'a>> {
        let first = self.locator.find_at(bytes, offset)?;
        let start = first.start();
        let Some(longest) = &self.longest else {
            return Some(Match {
                bytes,
                start,
                end: first.end(),
                can_extend: false,
            });
        };
        let input = Input::new(bytes)
            .span(start..bytes.len())
            .anchored(Anchored::Yes);
        let mut state = longest
            .start_state_forward(&input)
            .expect("byte DFA supports anchored search");
        let mut end = first.end();
        for (i, byte) in bytes.iter().enumerate().skip(start) {
            state = longest.next_state(state, *byte);
            if longest.is_match_state(state) {
                end = i;
            }
            if longest.is_dead_state(state) {
                return Some(Match {
                    bytes,
                    start,
                    end,
                    can_extend: false,
                });
            }
        }
        let last = longest.next_eoi_state(state);
        if longest.is_match_state(last) {
            end = bytes.len();
        }
        Some(Match {
            bytes,
            start,
            end,
            can_extend: !longest.is_dead_state(state),
        })
    }
}

pub(crate) fn split(bytes: &[u8], separator: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if separator == b" " {
        return Ok(bytes
            .split(|b| matches!(b, b' ' | b'\t' | b'\n'))
            .filter(|p| !p.is_empty())
            .map(Vec::from)
            .collect());
    }
    if separator.is_empty() {
        return Ok(bytes.iter().map(|b| vec![*b]).collect());
    }
    if separator.len() == 1 {
        return Ok(bytes.split(|b| *b == separator[0]).map(Vec::from).collect());
    }
    Ok(split_using(bytes, &Ere::new(separator)?))
}

pub(crate) fn split_using(bytes: &[u8], re: &Ere) -> Vec<Vec<u8>> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut parts = Vec::new();
    let mut start = 0;
    let mut search = 0;
    while search <= bytes.len() {
        let Some(m) = re.find_at(bytes, search) else {
            break;
        };
        if m.is_empty() {
            search = m.end() + 1;
            continue;
        }
        parts.push(bytes[start..m.start()].to_vec());
        start = m.end();
        search = start;
    }
    parts.push(bytes[start..].to_vec());
    parts
}

// Match the original C engine's repetition bound without interpreting escaped
// braces or digits inside bracket expressions as quantifiers.
fn validate_repetitions(pattern: &[u8]) -> Result<(), String> {
    let mut i = 0;
    while i < pattern.len() {
        match pattern[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'[' => {
                i += 1;
                if pattern.get(i) == Some(&b'^') {
                    i += 1;
                }
                if pattern.get(i) == Some(&b']') {
                    i += 1;
                }
                while i < pattern.len() && pattern[i] != b']' {
                    if pattern[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if pattern[i] == b'[' && pattern.get(i + 1).is_some_and(|b| b":.=".contains(b))
                    {
                        let delimiter = pattern[i + 1];
                        i += 2;
                        while i + 1 < pattern.len()
                            && !(pattern[i] == delimiter && pattern[i + 1] == b']')
                        {
                            i += 1;
                        }
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }
            b'{' => {
                let mut j = i + 1;
                let mut count = 0u32;
                while let Some(&b) = pattern.get(j) {
                    if b.is_ascii_digit() {
                        count = count * 10 + u32::from(b - b'0');
                        if count > 255 {
                            return Err("repetition count exceeds 255".into());
                        }
                    } else if b == b',' {
                        count = 0;
                    } else {
                        break;
                    }
                    j += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    Ok(())
}

/// Expand bracket expressions in the byte profile before handing them to Rust's
/// regex engine. BWK drops a descending range rather than rejecting the pattern;
/// ranges are evaluated left to right, including chained ranges such as a-z-a.
fn normalize_brackets(pattern: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(pattern.len());
    let mut i = 0;
    while i < pattern.len() {
        if pattern[i] == b'\\' {
            out.push(pattern[i]);
            i += 1;
            if let Some(&next) = pattern.get(i) {
                out.push(next);
                i += 1;
            }
        } else if pattern[i] != b'[' {
            out.push(pattern[i]);
            i += 1;
        } else {
            i += 1;
            let negate = pattern.get(i) == Some(&b'^');
            if negate {
                i += 1;
            }
            let mut tokens = Vec::new();
            while i < pattern.len() && (pattern[i] != b']' || tokens.is_empty()) {
                if pattern[i] == b'\\' {
                    i += 1;
                    tokens.push((quoted_class_byte(pattern, &mut i)?, false));
                } else if pattern.get(i..i + 2) == Some(b"[:") {
                    let start = i + 2;
                    let end = pattern[start..]
                        .windows(2)
                        .position(|b| b == b":]")
                        .map(|n| start + n)
                        .ok_or("unterminated named character class")?;
                    let name = &pattern[start..end];
                    for b in 0u8..=255 {
                        let member = match name {
                            b"alnum" => b.is_ascii_alphanumeric(),
                            b"alpha" => b.is_ascii_alphabetic(),
                            b"blank" => matches!(b, b' ' | b'\t'),
                            b"cntrl" => b.is_ascii_control(),
                            b"digit" => b.is_ascii_digit(),
                            b"graph" => (33..=126).contains(&b),
                            b"lower" => b.is_ascii_lowercase(),
                            b"print" => (32..=126).contains(&b),
                            b"punct" => b.is_ascii_punctuation(),
                            b"space" => b == b' ' || (9..=13).contains(&b),
                            b"upper" => b.is_ascii_uppercase(),
                            b"xdigit" => b.is_ascii_hexdigit(),
                            _ => return Err("unknown named character class".into()),
                        };
                        if member {
                            tokens.push((b, false));
                        }
                    }
                    i = end + 2;
                } else {
                    tokens.push((pattern[i], pattern[i] == b'-'));
                    i += 1;
                }
            }
            if pattern.get(i) != Some(&b']') {
                return Err("unterminated character class".into());
            }
            i += 1;
            let mut members = Vec::new();
            let mut t = 0;
            while t < tokens.len() {
                let (byte, dash) = tokens[t];
                if dash && !members.is_empty() && t + 1 < tokens.len() {
                    let start = *members.last().unwrap();
                    let end = tokens[t + 1].0;
                    if start > end {
                        members.pop();
                    } else {
                        members.extend((u16::from(start) + 1..=u16::from(end)).map(|n| n as u8));
                    }
                    t += 2;
                } else {
                    members.push(byte);
                    t += 1;
                }
            }
            if members.is_empty() {
                out.extend_from_slice(if negate { b"[\x00-\xff]" } else { b"(?:)" });
            } else {
                out.push(b'[');
                if negate {
                    out.push(b'^');
                }
                // Escape every member so Rust cannot reinterpret a literal
                // dash, bracket, ampersand or byte as its own class syntax.
                members.sort_unstable();
                members.dedup();
                for b in members {
                    out.extend_from_slice(format!("\\x{b:02x}").as_bytes());
                }
                out.push(b']');
            }
        }
    }
    Ok(out)
}

fn quoted_class_byte(pattern: &[u8], i: &mut usize) -> Result<u8, String> {
    let b = *pattern
        .get(*i)
        .ok_or("trailing backslash in character class")?;
    *i += 1;
    let value = match b {
        b'a' => 7,
        b'b' => 8,
        b't' => 9,
        b'n' => 10,
        b'v' => 11,
        b'f' => 12,
        b'r' => 13,
        b'x' | b'u' => {
            let limit = if b == b'x' { 2 } else { 8 };
            let mut value = 0u32;
            let mut count = 0;
            while count < limit {
                let Some(d) = pattern.get(*i).and_then(|b| (*b as char).to_digit(16)) else {
                    break;
                };
                value = value * 16 + d;
                *i += 1;
                count += 1;
            }
            if count == 0 {
                return Err("missing hex digits in character class".into());
            }
            return u8::try_from(value)
                .map_err(|_| "character class escape exceeds byte profile".into());
        }
        b'0'..=b'7' => {
            let mut value = u16::from(b - b'0');
            for _ in 0..2 {
                let Some(&d @ b'0'..=b'7') = pattern.get(*i) else {
                    break;
                };
                value = value * 8 + u16::from(d - b'0');
                *i += 1;
            }
            return u8::try_from(value)
                .map_err(|_| "character class escape exceeds byte profile".into());
        }
        _ => b,
    };
    Ok(value)
}

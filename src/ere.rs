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
        let pattern = crate::types::regex_pattern_from_bytes(bytes);
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

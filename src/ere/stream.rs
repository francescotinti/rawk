//! BWK fnematch's Shift-JIS input visibility, separate from in-memory regexes.
use regex_automata::{
    Anchored, Input, MatchKind,
    dfa::{Automaton, dense},
    util::{primitives::StateID, syntax},
};

pub(crate) struct StreamEre {
    dfa: dense::DFA<Vec<u32>>,
}

pub(crate) enum Progress {
    NeedMore,
    Found { start: usize, end: usize },
    Done,
}

impl StreamEre {
    pub(crate) fn new(pattern: &[u8]) -> Result<Self, String> {
        super::validate_repetitions(pattern)?;
        let pattern = crate::unicode_ere::compile_stream(pattern)?;
        let dfa = dense::Builder::new()
            .configure(
                dense::Config::new()
                    .match_kind(MatchKind::All)
                    .dfa_size_limit(Some(4 * 1024 * 1024)),
            )
            .syntax(syntax::Config::new().unicode(false).utf8(false))
            .build(&pattern)
            .map_err(|e| e.to_string())?;
        Ok(Self { dfa })
    }

    fn initial(&self, beginning: bool) -> StateID {
        // The retained prefix suppresses ^ after the first candidate/file read.
        let input = Input::new(b"x")
            .span(if beginning { 0..0 } else { 1..1 })
            .anchored(Anchored::Yes);
        self.dfa
            .start_state_forward(&input)
            .expect("byte DFA supports anchored search")
    }

    pub(crate) fn search(&self, beginning: bool) -> Search<'_> {
        Search {
            re: self,
            state: self.initial(beginning),
            origin: 0,
            cursor: 0,
            visible: 0,
            best: None,
        }
    }
}

pub(crate) struct Search<'a> {
    re: &'a StreamEre,
    state: StateID,
    origin: usize,
    cursor: usize,
    visible: usize,
    best: Option<usize>,
}

impl Search<'_> {
    /// Offsets are relative to the current record, even if the physical reader
    /// compacts its buffer. Keep this search alive across physical short reads.
    pub(crate) fn resume(&mut self, bytes: &[u8], eof: bool) -> Progress {
        loop {
            // C reads MB_CUR_MAX bytes, not merely the missing lookahead. In
            // SJIS that is two: after consuming one byte there may be three
            // visible bytes. Restarting a failed candidate retains lookahead.
            if self.visible.saturating_sub(self.cursor) < 2 {
                let next = self.visible.saturating_add(2);
                if next > bytes.len() && !eof {
                    return Progress::NeedMore;
                }
                self.visible = next.min(bytes.len());
            }
            let at_end = self.cursor == bytes.len() && eof;
            let (rune, size) = if at_end {
                (crate::unicode_ere::STREAM_EOF, 0)
            } else {
                crate::text::rune(&bytes[self.cursor..self.visible])
            };
            self.cursor += size;
            for byte in rune.to_be_bytes() {
                self.state = self.re.dfa.next_state(self.state, byte);
            }
            // The byte DFA delays acceptance by a byte. Its EOI transition
            // observes acceptance without consuming another rune. $ is an
            // explicit out-of-range rune, so this cannot invent a false EOF.
            if self
                .re
                .dfa
                .is_match_state(self.re.dfa.next_eoi_state(self.state))
                && self.cursor > self.origin
            {
                self.best = Some(self.cursor);
            }
            if !at_end && !self.re.dfa.is_dead_state(self.state) {
                continue;
            }
            if let Some(end) = self.best {
                return Progress::Found {
                    start: self.origin,
                    end,
                };
            }
            if self.origin == bytes.len() {
                return Progress::Done;
            }
            self.origin += crate::text::rune(&bytes[self.origin..self.visible]).1;
            self.cursor = self.origin;
            self.state = self.re.initial(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_short_reads_do_not_change_bwk_lookahead() {
        for (pattern, input, expected) in [
            (b"..".as_slice(), b"\xe1\x80\x80z".as_slice(), Some((0, 2))),
            (b"..", b"a\xe1\x80\x80z", Some((0, 4))),
            (b"..", b"\xf0\x90\x80\x80z", Some((0, 2))),
            (b".+", b"a\xe1\x80\x80z", Some((0, 5))),
            (b"x+", b"\xe1\x80\x80xxz", Some((3, 5))),
            (b"x+$", b"\xe1\x80\x80xx", Some((3, 5))),
            (b"^x+", b"ax", None),
            (b"x?", b"aaa", None),
            (b".+", b"a\0b", Some((0, 3))),
        ] {
            let re = StreamEre::new(pattern).unwrap();
            for chunk in [1, 2, 3, 7, 8192] {
                let mut search = re.search(true);
                let mut available = 0;
                let actual = loop {
                    match search.resume(&input[..available], available == input.len()) {
                        Progress::NeedMore => available = (available + chunk).min(input.len()),
                        Progress::Found { start, end } => break Some((start, end)),
                        Progress::Done => break None,
                    }
                };
                assert_eq!(actual, expected, "pattern={pattern:?}, chunk={chunk}");
            }
        }
    }
}

//! Character units and immutable LC_CTYPE selection. No process-global setlocale.
use std::{
    ffi::{CStr, CString},
    sync::OnceLock,
};

struct Locale {
    handle: usize,
    utf8: bool,
    shift_jis: bool,
    legacy_byte: bool,
}
static LOCALE: OnceLock<Locale> = OnceLock::new();

// libc does not expose Darwin's mbstate_t. Its SDK defines a union of
// char[128] and long long (arm/_types.h and i386/_types.h).
#[cfg(target_vendor = "apple")]
#[repr(C)]
union MbState {
    bytes: [libc::c_char; 128],
    alignment: libc::c_longlong,
}
#[cfg(not(target_vendor = "apple"))]
type MbState = libc::mbstate_t;

unsafe extern "C" {
    fn mbrtowc(
        wc: *mut libc::wchar_t,
        bytes: *const libc::c_char,
        len: usize,
        state: *mut MbState,
    ) -> usize;
    fn wcrtomb(bytes: *mut libc::c_char, wc: libc::wchar_t, state: *mut MbState) -> usize;
    fn nl_langinfo_l(item: libc::nl_item, locale: libc::locale_t) -> *mut libc::c_char;
    fn towupper_l(c: u32, locale: libc::locale_t) -> u32;
    fn towlower_l(c: u32, locale: libc::locale_t) -> u32;
    fn toupper_l(c: i32, locale: libc::locale_t) -> i32;
    fn tolower_l(c: i32, locale: libc::locale_t) -> i32;
    fn isalnum_l(c: i32, locale: libc::locale_t) -> i32;
    fn isalpha_l(c: i32, locale: libc::locale_t) -> i32;
    fn isblank_l(c: i32, locale: libc::locale_t) -> i32;
    fn iscntrl_l(c: i32, locale: libc::locale_t) -> i32;
    fn isdigit_l(c: i32, locale: libc::locale_t) -> i32;
    fn isgraph_l(c: i32, locale: libc::locale_t) -> i32;
    fn islower_l(c: i32, locale: libc::locale_t) -> i32;
    fn isprint_l(c: i32, locale: libc::locale_t) -> i32;
    fn ispunct_l(c: i32, locale: libc::locale_t) -> i32;
    fn isspace_l(c: i32, locale: libc::locale_t) -> i32;
    fn isupper_l(c: i32, locale: libc::locale_t) -> i32;
    fn isxdigit_l(c: i32, locale: libc::locale_t) -> i32;
}

/// Select LC_CTYPE only for this synchronous conversion and restore the
/// caller's thread locale on every exit, including errors. No global setlocale
/// or shared implicit mbtowc/wctomb conversion state is used.
struct ThreadLocale(libc::locale_t);
impl Drop for ThreadLocale {
    fn drop(&mut self) {
        unsafe { libc::uselocale(self.0) };
    }
}

fn convert_shift_jis(bytes: &[u8], upper: bool) -> Result<Vec<u8>, String> {
    let handle = LOCALE.get().unwrap().handle as libc::locale_t;
    let previous = unsafe { libc::uselocale(handle) };
    if previous.is_null() {
        return Err("cannot select case conversion locale".into());
    }
    let _restore = ThreadLocale(previous);
    // An all-zero mbstate_t is the initial conversion state on both targets.
    let mut decode: MbState = unsafe { std::mem::zeroed() };
    let mut encode: MbState = unsafe { std::mem::zeroed() };
    let mut out = Vec::with_capacity(bytes.len());
    let mut offset = 0;
    while offset < bytes.len() {
        let mut wc = 0;
        let n = unsafe {
            mbrtowc(
                &mut wc,
                bytes[offset..].as_ptr().cast(),
                bytes.len() - offset,
                &mut decode,
            )
        };
        if n == usize::MAX || n == usize::MAX - 1 {
            return Err("illegal byte sequence in case conversion".into());
        }
        // Unlike C strings, Rust's deliberate binary extension continues
        // beyond NUL. mbrtowc consumes that byte but returns zero.
        offset += n.max(1);
        let mapped = unsafe {
            if upper {
                towupper_l(wc as u32, handle)
            } else {
                towlower_l(wc as u32, handle)
            }
        };
        // Shift-JIS emits at most two bytes; leave MB_LEN_MAX-sized room
        // (Darwin 6, glibc 16). This path only accepts the SJIS codeset.
        let mut buffer = [0u8; 16];
        let n = unsafe {
            wcrtomb(
                buffer.as_mut_ptr().cast(),
                mapped as libc::wchar_t,
                &mut encode,
            )
        };
        if n == usize::MAX {
            return Err("illegal wide character".into());
        }
        out.extend_from_slice(&buffer[..n]);
    }
    Ok(out)
}

pub(crate) fn init() {
    LOCALE.get_or_init(|| {
        let name = ["LC_ALL", "LC_CTYPE", "LANG"]
            .iter()
            .find_map(|k| std::env::var(k).ok().filter(|s| !s.is_empty()))
            .unwrap_or_else(|| "C".into());
        let name = CString::new(name).unwrap_or_default();
        // The locale object is immutable and intentionally lives until process
        // exit. Queries never mutate libc's global locale or numeric parsing.
        let handle =
            unsafe { libc::newlocale(libc::LC_CTYPE_MASK, name.as_ptr(), std::ptr::null_mut()) };
        if handle.is_null() {
            return Locale {
                handle: 0,
                utf8: false,
                shift_jis: false,
                legacy_byte: false,
            };
        }
        let codeset = unsafe { CStr::from_ptr(nl_langinfo_l(libc::CODESET, handle)) }.to_bytes();
        let utf8 = codeset.eq_ignore_ascii_case(b"UTF-8") || codeset.eq_ignore_ascii_case(b"UTF8");
        // Only explicitly supported single-byte encodings enter this path.
        // A non-UTF-8 codeset is not necessarily single-byte (e.g. Shift-JIS).
        let normalized: Vec<u8> = codeset
            .iter()
            .filter(|&&b| b != b'-' && b != b'_')
            .map(u8::to_ascii_uppercase)
            .collect();
        let legacy_byte = matches!(normalized.as_slice(), b"ISO88591" | b"ISO88599");
        Locale {
            handle: handle as usize,
            utf8,
            shift_jis: matches!(normalized.as_slice(), b"SJIS" | b"SHIFTJIS"),
            legacy_byte,
        }
    });
}
/// BWK uses its structural UTF decoder for supported multibyte locales,
/// including Shift-JIS. This describes character units, not the codeset.
pub(crate) fn multibyte() -> bool {
    LOCALE.get().is_some_and(|l| l.utf8 || l.shift_jis)
}
pub(crate) fn legacy_byte_locale() -> bool {
    LOCALE.get().is_some_and(|l| l.legacy_byte)
}

/// BWK's structural decoder: malformed/truncated sequences consume one byte;
/// complete sequences follow its 2/3/4-byte rule, including non-scalar values.
pub(crate) fn rune(bytes: &[u8]) -> (u32, usize) {
    let Some(&b) = bytes.first() else {
        return (0, 0);
    };
    let n = match b {
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        _ => 1,
    };
    if n == 1 || bytes.len() < n || !bytes[1..n].iter().all(|b| b & 0xc0 == 0x80) {
        return (u32::from(b), 1);
    }
    let mut value = u32::from(b & (0x7f >> n));
    for b in &bytes[1..n] {
        value = (value << 6) | u32::from(b & 63);
    }
    (value, n)
}
pub(crate) fn next_len(bytes: &[u8]) -> usize {
    if multibyte() {
        rune(bytes).1
    } else {
        usize::from(!bytes.is_empty())
    }
}
pub(crate) fn advance(bytes: &[u8], at: usize) -> usize {
    if at >= bytes.len() {
        bytes.len().saturating_add(1)
    } else {
        at + next_len(&bytes[at..])
    }
}
pub(crate) fn byte_offset(bytes: &[u8], count: usize) -> usize {
    if !multibyte() {
        return count.min(bytes.len());
    }
    let mut i = 0;
    for _ in 0..count {
        if i == bytes.len() {
            break;
        }
        i += rune(&bytes[i..]).1;
    }
    i
}
pub(crate) fn len(bytes: &[u8]) -> usize {
    if !multibyte() {
        return bytes.len();
    }
    let (mut i, mut n) = (0, 0);
    while i < bytes.len() {
        i += rune(&bytes[i..]).1;
        n += 1;
    }
    n
}
pub(crate) fn convert(bytes: &[u8], upper: bool) -> Result<Vec<u8>, String> {
    if LOCALE.get().is_some_and(|l| l.shift_jis) {
        return convert_shift_jis(bytes, upper);
    }
    if legacy_byte_locale() {
        let handle = LOCALE.get().unwrap().handle as libc::locale_t;
        return Ok(bytes
            .iter()
            .map(|&b| {
                // ctype accepts unsigned-char values. Preserve embedded NUL,
                // as in the existing Rust binary-string extension.
                let mapped = unsafe {
                    if upper {
                        toupper_l(i32::from(b), handle)
                    } else {
                        tolower_l(i32::from(b), handle)
                    }
                };
                mapped as u8
            })
            .collect());
    }
    if !multibyte() {
        return Ok(if upper {
            bytes.to_ascii_uppercase()
        } else {
            bytes.to_ascii_lowercase()
        });
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| "illegal byte sequence in case conversion")?;
    let handle = LOCALE.get().unwrap().handle as libc::locale_t;
    let mut out = Vec::new();
    for c in text.chars() {
        let mapped = unsafe {
            if upper {
                towupper_l(c as u32, handle)
            } else {
                towlower_l(c as u32, handle)
            }
        };
        let c = char::from_u32(mapped).ok_or("illegal wide character")?;
        out.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
    }
    Ok(out)
}
pub(crate) fn class_member(name: &str, b: u8) -> Result<bool, String> {
    let handle = LOCALE.get().unwrap().handle as libc::locale_t;
    let c = i32::from(b);
    // Match the original's byte ctype table, not Unicode property classes.
    let result = unsafe {
        match name {
            "alnum" => isalnum_l(c, handle),
            "alpha" => isalpha_l(c, handle),
            "blank" => isblank_l(c, handle),
            "cntrl" => iscntrl_l(c, handle),
            "digit" => isdigit_l(c, handle),
            "graph" => isgraph_l(c, handle),
            "lower" => islower_l(c, handle),
            "print" => isprint_l(c, handle),
            "punct" => ispunct_l(c, handle),
            "space" => isspace_l(c, handle),
            "upper" => isupper_l(c, handle),
            "xdigit" => isxdigit_l(c, handle),
            _ => return Err("unknown named character class".into()),
        }
    };
    Ok(result != 0)
}

/// Source \u escapes follow BWK's rune encoder, including its error rune 128.
pub(crate) fn encode_rune(mut c: u32) -> Vec<u8> {
    if c > 0x10ffff {
        c = 128;
    }
    if c < 128 {
        vec![c as u8]
    } else if c < 0x800 {
        vec![0xc0 | (c >> 6) as u8, 0x80 | (c & 63) as u8]
    } else if c < 0x10000 {
        vec![
            0xe0 | (c >> 12) as u8,
            0x80 | ((c >> 6) & 63) as u8,
            0x80 | (c & 63) as u8,
        ]
    } else {
        vec![
            0xf0 | (c >> 18) as u8,
            0x80 | ((c >> 12) & 63) as u8,
            0x80 | ((c >> 6) & 63) as u8,
            0x80 | (c & 63) as u8,
        ]
    }
}

pub(crate) fn character_index(bytes: &[u8], offset: usize) -> usize {
    if !multibyte() {
        return offset + 1;
    }
    let (mut i, mut count) = (0, 0);
    while i <= offset && i < bytes.len() {
        i += rune(&bytes[i..]).1;
        count += 1;
    }
    count
}

/// A suffix that may become one character when the next read arrives must not
/// be interpreted as invalid standalone bytes before EOF is known.
pub(crate) fn incomplete_tail(bytes: &[u8]) -> Option<usize> {
    for start in bytes.len().saturating_sub(3)..bytes.len() {
        let n = match bytes[start] {
            0xc0..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf7 => 4,
            _ => 1,
        };
        if n > bytes.len() - start && bytes[start + 1..].iter().all(|b| b & 0xc0 == 0x80) {
            return Some(start);
        }
    }
    None
}

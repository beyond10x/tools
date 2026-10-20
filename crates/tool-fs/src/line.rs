//! One line of a file at a time, with its true length kept whatever is held of it, and the digest
//! of every byte consumed fed along the way.

use std::io::{self, BufRead};

use sha2::{Digest, Sha256};

/// One line of a file: what is kept of it, how long it really was, and whether the two agree.
pub(crate) struct Line {
    /// The line's bytes, up to what the reader was told to keep, with a `\r` before the newline
    /// dropped.
    pub(crate) kept: Vec<u8>,
    /// Its true length in the file, before anything was dropped: separator excluded, `\r`
    /// included. What the byte ceiling is charged.
    pub(crate) length: u64,
    /// Whether `kept` holds the whole line. Not `kept.len() == length`: a stripped `\r` makes the
    /// two differ on a file the reader saw all of, and a reply that called that line *cut* would
    /// name every line of a CRLF file.
    pub(crate) whole: bool,
    /// Whether a newline ended the line. A last line without one is a line, but there is no
    /// separator after it to charge to any bound.
    pub(crate) terminated: bool,
}

/// The next line from `reader`, keeping at most `keep` of its bytes, with every byte consumed
/// (separators and a dropped `\r` included) fed to `digest`.
///
/// Its own function rather than [`BufRead::read_until`] for one reason: `read_until` on a file
/// with no newline in it allocates the whole file. A minified bundle is exactly that, and a read
/// of it must not be a way to spend a gigabyte of this process's memory. The line's true length is
/// counted whatever is kept, so the reply can say the line was cut.
///
/// **A `\r` before the newline is dropped**, which is what `str::lines` does. A `\r` a model
/// quotes back to an edit matches nothing in a file it was just handed, so the reader does not
/// hand one out. A trailing `\r` at the very end of a file with no newline after it is kept,
/// because that is a `\r` and not a line ending, and `str::lines` keeps it too.
///
/// The digest sees the bytes as they are in the file, so after the last line it is the digest of
/// the whole file.
///
/// **The scan is bounded in bytes, not in lines**: at most `budget` bytes are consumed. When the
/// budget runs out before a newline, the line is returned as far as it was read, `terminated`
/// false, and the caller's byte count (which the budget was computed from) is what says the
/// bound was passed. Without this a file over the counting bound that holds no newline would be
/// one line, consumed whole, and counted as if the bound did not exist.
///
/// `None` at the end of the file, or when `budget` is 0 before anything was read. A last line
/// with no trailing newline is a line.
pub(crate) fn next_line(
    reader: &mut impl BufRead,
    keep: usize,
    digest: &mut Sha256,
    budget: u64,
) -> io::Result<Option<Line>> {
    let mut line = Line {
        kept: Vec::new(),
        length: 0,
        whole: true,
        terminated: false,
    };
    let mut budget = budget;
    let mut any = false;
    loop {
        if budget == 0 {
            return Ok(any.then_some(line));
        }
        let (ends_at, consumed) = {
            let available = reader.fill_buf()?;
            if available.is_empty() {
                return Ok(any.then_some(line));
            }
            any = true;
            let allowed =
                usize::try_from(budget).map_or(available.len(), |most| most.min(available.len()));
            let available = &available[..allowed];
            let ends_at = available.iter().position(|byte| *byte == b'\n');
            let end = ends_at.unwrap_or(available.len());
            let room = keep.saturating_sub(line.kept.len());
            line.whole &= end <= room;
            line.kept.extend_from_slice(&available[..end.min(room)]);
            line.length += end as u64;
            let consumed = ends_at.map_or(available.len(), |at| at + 1);
            digest.update(&available[..consumed]);
            (ends_at, consumed)
        };
        reader.consume(consumed);
        budget -= consumed as u64;
        if ends_at.is_some() {
            line.terminated = true;
            if line.whole && line.kept.last() == Some(&b'\r') {
                line.kept.pop();
            }
            return Ok(Some(line));
        }
    }
}

/// How much of `bytes` is whole UTF-8, so a cut lands between characters rather than inside one.
pub(crate) fn whole_prefix(bytes: &[u8]) -> usize {
    match std::str::from_utf8(bytes) {
        // "the bytes ran out mid-character" is the cut this made; anything else is the file's own
        // encoding and is reported lossily, as it always was.
        Err(error) if error.error_len().is_none() => error.valid_up_to(),
        Ok(_) | Err(_) => bytes.len(),
    }
}

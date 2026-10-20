//! A child's stream, read to its end and kept at both ends: harness's `drain`, `read_ends`,
//! `keep_both_ends`, `whole_prefix` and `whole_suffix`.
//!
//! The ends live in [`Ends`], shared between the thread that reads the stream and the run that
//! answers with it, so a run can answer with what was read so far without waiting for the stream
//! to end: a process that left the run's group can hold the stream open for as long as it likes.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::io::Read;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};

/// What one of a child's streams said, bounded at both ends.
#[derive(Debug, Default)]
pub(crate) struct Drained {
    pub(crate) text: String,
    pub(crate) omitted: u64,
}

/// The two ends of a stream kept so far: its first `half` bytes and its last `half` bytes, and
/// how many bytes came **after** the head, enough with the two ends to say how many went missing
/// between them.
///
/// # The head is not where the answer is
///
/// Keeping only the first 64 KiB handed a caller the compiler's progress for the one program a
/// run cares most about, a test suite, whose answer is the last twenty lines. So both ends are
/// kept, and the middle says how many bytes went missing between them. Memory stays at the cap:
/// the tail is a ring, not a buffer of everything seen so far.
///
/// # And the bytes on the far side of each cut
///
/// Whether a cut broke a character depends on the bytes on both sides of it, and the far side is
/// what the cut drops. So the first [`BESIDE`] bytes after the head and the last [`BESIDE`]
/// bytes before the tail are kept too: enough to finish any character an end opens or closes.
#[derive(Debug)]
pub(crate) struct Ends {
    half: usize,
    head: Vec<u8>,
    /// The first bytes of the stream after the head, at most [`BESIDE`].
    after_head: Vec<u8>,
    tail: VecDeque<u8>,
    /// The last bytes that left the tail's ring, at most [`BESIDE`], in stream order.
    left_tail: VecDeque<u8>,
    beyond_head: u64,
}

/// How many bytes beside a cut are kept: the most of one character that can lie on the far side.
const BESIDE: usize = MAX_CHAR_BYTES - 1;

impl Ends {
    /// Nothing read yet, keeping `half` bytes at each end.
    pub(crate) fn new(half: usize) -> Self {
        Self {
            half,
            head: Vec::new(),
            after_head: Vec::new(),
            tail: VecDeque::new(),
            left_tail: VecDeque::new(),
            beyond_head: 0,
        }
    }

    /// Takes the next chunk of the stream.
    fn take(&mut self, mut bytes: &[u8]) {
        let room = self.half.saturating_sub(self.head.len());
        if room > 0 {
            let taken = room.min(bytes.len());
            self.head.extend_from_slice(&bytes[..taken]);
            bytes = &bytes[taken..];
        }
        let wanted = BESIDE - self.after_head.len();
        self.after_head
            .extend_from_slice(&bytes[..wanted.min(bytes.len())]);
        self.beyond_head += bytes.len() as u64;
        // A chunk at a time, then one drain: a program that prints a hundred megabytes would
        // otherwise pay a push and a pop per byte to keep a 32 KiB window.
        self.tail.extend(bytes.iter().copied());
        if self.tail.len() > self.half {
            let leaving = self.tail.len() - self.half;
            for at in leaving.saturating_sub(BESIDE)..leaving {
                self.left_tail.push_back(self.tail[at]);
            }
            while self.left_tail.len() > BESIDE {
                self.left_tail.pop_front();
            }
            self.tail.drain(..leaving);
        }
    }

    /// What the stream said so far, both ends kept and the middle marked.
    pub(crate) fn drained(&self) -> Drained {
        let tail: Vec<u8> = self.tail.iter().copied().collect();
        // The bytes just before the tail: those that left its ring, after the head's last ones
        // when fewer than [`BESIDE`] have left, because then they follow the head directly.
        let mut before_tail = self.head[self.head.len().saturating_sub(BESIDE)..].to_vec();
        before_tail.extend(self.left_tail.iter().copied());
        let before_tail = &before_tail[before_tail.len().saturating_sub(BESIDE)..];
        keep_both_ends(
            Cut {
                head: &self.head,
                after_head: &self.after_head,
                before_tail,
                tail: &tail,
            },
            self.head.len() as u64 + self.beyond_head,
        )
    }
}

/// The two ends of a stream and the bytes on the far side of each cut.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cut<'a> {
    /// The stream's first bytes.
    pub(crate) head: &'a [u8],
    /// The bytes right after `head`, at most [`BESIDE`].
    pub(crate) after_head: &'a [u8],
    /// The bytes right before `tail`, at most [`BESIDE`].
    pub(crate) before_tail: &'a [u8],
    /// The stream's last bytes.
    pub(crate) tail: &'a [u8],
}

/// Reads `stream` to the end into `ends`, a chunk at a time.
///
/// It keeps reading after the cap rather than stopping: a reader that walked away would block the
/// child on a full pipe, and the point of the cap is the size of the answer, not the child's fate.
pub(crate) fn read_ends(mut stream: impl Read, ends: &Mutex<Ends>) {
    let mut buffer = [0_u8; 8192];
    while let Ok(read) = stream.read(&mut buffer) {
        if read == 0 {
            break;
        }
        ends.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take(&buffer[..read]);
    }
}

/// One of a child's streams being read on a thread of its own, into ends the run can read at any
/// time.
///
/// On a thread rather than after `wait`: a child that fills a pipe buffer blocks until somebody
/// reads it, and a `wait` that has not read is the deadlock. Never joined: a run that stops
/// waiting answers with [`Self::drained`] and leaves the thread to end with the stream.
pub(crate) struct Drain {
    ends: Arc<Mutex<Ends>>,
    thread: Option<JoinHandle<()>>,
}

impl Drain {
    /// Starts reading `stream`, keeping `half` bytes at each end; with no stream, a drain that has
    /// already finished with nothing in it.
    pub(crate) fn start(stream: Option<impl Read + Send + 'static>, half: usize) -> Self {
        let ends = Arc::new(Mutex::new(Ends::new(half)));
        let thread = stream.map(|stream| {
            let ends = Arc::clone(&ends);
            thread::spawn(move || read_ends(stream, &ends))
        });
        Self { ends, thread }
    }

    /// Whether the stream has ended: every holder of its write end has closed it.
    pub(crate) fn finished(&self) -> bool {
        self.thread.as_ref().is_none_or(JoinHandle::is_finished)
    }

    /// What the stream said so far.
    pub(crate) fn drained(&self) -> Drained {
        self.ends
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .drained()
    }
}

/// Reads a whole stream and answers it as a run would: the drain of one stream, start to end.
#[cfg(test)]
pub(crate) fn drain(stream: impl Read) -> Drained {
    let ends = Mutex::new(Ends::new(crate::MAX_RUN_OUTPUT_BYTES / 2));
    read_ends(stream, &ends);
    ends.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .drained()
}

/// Joins the two ends of a stream of `total` bytes, with a marker naming what fell between them.
///
/// Its own function, and pure, because the arithmetic here is the part that can be wrong: the
/// character boundaries, the count in the marker, whether the last line survives.
///
/// `cut` holds the two ends as [`Ends`] kept them and the bytes beside each cut; `total` is the
/// whole stream's length. When the two ends are the whole stream, nothing is omitted and no
/// marker is written.
pub(crate) fn keep_both_ends(cut: Cut<'_>, total: u64) -> Drained {
    let Cut {
        head,
        after_head,
        before_tail,
        tail,
    } = cut;
    if head.len() as u64 + tail.len() as u64 >= total {
        let mut whole = head.to_vec();
        whole.extend_from_slice(tail);
        return Drained {
            text: String::from_utf8_lossy(&whole).into_owned(),
            omitted: 0,
        };
    }

    // Both cuts land on character boundaries. A cut through a multi-byte character becomes U+FFFD,
    // which reads as damage to the program's output rather than to this reply.
    let head = &head[..whole_prefix(head, after_head)];
    let tail = &tail[whole_suffix(before_tail, tail)..];
    // Counted from what is actually in the reply, so the bytes dropped for a character boundary
    // are in the figure too. A marker that undercounted would be a smaller lie than a silent cut
    // and still a lie.
    let omitted = total - head.len() as u64 - tail.len() as u64;
    let mut text = String::from_utf8_lossy(head).into_owned();
    let _ = write!(text, "\n… {omitted} bytes omitted here …\n");
    text.push_str(&String::from_utf8_lossy(tail));
    Drained { text, omitted }
}

/// The widest a UTF-8 character gets, in bytes.
const MAX_CHAR_BYTES: usize = 4;

fn is_continuation(byte: u8) -> bool {
    byte & 0b1100_0000 == 0b1000_0000
}

/// How many bytes the character a lead byte opens takes; `None` for a byte that opens none.
fn char_width(lead: u8) -> Option<usize> {
    match lead {
        0x00..=0x7f => Some(1),
        _ if lead & 0b1110_0000 == 0b1100_0000 => Some(2),
        _ if lead & 0b1111_0000 == 0b1110_0000 => Some(3),
        _ if lead & 0b1111_1000 == 0b1111_0000 => Some(4),
        _ => None,
    }
}

/// The character a cut between `before` and `after` broke, if it broke one: how many of its bytes
/// lie before the cut and how many after.
///
/// Only the last one to three bytes before the cut can open a character that runs past it, and it
/// was broken only when the bytes after the cut finish it into valid UTF-8. Anything else at the
/// cut, a byte that opens no character, one that the bytes after it do not continue, a run of
/// stray continuation bytes, is the stream's own encoding: it is answered lossily and not moved.
fn broken_at(before: &[u8], after: &[u8]) -> Option<(usize, usize)> {
    let len = before.len();
    for back in 1..=len.min(BESIDE) {
        let byte = before[len - back];
        if is_continuation(byte) {
            continue;
        }
        let missing = char_width(byte)?.checked_sub(back).filter(|n| *n > 0)?;
        let mut character = before[len - back..].to_vec();
        character.extend_from_slice(after.get(..missing)?);
        return std::str::from_utf8(&character)
            .is_ok()
            .then_some((back, missing));
    }
    None
}

/// How much of the head to keep so its cut lands between characters: all of it, unless the cut
/// broke the head's last character, which then leaves with the omitted bytes.
///
/// Harness asked whether the whole head was UTF-8, so one bad byte anywhere in it left the cut
/// through a character; looking at the head alone then moved the cut before a stray lead byte the
/// stream never continued. `after` is what decides it.
fn whole_prefix(head: &[u8], after: &[u8]) -> usize {
    head.len() - broken_at(head, after).map_or(0, |(before_cut, _)| before_cut)
}

/// Where the tail's whole characters start: past the bytes of a character the cut broke, and
/// nowhere else.
///
/// Harness skipped every continuation byte the tail opened with, and looking at the tail alone
/// skipped a stray one that followed a whole character; `before` is what decides it.
fn whole_suffix(before: &[u8], tail: &[u8]) -> usize {
    broken_at(before, tail).map_or(0, |(_, after_cut)| after_cut)
}

//! The interactive TTY picker (issue 126 follow-up — the owner's ask: the
//! typed `show which (seq, …)` prompt was hard to read). Arrow-key
//! selection over the open checkouts, the newest preselected (Enter
//! immediately = newest), `v` arms a void (a second keystroke confirms —
//! a void is quasi-destructive, one fat-finger must not spend it), `q`/
//! Esc/Ctrl+C quit. The pure engine is lib-tested at default features
//! through scripted key readers; the raw-mode reader is unix-only via
//! libc's termios (already in the lock transitively — zero new packages)
//! and a terminal that refuses raw mode falls back to the typed prompt
//! verbatim (the piped/agent posture never enters here — stdin is not a
//! tty and the caller goes straight to the typed/flag path).

use std::io::Write;

/// One selectable checkout row. The caller pre-renders `text` — the SAME
/// string the 429 listing prints, so the picker can never disagree with
/// the list above it.
pub struct PickRow {
    pub seq: u64,
    pub text: String,
}

/// A decoded key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Enter,
    Char(char),
    Esc,
    CtrlC,
}

/// The key source. `None` = end of input (cancels).
pub trait KeyReader {
    fn read_key(&mut self) -> Option<Key>;
}

/// What the picker decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picked {
    /// Re-print this checkout's QR.
    Show(u64),
    /// Release this checkout's slot (already confirmed in-band).
    Void(u64),
    /// Nothing chosen — the caller says so and exits 2.
    Cancelled,
}

/// Run the picker: draw, read a key, redraw in place. Rows arrive
/// newest-first (the server's order); index 0 starts selected — the
/// owner's "default to the last one".
pub fn pick<K: KeyReader, W: Write>(rows: &[PickRow], keys: &mut K, out: &mut W) -> Picked {
    if rows.is_empty() {
        return Picked::Cancelled;
    }
    let mut sel = 0usize;
    // The armed void: `v` highlights the pending release and draws the
    // confirm line; any navigation key (or q/Esc/n) disarms it.
    let mut armed: Option<usize> = None;
    let mut drawn = 0usize;
    loop {
        drawn = draw(out, rows, sel, armed, drawn);
        let _ = out.flush();
        let Some(key) = keys.read_key() else {
            return Picked::Cancelled;
        };
        match key {
            Key::Up | Key::Char('k') => {
                sel = sel.saturating_sub(1);
                armed = None;
            }
            Key::Down | Key::Char('j') => {
                if sel + 1 < rows.len() {
                    sel += 1;
                }
                armed = None;
            }
            Key::Enter => {
                let seq = rows[sel].seq;
                return if armed == Some(sel) {
                    Picked::Void(seq)
                } else {
                    Picked::Show(seq)
                };
            }
            Key::Char('v' | 'V') => armed = Some(sel),
            Key::Char('y' | 'Y') if armed.is_some() => {
                return Picked::Void(rows[sel].seq);
            }
            Key::Char('n' | 'N') => armed = None,
            Key::Char('q' | 'Q') | Key::Esc => {
                if armed.is_some() {
                    armed = None;
                } else {
                    return Picked::Cancelled;
                }
            }
            Key::CtrlC => return Picked::Cancelled,
            Key::Char(_) => {}
        }
    }
}

/// Redraw the block in place: move the cursor up over the previous frame,
/// clear to end of screen, print the new frame. Returns the line count to
/// skip on the next redraw (header + rows + the optional confirm line).
fn draw<W: Write>(
    out: &mut W,
    rows: &[PickRow],
    sel: usize,
    armed: Option<usize>,
    prev: usize,
) -> usize {
    use std::fmt::Write as _;
    let mut buf = String::new();
    if prev > 0 {
        let _ = write!(buf, "\x1b[{prev}A\x1b[J");
    }
    let _ = writeln!(buf, "  \x1b[1m↑/↓ move · Enter = show · v = void · q = quit\x1b[0m");
    for (i, r) in rows.iter().enumerate() {
        let marker = if i == sel { '▸' } else { ' ' };
        if i == sel {
            let _ = writeln!(buf, "{marker} \x1b[1m{}\x1b[0m", r.text);
        } else {
            let _ = writeln!(buf, "{marker} {}", r.text);
        }
    }
    if let Some(i) = armed {
        let _ = writeln!(
            buf,
            "  void seq {}? \x1b[1my\x1b[0m = confirm · any other key cancels",
            rows[i].seq
        );
    }
    let _ = out.write_all(buf.as_bytes());
    1 + rows.len() + usize::from(armed.is_some())
}

/// Raw-mode key reader (unix): canonical mode, echo and signals off, so
/// arrows and plain keys arrive immediately and Ctrl+C arrives as a byte
/// (the Drop restores the saved termios exactly — the terminal is never
/// left raw). ISIG stays off on purpose: a signal exit would skip the
/// Drop and orphan the terminal.
#[cfg(unix)]
pub struct TtyKeys {
    saved: libc::termios,
    raw: libc::termios,
}

#[cfg(unix)]
impl TtyKeys {
    /// `None` = not a tty or the termios dance failed — the caller falls
    /// back to the typed prompt.
    #[must_use]
    pub fn enable() -> Option<Self> {
        if unsafe { libc::isatty(0) } == 0 {
            return None;
        }
        let mut saved: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(0, &raw mut saved) } != 0 {
            return None;
        }
        let mut raw = saved;
        raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
        raw.c_iflag &= !(libc::IXON | libc::ICRNL);
        raw.c_cc[libc::VMIN] = 1;
        raw.c_cc[libc::VTIME] = 0;
        if unsafe { libc::tcsetattr(0, libc::TCSANOW, &raw const raw) } != 0 {
            return None;
        }
        Some(Self { saved, raw })
    }

    /// Re-aim the non-blocking window: `vtime` is in 0.1 s units (0 = the
    /// blocking posture). Used only around escape-sequence tails.
    fn set_wait(&mut self, vmin: u8, vtime: u8) {
        self.raw.c_cc[libc::VMIN] = vmin;
        self.raw.c_cc[libc::VTIME] = vtime;
        unsafe { libc::tcsetattr(0, libc::TCSANOW, &raw const self.raw) };
    }

    /// One byte from the controlling terminal (fd 0) — `None` on EOF or
    /// an unrecoverable read error. A static reader: the raw mode lives in
    /// the tty, not in `self`.
    fn read_byte() -> Option<u8> {
        let mut b = 0u8;
        loop {
            // SAFETY: fd 0 is open; `b` is a valid 1-byte destination.
            let n = unsafe { libc::read(0, (&raw mut b).cast(), 1) };
            if n == 1 {
                return Some(b);
            }
            if n == 0 {
                return None;
            }
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            return None;
        }
    }
}

#[cfg(unix)]
impl Drop for TtyKeys {
    fn drop(&mut self) {
        unsafe { libc::tcsetattr(0, libc::TCSANOW, &raw const self.saved) };
    }
}

#[cfg(unix)]
impl KeyReader for TtyKeys {
    fn read_key(&mut self) -> Option<Key> {
        self.set_wait(1, 0);
        loop {
            let b = Self::read_byte()?;
            match b {
                0x1b => {
                    // An escape sequence's tail arrives together with the
                    // ESC byte; a 100 ms window separates "arrow" from a
                    // lone Esc press without ever blocking on it.
                    self.set_wait(0, 1);
                    let tail = Self::read_byte();
                    let key = match tail {
                        Some(b'[' | b'O') => match Self::read_byte() {
                            Some(b'A') => Some(Key::Up),
                            Some(b'B') => Some(Key::Down),
                            _ => None, // an unknown sequence — swallow it
                        },
                        Some(_) => None, // an ESC chord we do not bind
                        None => Some(Key::Esc),
                    };
                    self.set_wait(1, 0);
                    if let Some(k) = key {
                        return Some(k);
                    }
                }
                b'\r' | b'\n' => return Some(Key::Enter),
                0x03 => return Some(Key::CtrlC),
                0x7f | 0x08 => return Some(Key::Esc),
                b if b.is_ascii_graphic() => return Some(Key::Char(b as char)),
                _ => {} // control bytes we do not bind — ignore
            }
        }
    }
}

/// Non-unix: no raw mode — `enable()` is always `None` and the typed
/// prompt stays the interaction (the picker engine is still lib-tested).
#[cfg(not(unix))]
pub struct TtyKeys;

#[cfg(not(unix))]
impl TtyKeys {
    #[must_use]
    pub fn enable() -> Option<Self> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scripted key source: each call pops the next key.
    struct Script(Vec<Option<Key>>);
    impl KeyReader for Script {
        fn read_key(&mut self) -> Option<Key> {
            if self.0.is_empty() {
                None
            } else {
                self.0.remove(0)
            }
        }
    }

    fn rows() -> Vec<PickRow> {
        vec![
            PickRow {
                seq: 4,
                text: "seq 4    1 USDC".to_string(),
            },
            PickRow {
                seq: 2,
                text: "seq 2    1 USDC".to_string(),
            },
        ]
    }

    fn run(keys: Vec<Option<Key>>) -> (Picked, String) {
        let mut out: Vec<u8> = Vec::new();
        let picked = pick(&rows(), &mut Script(keys), &mut out);
        (picked, String::from_utf8(out).expect("utf8"))
    }

    #[test]
    fn enter_immediately_shows_the_newest() {
        let (picked, out) = run(vec![Some(Key::Enter)]);
        assert_eq!(picked, Picked::Show(4));
        assert!(out.contains('▸'), "the newest preselected: {out}");
        assert!(out.contains("seq 4"), "{out}");
    }

    #[test]
    fn arrows_navigate_and_clamp() {
        let (picked, _) = run(vec![Some(Key::Down), Some(Key::Down), Some(Key::Enter)]);
        assert_eq!(picked, Picked::Show(2), "the bottom row clamps");
        let (picked, _) = run(vec![Some(Key::Up), Some(Key::Enter)]);
        assert_eq!(picked, Picked::Show(4), "the top row clamps");
        let (picked, _) = run(vec![Some(Key::Char('j')), Some(Key::Char('k')), Some(Key::Enter)]);
        assert_eq!(picked, Picked::Show(4), "j/k mirror the arrows");
    }

    #[test]
    fn the_marker_moves_with_the_selection() {
        let (_, out) = run(vec![Some(Key::Down), Some(Key::Enter)]);
        // The redraw escape skips prev_lines (header + 2 rows) upward; the
        // last frame is everything after that prefix.
        let last = out.rsplit("\x1b[3A").next().unwrap_or("");
        assert!(last.contains("▸ \x1b[1mseq 2"), "last frame: {last}");
        assert!(!last.contains("▸ \x1b[1mseq 4"), "the old marker is gone: {last}");
    }

    #[test]
    fn void_arms_then_confirms_by_enter_or_y() {
        let (picked, out) = run(vec![Some(Key::Char('v')), Some(Key::Enter)]);
        assert_eq!(picked, Picked::Void(4));
        assert!(out.contains("void seq 4?"), "the confirm line drew: {out}");
        let (picked, _) = run(vec![Some(Key::Char('v')), Some(Key::Char('y'))]);
        assert_eq!(picked, Picked::Void(4));
    }

    #[test]
    fn any_other_key_disarms_a_void() {
        let (picked, _) = run(vec![
            Some(Key::Char('v')),
            Some(Key::Char('n')),
            Some(Key::Enter),
        ]);
        assert_eq!(picked, Picked::Show(4));
        let (picked, _) = run(vec![
            Some(Key::Char('v')),
            Some(Key::Esc),
            Some(Key::Enter),
        ]);
        assert_eq!(picked, Picked::Show(4));
        // An arrow away disarms too.
        let (picked, _) = run(vec![
            Some(Key::Char('v')),
            Some(Key::Down),
            Some(Key::Enter),
        ]);
        assert_eq!(picked, Picked::Show(2));
    }

    #[test]
    fn quit_ctrl_c_and_eof_cancel() {
        let (picked, _) = run(vec![Some(Key::Char('q'))]);
        assert_eq!(picked, Picked::Cancelled);
        let (picked, _) = run(vec![Some(Key::CtrlC)]);
        assert_eq!(picked, Picked::Cancelled);
        let (picked, _) = run(vec![]);
        assert_eq!(picked, Picked::Cancelled);
    }

    #[test]
    fn empty_rows_cancel_without_drawing() {
        let mut out: Vec<u8> = Vec::new();
        let picked = pick(&[], &mut Script(vec![Some(Key::Enter)]), &mut out);
        assert_eq!(picked, Picked::Cancelled);
        assert!(out.is_empty());
    }
}

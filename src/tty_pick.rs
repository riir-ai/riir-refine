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
    /// End of input.
    Eof,
    /// The first-keypress window expired with nothing pressed — the
    /// agent-stuck defense: an unattended terminal never wedges the UI.
    Timeout,
}

/// The agent-stuck defense window (issue 126 follow-up, owner ask):
/// `VTIME` is 0.1 s units and `u8`, so 100 = a 10 s first-key window
/// (the `u8` ceiling is 25.5 s — well above).
pub const FIRST_KEY_TIMEOUT_DECISECS: u8 = 100;

/// The key source. The first key may carry a timeout window
/// (`TtyKeys::first_timeout`); after it, reads block.
pub trait KeyReader {
    fn read_key(&mut self) -> Key;
}

/// What the picker decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picked {
    /// Re-print this checkout's QR.
    Show(u64),
    /// Release this checkout's slot (already confirmed in-band).
    Void(u64),
    /// Nothing chosen (quit / Ctrl+C / end of input) — the caller says
    /// so and exits 2.
    Cancelled,
    /// The first-key window expired untouched — the caller prints the
    /// non-interactive hint (an agent is on the other end).
    TimedOut,
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
        match keys.read_key() {
            Key::Eof => return Picked::Cancelled,
            Key::Timeout => return Picked::TimedOut,
            key => match key {
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
                Key::Eof | Key::Timeout => unreachable!("handled above"),
            },
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
    let _ = writeln!(
        buf,
        "  \x1b[1m↑/↓ move · Enter = show · v = void · q = quit\x1b[0m"
    );
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
    /// The armed first-keypress window (deciseconds), consumed by the
    /// next `read_key` — the agent-stuck defense. `None` = block.
    first_timeout: Option<u8>,
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
        Some(Self {
            saved,
            raw,
            first_timeout: None,
        })
    }

    /// Arm the FIRST-keypress window (0.1 s units): if nothing is pressed
    /// within it, `read_key` answers `Key::Timeout` and the picker cancels
    /// — an agent (or a scrolled-away human) never wedges the UI. One
    /// window: after any key, reads block.
    pub fn first_timeout(&mut self, decisecs: u8) {
        self.first_timeout = Some(decisecs);
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
    fn read_key(&mut self) -> Key {
        let first = self.first_timeout.take();
        let wait = match first {
            Some(d) => (0u8, d),
            None => (1u8, 0u8),
        };
        self.set_wait(wait.0, wait.1);
        loop {
            match Self::read_byte() {
                None => {
                    // A VTIME expiry and a real EOF both read as 0/EIO —
                    // the window tells them apart (only the first read
                    // can time out).
                    return if first.is_some() {
                        Key::Timeout
                    } else {
                        Key::Eof
                    };
                }
                Some(0x1b) => {
                    // An escape sequence's tail arrives together with the
                    // ESC byte; a 100 ms window separates "arrow" from a
                    // lone Esc press without ever blocking on it. The
                    // wait posture re-arms to whatever this call started
                    // with (the first-key window keeps running).
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
                    self.set_wait(wait.0, wait.1);
                    if let Some(k) = key {
                        return k;
                    }
                }
                Some(b'\r' | b'\n') => return Key::Enter,
                Some(0x03) => return Key::CtrlC,
                Some(0x7f | 0x08) => return Key::Esc,
                Some(b) if b.is_ascii_graphic() => return Key::Char(b as char),
                Some(_) => {} // control bytes we do not bind — ignore
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

    pub fn first_timeout(&mut self, _decisecs: u8) {}
}

// Shape parity with the unix guard (whose Drop restores the saved
// termios exactly): the stub never entered raw mode, so its restore is
// vacuous — but call sites stay identical on every platform and
// `drop(keys)` stays meaningful instead of tripping clippy::drop_non_drop
// against a non-Drop ZST (heal.rs's topup picker). Never constructed at
// runtime: `enable()` is always `None` here, so this impl is compile-time
// shape only, exactly like the KeyReader stub below.
#[cfg(not(unix))]
impl Drop for TtyKeys {
    fn drop(&mut self) {}
}

// Compile-time reachability only: `enable()` is always `None` on this
// platform, so a `TtyKeys` can never reach `pick` at runtime — but the
// call site's `K: KeyReader` bound is checked at compile time on every
// platform. The stub answers EOF (the caller's cancel path), mirroring
// the unix reader's real EOF.
#[cfg(not(unix))]
impl KeyReader for TtyKeys {
    fn read_key(&mut self) -> Key {
        Key::Eof
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scripted key source: each call pops the next key; an empty
    /// script is end-of-input.
    struct Script(Vec<Key>);
    impl KeyReader for Script {
        fn read_key(&mut self) -> Key {
            if self.0.is_empty() {
                Key::Eof
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

    fn run(keys: Vec<Key>) -> (Picked, String) {
        let mut out: Vec<u8> = Vec::new();
        let picked = pick(&rows(), &mut Script(keys), &mut out);
        (picked, String::from_utf8(out).expect("utf8"))
    }

    #[test]
    fn enter_immediately_shows_the_newest() {
        let (picked, out) = run(vec![Key::Enter]);
        assert_eq!(picked, Picked::Show(4));
        assert!(out.contains('▸'), "the newest preselected: {out}");
        assert!(out.contains("seq 4"), "{out}");
    }

    #[test]
    fn arrows_navigate_and_clamp() {
        let (picked, _) = run(vec![Key::Down, Key::Down, Key::Enter]);
        assert_eq!(picked, Picked::Show(2), "the bottom row clamps");
        let (picked, _) = run(vec![Key::Up, Key::Enter]);
        assert_eq!(picked, Picked::Show(4), "the top row clamps");
        let (picked, _) = run(vec![Key::Char('j'), Key::Char('k'), Key::Enter]);
        assert_eq!(picked, Picked::Show(4), "j/k mirror the arrows");
    }

    #[test]
    fn the_marker_moves_with_the_selection() {
        let (_, out) = run(vec![Key::Down, Key::Enter]);
        // The redraw escape skips prev_lines (header + 2 rows) upward; the
        // last frame is everything after that prefix.
        let last = out.rsplit("\x1b[3A").next().unwrap_or("");
        assert!(last.contains("▸ \x1b[1mseq 2"), "last frame: {last}");
        assert!(
            !last.contains("▸ \x1b[1mseq 4"),
            "the old marker is gone: {last}"
        );
    }

    #[test]
    fn void_arms_then_confirms_by_enter_or_y() {
        let (picked, out) = run(vec![Key::Char('v'), Key::Enter]);
        assert_eq!(picked, Picked::Void(4));
        assert!(out.contains("void seq 4?"), "the confirm line drew: {out}");
        let (picked, _) = run(vec![Key::Char('v'), Key::Char('y')]);
        assert_eq!(picked, Picked::Void(4));
    }

    #[test]
    fn any_other_key_disarms_a_void() {
        let (picked, _) = run(vec![Key::Char('v'), Key::Char('n'), Key::Enter]);
        assert_eq!(picked, Picked::Show(4));
        let (picked, _) = run(vec![Key::Char('v'), Key::Esc, Key::Enter]);
        assert_eq!(picked, Picked::Show(4));
        // An arrow away disarms too.
        let (picked, _) = run(vec![Key::Char('v'), Key::Down, Key::Enter]);
        assert_eq!(picked, Picked::Show(2));
    }

    #[test]
    fn quit_ctrl_c_and_eof_cancel() {
        let (picked, _) = run(vec![Key::Char('q')]);
        assert_eq!(picked, Picked::Cancelled);
        let (picked, _) = run(vec![Key::CtrlC]);
        assert_eq!(picked, Picked::Cancelled);
        let (picked, _) = run(vec![]);
        assert_eq!(picked, Picked::Cancelled);
    }

    #[test]
    fn the_first_key_window_timing_out_is_timed_out() {
        let mut out: Vec<u8> = Vec::new();
        let picked = pick(&rows(), &mut Script(vec![Key::Timeout]), &mut out);
        assert_eq!(picked, Picked::TimedOut);
        // The frame was still drawn (the operator can see what WOULD have
        // been picked) — the caller adds the non-interactive hint.
        assert!(String::from_utf8(out).expect("utf8").contains('▸'));
    }

    #[test]
    fn empty_rows_cancel_without_drawing() {
        let mut out: Vec<u8> = Vec::new();
        let picked = pick(&[], &mut Script(vec![Key::Enter]), &mut out);
        assert_eq!(picked, Picked::Cancelled);
        assert!(out.is_empty());
    }
}

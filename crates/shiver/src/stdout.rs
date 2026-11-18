//! Printing a line in the terminal as a speech box.

use std::io::{IsTerminal, Write};

use shiver_core::Interjection;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::output::Output;

/// Maximum width of the text inside the box, in display columns.
const WRAP: usize = 60;

pub struct Stdout {
    color: bool,
}

impl Stdout {
    /// Colour is on when stdout is a terminal and `NO_COLOR` is unset or empty.
    pub fn detect() -> Self {
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
        Stdout {
            color: std::io::stdout().is_terminal() && !no_color,
        }
    }
}

impl Output for Stdout {
    fn emit(&self, interjection: &Interjection) -> anyhow::Result<()> {
        let mut out = std::io::stdout().lock();
        out.write_all(render(interjection, self.color).as_bytes())?;
        out.flush()?;
        Ok(())
    }
}

/// Renders the voice name and a box around the wrapped text.
fn render(i: &Interjection, color: bool) -> String {
    let name = if color {
        let style = match i.voice.color.as_deref().and_then(rgb) {
            Some((r, g, b)) => format!("\x1b[1;38;2;{r};{g};{b}m"),
            None => "\x1b[1m".to_owned(),
        };
        format!("{style}{}\x1b[0m", i.voice.display)
    } else {
        i.voice.display.clone()
    };

    let lines = wrap(&i.text, WRAP);
    let width = lines.iter().map(|l| l.width()).max().unwrap_or(0);
    let rule = "─".repeat(width + 2);

    let mut s = format!(" {name}\n╭{rule}╮\n");
    for line in &lines {
        let pad = " ".repeat(width - line.width());
        s.push_str(&format!("│ {line}{pad} │\n"));
    }
    s.push_str(&format!("╰{rule}╯\n"));
    s
}

/// Word-wraps `text` to at most `max` display columns per line, keeping line
/// breaks and splitting words that are too long on their own.
fn wrap(text: &str, max: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            for piece in split_word(word, max) {
                let needed = if line.is_empty() {
                    piece.width()
                } else {
                    line.width() + 1 + piece.width()
                };
                if needed > max {
                    lines.push(std::mem::take(&mut line));
                }
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(&piece);
            }
        }
        lines.push(line);
    }
    lines
}

/// Splits a word into pieces no wider than `max` columns.
fn split_word(word: &str, max: usize) -> Vec<String> {
    if word.width() <= max {
        return vec![word.to_owned()];
    }
    let mut pieces = Vec::new();
    let mut piece = String::new();
    let mut width = 0;
    for c in word.chars() {
        let w = c.width().unwrap_or(0);
        if width + w > max && !piece.is_empty() {
            pieces.push(std::mem::take(&mut piece));
            width = 0;
        }
        piece.push(c);
        width += w;
    }
    pieces.push(piece);
    pieces
}

/// Parses `#rrggbb`.
fn rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.strip_prefix('#')?;
    let channel = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some((channel(0)?, channel(2)?, channel(4)?))
}

#[cfg(test)]
mod tests {
    use shiver_core::Voice;

    use super::*;

    fn line(text: &str, color: Option<&str>) -> Interjection {
        Interjection {
            voice: Voice {
                key: "drift".into(),
                display: "DRIFT".into(),
                color: color.map(Into::into),
                sound: None,
                icon: None,
            },
            text: text.into(),
            sound: None,
            icon: None,
        }
    }

    #[test]
    fn renders_box() {
        assert_eq!(
            render(&line("Tea's ready.", None), false),
            " DRIFT\n\
             ╭──────────────╮\n\
             │ Tea's ready. │\n\
             ╰──────────────╯\n"
        );
    }

    #[test]
    fn wraps_at_60_columns() {
        let text = "word ".repeat(30);
        let lines = wrap(&text, WRAP);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|l| l.width() <= WRAP), "{lines:?}");
        // "word" is 4 columns; 12 words + 11 spaces = 59 fits, 13 would be 64.
        assert_eq!(lines[0], "word ".repeat(12).trim_end());
        assert_eq!(lines.join(" "), text.trim_end());
    }

    #[test]
    fn splits_long_words() {
        let word = "x".repeat(130);
        assert_eq!(
            wrap(&format!("a {word}"), WRAP),
            ["a", &"x".repeat(60), &"x".repeat(60), &"x".repeat(10)]
        );
    }

    #[test]
    fn keeps_line_breaks() {
        assert_eq!(wrap("one\ntwo\n\nthree", WRAP), ["one", "two", "", "three"]);
        let out = render(&line("hi\nthere", None), false);
        assert!(out.contains("│ hi    │\n│ there │\n"), "{out}");
    }

    #[test]
    fn measures_wide_characters() {
        // Each of these characters is 2 columns wide.
        let out = render(&line("お茶", None), false);
        assert!(out.contains("╭──────╮\n│ お茶 │\n"), "{out}");
        let lines = wrap(&"茶".repeat(40), WRAP);
        assert_eq!(lines, ["茶".repeat(30), "茶".repeat(10)]);
    }

    #[test]
    fn colours_name_only_when_enabled() {
        let plain = render(&line("Hi.", Some("#8a5cd6")), false);
        assert!(!plain.contains('\x1b'));

        let coloured = render(&line("Hi.", Some("#8a5cd6")), true);
        assert!(coloured.starts_with(" \x1b[1;38;2;138;92;214mDRIFT\x1b[0m\n"));
        assert_eq!(
            coloured.matches('\x1b').count(),
            2,
            "only the name is styled"
        );

        let bold = render(&line("Hi.", None), true);
        assert!(bold.starts_with(" \x1b[1mDRIFT\x1b[0m\n"));
    }
}

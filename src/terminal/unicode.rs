use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const ZERO_WIDTH_JOINER: &str = "\u{200d}";
const VARIATION_SELECTOR_16: &str = "\u{fe0f}";
const SKIN_TONES: [&str; 5] = [
    "\u{1f3fb}", // Light Skin Tone
    "\u{1f3fc}", // Medium-Light Skin Tone
    "\u{1f3fd}", // Medium Skin Tone
    "\u{1f3fe}", // Medium-Dark Skin Tone
    "\u{1f3ff}", // Dark Skin Tone
];

// Return String display width as rendered in a monospace font according to the Unicode
// specification.
//
// This may return some odd results at times where some symbols are counted as more character width
// than they actually are.
//
// This function has exceptions for skin tones and other emoji modifiers to determine a more
// accurate display with.
pub fn display_width(string: &str) -> usize {
    // String expressed as a vec of Unicode characters. Characters with accents and emoji may
    // be multiple characters combined.
    let mut graphemes = string.graphemes(true);
    let mut width = 0;
    while let Some(grapheme) = graphemes.next() {
        if skip_ansi_escape_sequence(grapheme, &mut graphemes) {
            continue;
        }
        width += display_width_char(grapheme);
    }
    width
}

/// Calculate the render width of a single Unicode character. Unicode characters may consist of
/// multiple String characters, which is why the function argument takes a string.
fn display_width_char(string: &str) -> usize {
    // Characters that are used as modifiers on emoji. By themselves they have no width.
    if string == ZERO_WIDTH_JOINER || string == VARIATION_SELECTOR_16 {
        return 0;
    }
    // Emoji that are representations of combined emoji. They are normally calculated as the
    // combined width of the emoji, rather than the actual display width. This check fixes that and
    // returns a width of 2 instead.
    if string.contains(ZERO_WIDTH_JOINER) {
        return 2;
    }
    // Any character with a skin tone is most likely an emoji.
    // Normally it would be counted as as four or more characters, but these emoji should be
    // rendered as having a width of two.
    for skin_tone in SKIN_TONES {
        if string.contains(skin_tone) {
            return 2;
        }
    }

    match string {
        _ => UnicodeWidthStr::width(string),
    }
}

/// The CSI or “Control Sequence Introducer” introduces an ANSI escape
/// sequence. This is typically used for colored text and will be
/// ignored when computing the text width.
const CSI: (&str, &str) = ("\x1b", "[");
/// The final bytes of an ANSI escape sequence must be in this range.
const ANSI_FINAL_BYTE: std::ops::RangeInclusive<char> = '\x40'..='\x7e';

/// Skip ANSI escape sequences.
///
/// The `ch` is the current `char`, the `chars` provide the following
/// characters. The `chars` will be modified if `ch` is the start of
/// an ANSI escape sequence.
///
/// Returns `true` if one or more chars were skipped.
fn skip_ansi_escape_sequence<'a, I: Iterator<Item = &'a str>>(str: &str, iter: &mut I) -> bool {
    if str != CSI.0 {
        return false; // Nothing to skip here.
    }

    let next = iter.next();
    if next == Some(CSI.1) {
        // We have found the start of an ANSI escape code, typically
        // used for colored terminal text. We skip until we find a
        // "final byte" in the range 0x40–0x7E.
        for str in iter {
            if let Some(ch) = str.chars().next() {
                if ANSI_FINAL_BYTE.contains(&ch) {
                    break;
                }
            }
        }
    } else if next == Some("]") {
        // We have found the start of an Operating System Command,
        // which extends until the next sequence "\x1b\\" (the String
        // Terminator sequence) or the BEL character. The BEL
        // character is non-standard, but it is still used quite
        // often, for example, by GNU ls.
        let mut last = "]";
        for new_str in iter {
            if new_str == "\x07" || (new_str == "\\" && last == CSI.0) {
                break;
            }
            last = new_str;
        }
    }

    true // Indicate that some chars were skipped.
}

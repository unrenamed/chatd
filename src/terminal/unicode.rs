use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Zero Width Joiner (ZWJ) is a Unicode character that joins two or
/// more other characters together in sequence to create a new emoji.
const ZERO_WIDTH_JOINER: &str = "\u{200d}";

/// An invisible codepoint which specifies that the preceding
/// character should be displayed with emoji presentation. Only
/// required if the preceding character defaults to text presentation.
const VARIATION_SELECTOR_16: &str = "\u{fe0f}";

/// Some emojis can be modified by using one of five different skin
/// tones:
const SKIN_TONES: [&str; 5] = [
    "\u{1f3fb}", // Light Skin Tone
    "\u{1f3fc}", // Medium-Light Skin Tone
    "\u{1f3fd}", // Medium Skin Tone
    "\u{1f3fe}", // Medium-Dark Skin Tone
    "\u{1f3ff}", // Dark Skin Tone
];

/// The CSI or “Control Sequence Introducer” introduces an ANSI escape
/// sequence. This is typically used for colored text and will be
/// ignored when computing the text width.
const CSI: (&str, &str) = ("\x1b", "[");

/// The final bytes of an ANSI escape sequence must be in this range.
const ANSI_FINAL_BYTE: std::ops::RangeInclusive<char> = '\x40'..='\x7e';

/// Compute the display width of `text` while skipping over ANSI
/// escape sequences.
///
/// # Examples
///
/// ```
/// assert_eq!(display_width("Café Plain"), 10);
/// assert_eq!(display_width("\u{1b}[31mCafé Rouge\u{1b}[0m"), 10);
/// assert_eq!(display_width("\x1b]8;;http://example.com\x1b\\This is a link\x1b]8;;\x1b\\"), 14);
/// ```
///
/// ## Emojis and CJK Characters
///
/// Characters such as emojis and [CJK characters] used in the
/// Chinese, Japanese, and Korean languages are seen as double-width.
///
/// ```
/// assert_eq!(display_width("😂😭🥺🤣✨😍🙏🥰😊🔥"), 20);
/// assert_eq!(display_width("你好"), 4);  // “Nǐ hǎo” or “Hello” in Chinese
/// ```
///
/// ## Emojis Skin Tones
///
/// Skin tones and other emoji modifiers although add more bytes to
/// the string are still seen as double-width.
///
/// ```
/// assert_width("👩", 2);
/// assert_width("👩🏻", 2);
/// assert_width("👩🏼", 2);
/// assert_width("👩🏽", 2);
/// assert_width("👩🏾", 2);
/// assert_width("👩🏿", 2);
/// ```
///
/// # Limitations
///
/// The displayed width of a string cannot always be computed from the
/// string alone. This is because the width depends on the rendering
/// engine used. This is particularly visible with [emoji modifier
/// sequences] where a base emoji is modified with, e.g., skin tone or
/// hair color modifiers. It is up to the rendering engine to detect
/// this and to produce a suitable emoji.
///
/// That's why this function has exceptions for skin tones and other
/// emoji modifiers to determine as much accurate display width as
/// needed for the app use case.
pub fn display_width(text: &str) -> usize {
    // String expressed as a vec of Unicode characters.
    // Characters with accents and emoji may be multiple
    // characters combined.
    let mut graphemes = text.graphemes(true);
    let mut width = 0;
    while let Some(grapheme) = graphemes.next() {
        if skip_ansi_escape_sequence(grapheme, &mut graphemes) {
            continue;
        }
        width += display_width_char(grapheme);
    }
    width
}

/// Calculate the render width of a single Unicode character. Unicode
/// characters may consist of multiple String characters, which is why
/// the function argument takes a string.
fn display_width_char(str: &str) -> usize {
    // Characters that are used as modifiers on emoji. By themselves they
    // have no width.
    if is_emoji_modifier(str) {
        return 0;
    }

    // Emoji that are representations of combined emoji. They are normally
    // calculated as the combined width of the emoji, rather than the
    // actual display width. This check fixes that and returns a width of
    // 2 instead.
    if is_combined_emoji(str) {
        return 2;
    }

    // Any character with a skin tone is most likely an emoji. Normally it
    // would be counted as as four or more characters, but these emoji
    // should be rendered as having a width of two.
    if contains_skin_tone(str) {
        return 2;
    }

    // Any character followed by U+FE0F (Variation Selector-16) modifier
    // may be either 2 width emoji or 1 width character. By itself this
    // modifier has no width and should not affect the unicode width
    // calculation. So, we remove all selectors from the string slice.
    let cleaned_str = remove_variation_selector(str);
    UnicodeWidthStr::width(cleaned_str.as_str())
}

/// Skip ANSI escape sequences. The `str` is the current `str`, the
/// `iter` provide the following characters. The `iter` will be
/// modified if `str` is the start of an ANSI escape sequence. Returns
/// `true` if one or more chars were skipped.
fn skip_ansi_escape_sequence<'a, I: Iterator<Item = &'a str>>(str: &str, iter: &mut I) -> bool {
    if str != CSI.0 {
        return false; // Nothing to skip here.
    }

    let next = iter.next();
    if next == Some(CSI.1) {
        // We have found the start of an ANSI escape code, typically used for
        // colored terminal text. We skip until we find a "final byte" in the
        // range 0x40–0x7E.
        'outer: for str in iter {
            let mut chars = str.chars();
            while let Some(ch) = chars.next() {
                if ANSI_FINAL_BYTE.contains(&ch) {
                    break 'outer;
                }
            }
        }
    } else if next == Some("]") {
        // We have found the start of an Operating System Command, which
        // extends until the next sequence "\x1b\\" (the String Terminator
        // sequence) or the BEL character. The BEL character is non-standard,
        // but it is still used quite often, for example, by GNU ls.
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

fn is_emoji_modifier(str: &str) -> bool {
    str == ZERO_WIDTH_JOINER || str == VARIATION_SELECTOR_16
}

fn is_combined_emoji(str: &str) -> bool {
    str.contains(ZERO_WIDTH_JOINER)
}

fn contains_skin_tone(str: &str) -> bool {
    SKIN_TONES.iter().any(|&skin_tone| str.contains(skin_tone))
}

fn remove_variation_selector(str: &str) -> String {
    if str.contains(VARIATION_SELECTOR_16) {
        remove_substring(str, VARIATION_SELECTOR_16)
    } else {
        str.to_string()
    }
}

fn remove_substring(s: &str, to_remove: &str) -> String {
    s.split(to_remove).collect::<Vec<&str>>().join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_ansi_escape_sequence() {
        let blue_text = "\u{1b}[34mHello\u{1b}[0m";
        let mut graphemes = blue_text.graphemes(true);
        let grapheme = graphemes.next().unwrap();
        assert!(skip_ansi_escape_sequence(grapheme, &mut graphemes));
        assert_eq!(graphemes.next(), Some("H"));
    }

    fn assert_width(string: &str, width: usize) {
        assert_eq!(
            display_width(string),
            width,
            "String `{}` is not {} width",
            string,
            width
        );
    }

    #[test]
    fn test_display_width() {
        assert_width("a", 1);
        assert_width("…", 1);

        assert_width("é", 1);
        assert_width("ö", 1);
        assert_width("ø", 1);
        assert_width("a̐", 1);
        assert_width("é", 1);
        assert_width("ö̲", 1);

        assert_width("ぁ", 2);
        assert_width("あ", 2);

        // Zero width characters
        assert_width("\u{200d}", 0);
        assert_width("\u{fe0f}", 0);
        assert_width("⁉\u{fe0f}", 1);

        // Some of these characters don't match the width one would expect.
        // Most of these are displayed as 2 width in most modern terminals,
        // but unicode-width returns as the width according to the Unicode
        // specification, which may sometimes be different than the actual
        // display width.

        // Some of these the assertions below do not return the width
        // according to unicode-width. The `display_width` function will
        // check for things like skin tones and other emoji modifiers to
        // return a differen display width.
        assert_width("0️⃣", 1);
        assert_width("1️⃣", 1);
        assert_width("#️⃣", 1);
        assert_width("﹟", 2);
        assert_width("＃", 2);
        assert_width("*️⃣", 1);
        assert_width("＊", 2);
        assert_width("❗️", 2);
        assert_width("☁️", 1);
        assert_width("❤️", 1);
        assert_width("☂️", 1);
        assert_width("✏️", 1);
        assert_width("✂️", 1);
        assert_width("☎️", 1);
        assert_width("✈️", 1);
        assert_width("⁉", 1);
        assert_width("👁", 1); // Eye without variable selector 16
        assert_width("👁️", 1); // Eye + variable selector 16 `\u{fe0f}`
        assert_width("👁️‍🗨️", 2);
        assert_width("🚀", 2);

        // Skin tones
        assert_width("👩", 2);
        assert_width("👩🏻", 2);
        assert_width("👩🏼", 2);
        assert_width("👩🏽", 2);
        assert_width("👩🏾", 2);
        assert_width("👩🏿", 2);

        // Other variations
        assert_width("👩‍🔬", 2);
        assert_width("🧘🏽‍♀️", 2);
        assert_width("👨🏻‍❤️‍👨🏿", 2);
        assert_width("🧑‍🦲", 2);
        assert_width("👨🏿‍🦲", 2);

        // Strings with multiple characters
        assert_width("abc", 3);
        assert_width(&"a".repeat(50), 50);
        assert_width("!*_-=+|[]`'.,<>():;!@#$%^&{}10/", 31);
        assert_width("I am a string with multiple 😁🚀あ", 34);
        assert_width("👩‍🔬👩‍🔬", 4);
        assert_width("😂😭🥺🤣✨😍🙏🥰😊🔥", 20);
        assert_width("👩‍👩‍👦👨‍👨‍👦👨‍👨‍👧‍👧👩‍🏭👯🏻‍♂️👩🏻‍🔬🕵🏿‍♂️👨‍💻🏄‍♂️👩‍🚒", 20);

        // Strings with ANSI escape sequences
        assert_eq!(display_width("\u{1b}[31mCafé Rouge\u{1b}[0m"), 10);
        assert_eq!(
            display_width("\x1b]8;;http://example.com\x1b\\This is a link\x1b]8;;\x1b\\"),
            14
        );
    }

    #[test]
    fn test_emojis_have_correct_width() {
        use unic_emoji_char::is_emoji;

        // Emojis in the Basic Latin (ASCII) and Latin-1 Supplement  blocks
        // all have a width of 1 column. This includes  characters such as '#'
        // and '©'.
        for ch in '\u{1}'..'\u{FF}' {
            if is_emoji(ch) {
                let desc = format!("{:?} U+{:04X}", ch, ch as u32);
                assert_eq!(display_width_char(&ch.to_string()), 1, "char: {}", desc);
            }
        }

        // Emojis in the remaining blocks of the Basic Multilingual Plane
        // (BMP), in the Supplementary Multilingual Plane (SMP), and in the
        // Supplementary Ideographic Plane (SIP), are all 1 (narrow emojis) or
        // 2 (single emojis or an emoji ZWJ sequence) columns wide. This
        // includes all of our favorite emojis such as 😊 and 👨‍💻.
        for ch in '\u{FF}'..'\u{2FFFF}' {
            if is_emoji(ch) {
                let desc = format!("{:?} U+{:04X}", ch, ch as u32);
                assert!(display_width_char(&ch.to_string()) <= 2, "char: {}", desc);
            }
        }

        // The remaining planes contain almost no assigned code points
        // and thus also no emojis.
    }
}

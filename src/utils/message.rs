use ratatui::{
    style::{Color, Style},
    text::Span,
};

pub fn split_by_indices<'a>(text: &'a str, indices: &[usize], substr_len: usize) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let mut prev_index = 0;

    for &index in indices {
        if index >= text.len() {
            break;
        }

        if prev_index < index {
            spans.push(Span::styled(&text[prev_index..index], Style::default()));
        }

        if index + substr_len <= text.len() {
            spans.push(Span::styled(
                &text[index..index + substr_len],
                Style::default().bg(Color::Rgb(255, 140, 0)),
            ));
            prev_index = index + substr_len;
        }
    }

    if prev_index < text.len() {
        spans.push(Span::styled(&text[prev_index..], Style::default()));
    }

    spans
}

/// Layout engine — flow-based card grid.

use crate::theme;

/// A positioned card slot.
#[derive(Clone, Debug)]
pub struct CardSlot {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Calculate card grid layout for a given screen area.
/// Returns card slots arranged in a flow-based grid.
pub fn card_grid(
    screen_width: u32,
    content_top: u32,
    content_bottom: u32,
    num_cards: usize,
) -> Vec<CardSlot> {
    if num_cards == 0 {
        return vec![];
    }

    let margin = theme::CONTENT_MARGIN as f32;
    let gap = theme::CARD_GAP as f32;
    let available_width = screen_width as f32 - margin * 2.0;

    // Calculate number of columns (min card width 350px)
    let min_card = theme::CARD_MIN_WIDTH as f32;
    let cols = ((available_width + gap) / (min_card + gap)).floor().max(1.0) as usize;
    let card_w = (available_width - (cols as f32 - 1.0) * gap) / cols as f32;

    let content_height = content_bottom as f32 - content_top as f32;
    let rows = (num_cards + cols - 1) / cols;
    let card_h = ((content_height - (rows as f32 + 1.0) * gap) / rows as f32).max(100.0).min(250.0);

    let mut slots = Vec::new();
    for i in 0..num_cards {
        let col = i % cols;
        let row = i / cols;
        let x = margin + col as f32 * (card_w + gap);
        let y = content_top as f32 + gap + row as f32 * (card_h + gap);
        slots.push(CardSlot {
            x,
            y,
            w: card_w,
            h: card_h,
        });
    }
    slots
}

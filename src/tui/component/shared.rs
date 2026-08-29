mod button;
pub use button::Button;

mod input_block;
pub use input_block::InputBlock;

mod single_selector;
pub use single_selector::SingleSelector;
pub use single_selector::SingleSelectorItem;

mod colored_tags;
pub use colored_tags::ColoredTags;

pub fn truncate_string_to_width(s: String, max_width: usize) -> String {
    if s.chars().count() <= max_width {
        s
    } else {
        // cut to make room for the ellipsis
        let mut out: String = s.chars().take(max_width.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

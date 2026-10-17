// `Look` is `#[non_exhaustive]`; see its documentation.
use leptos_rich_chat::{Look, Position};

fn main() {
    let _look = Look {
        position: Position::Left,
        background: None,
        foreground: None,
        properties: Vec::new(),
    };
}

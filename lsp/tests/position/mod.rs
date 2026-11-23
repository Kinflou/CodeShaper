// Standard Uses

// Crate Uses

// External Uses
use codeshaper_lsp::position::LineIndex;
use lsp_types::Position;


#[test]
fn offsets_and_positions_use_utf16() {
    let text = "first\nnäme \"😀x\"\nlast";
    let index = LineIndex::new(text);
    let x = text.find('x').unwrap();

    assert_eq!(index.position(0), Position::new(0, 0));
    assert_eq!(index.position(text.find("last").unwrap()), Position::new(2, 0));
    assert_eq!(index.position(x), Position::new(1, 8));
    assert_eq!(index.offset(Position::new(1, 8)), x);
    assert_eq!(index.offset(Position::new(1, 99)), text.find("\nlast").unwrap());
    assert_eq!(index.offset(Position::new(9, 0)), text.len());
}

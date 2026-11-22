#![doc(
html_logo_url =
"https://raw.githubusercontent.com/kinflou/CodeShaperDocs/main/_static/img/logos/codeshaper/logo-slogan.png",

html_favicon_url =
"https://raw.githubusercontent.com/kinflou/CodeShaperDocs/main/_static/img/logos/codeshaper/logo-only.png"
)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README-DOCS.md")]


// Relative Modules

// Standard Uses

// Crate Uses

// External Uses
pub use codeshaper_core as core;
pub use codeshaper_core::registry::Registry;


pub fn registry() -> Registry {
    #[allow(unused_mut)]
    let mut registry = Registry::default();

    #[cfg(any(feature = "cpp", feature = "gdscript"))]
    codeshaper_nav_ast_treesitter::register(&mut registry);

    #[cfg(feature = "godot")]
    codeshaper_target_godot::register(&mut registry);

    #[cfg(feature = "text")]
    codeshaper_target_text::register(&mut registry);

    registry
}

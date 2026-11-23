// Standard Uses
use std::borrow::Cow;

// Crate Uses

// External Uses


pub fn unix(text: &str) -> Cow<'_, str> {
    if text.contains("\r\n") { Cow::Owned(text.replace("\r\n", "\n")) } else { Cow::Borrowed(text) }
}

pub fn restore(original: &str, result: &str) -> String {
    let windows = original.matches("\r\n").count();
    let lines = original.matches('\n').count();
    let result = unix(result);

    if windows * 2 > lines { result.replace('\n', "\r\n") } else { result.into_owned() }
}

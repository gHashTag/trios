use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() {
    web_sys::console::log_1(&"trios-ext loaded".into());
}

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Trios says: Hello, {}!", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greet_returns_message() {
        let result = greet("Agent");
        assert!(result.contains("Agent"));
    }
}

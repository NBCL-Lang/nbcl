#[cfg(feature = "wasm")]
mod wasm {
    use wasm_bindgen::prelude::*;
    use std::cell::RefCell;

    thread_local! {
        static PRINT_BUFFER: RefCell<Vec<String>> = RefCell::new(Vec::new());
    }

    /// Print something to wasm buffer
    pub fn wasm_print(msg: String) {
        PRINT_BUFFER.with(|buf| buf.borrow_mut().push(msg));
    }

    #[wasm_bindgen]
    pub fn run(source: &str) -> String {
        PRINT_BUFFER.with(|buf| buf.borrow_mut().clear());

        let engine = crate::engine::NbclEngine::new();
        match engine.parse_str(source) {
            Ok(result) => {
                match engine.evaluate(result) {
                    Ok(value) => {
                        let output = PRINT_BUFFER.with(|buf| buf.borrow().join("\n"));
                        format!(
                            "{{\"ok\": true, \"output\": {}, \"result\": {}}}",
                            serde_json::to_string(&output).unwrap_or_default(),
                            serde_json::to_string(&value).unwrap_or_default()
                        )
                    }
                    Err(e) => format!("{{\"ok\": false, \"error\": {}}}", 
                        serde_json::to_string(&e.to_string()).unwrap_or_default()),
                }
            }
            Err(e) => format!("{{\"ok\": false, \"error\": {}}}", 
                serde_json::to_string(&e.to_string()).unwrap_or_default()),
        }
    }
}

#[cfg(feature = "wasm")]
pub use wasm::*;
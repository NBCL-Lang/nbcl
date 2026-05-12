//! API's for WebAssembly (available only with `wasm` feature)
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

thread_local! {
    static PRINT_BUFFER: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

#[derive(Serialize, Deserialize)]
pub struct WasmConfig {
    max_depth: usize
}

/// Print something to wasm buffer
pub fn wasm_print(msg: String) {
    PRINT_BUFFER.with(|buf| buf.borrow_mut().push(msg));
}

#[wasm_bindgen]
pub fn run(source: &str) -> String {
    let config = r#"{
        "max_depth": 5,
    }"#;
    run_with_config(source, config)
}

#[wasm_bindgen]
pub fn run_with_config(source: &str, config: &str) -> String {
    PRINT_BUFFER.with(|buf| buf.borrow_mut().clear());
    let Ok(wasm_config) = serde_json::from_str::<WasmConfig>(config) else {
        return "invalid configuration was passed".to_string()
    };
    let mut engine = crate::engine::NbclEngine::new();

    // == Apply Config ==
    engine.set_max_depth(wasm_config.max_depth);

    // == Evaluate ==
    match engine.parse_str(source) {
        Ok(result) => match engine.evaluate(result) {
            Ok(value) => {
                let output = PRINT_BUFFER.with(|buf| buf.borrow().join("\n"));
                format!(
                    "{{\"ok\": true, \"output\": {}, \"result\": {}}}",
                    serde_json::to_string(&output).unwrap_or_default(),
                    serde_json::to_string(&value).unwrap_or_default()
                )
            }
            Err(e) => format!(
                "{{\"ok\": false, \"error\": {}}}",
                serde_json::to_string(&e.to_string()).unwrap_or_default()
            ),
        },
        Err(e) => format!(
            "{{\"ok\": false, \"error\": {}}}",
            serde_json::to_string(&e.to_string()).unwrap_or_default()
        ),
    }
}

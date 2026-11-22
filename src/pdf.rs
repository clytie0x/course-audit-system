//! PDF.js interop: text extraction runs in JavaScript (see `index.html`).

use wasm_bindgen::prelude::*;

/// JavaScript interop function exposed by the PDF extractor in the frontend runtime.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = extractTextFromPDF)]
    pub fn extract_text_from_pdf(array_buffer: js_sys::Uint8Array) -> js_sys::Promise;
}

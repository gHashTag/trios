// GitHub content injector bootstrap (no-modules)
// trios_ext.js is loaded first and defines global wasm_bindgen
(async () => {
  try {
    const wasmUrl = chrome.runtime.getURL("dist/trios_ext_bg.wasm");
    await wasm_bindgen(wasmUrl);
    wasm_bindgen.github_injector_start();
    console.log("[trios-github] Content script initialized");
  } catch (err) {
    console.error("[trios-github] Init failed:", err);
  }
})();

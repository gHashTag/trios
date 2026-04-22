// Sidepanel bootstrap (no-modules)
// trios_ext.js is loaded via <script> tag and defines global wasm_bindgen
(async () => {
  try {
    await wasm_bindgen();
    console.log("[trios-sidepanel] WASM initialized");
  } catch (err) {
    console.error("[trios-sidepanel] WASM init failed:", err);
    const root = document.getElementById("root");
    if (root) {
      root.innerHTML = `<pre style="color:#FF6B6B;padding:16px">
⚠ WASM Init Failed
${String(err?.stack || err)}
      </pre>`;
    }
  }
})();

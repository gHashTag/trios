// Bootstrap for Trinity Agent Bridge sidepanel
// Fetches WASM as ArrayBuffer, passes via object syntax to avoid
// both instantiateStreaming CSP issues and wasm-bindgen URL object bug
import init from "./trios_ext.js";

(async () => {
  try {
    const url = new URL("./trios_ext_bg.wasm", import.meta.url);
    const resp = await fetch(url);
    if (!resp.ok) throw new Error(`fetch wasm: ${resp.status}`);
    const bytes = await resp.arrayBuffer();
    await init({ module_or_path: bytes });
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

// Trinity Agent Bridge — Background Service Worker (NO WASM)
// All MCP/UI logic lives in the sidepanel via WASM.
// Content scripts are declared in manifest.json (no-modules).
// API key storage: chrome.storage.local (keys never leave extension).

chrome.runtime.onInstalled.addListener(() => {
  console.log("[trios-bg] Extension installed/updated");
  if (chrome.sidePanel?.setPanelBehavior) {
    chrome.sidePanel.setPanelBehavior({ openPanelOnActionClick: true }).catch(() => {});
  }
});

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg === "ping" || msg?.type === "ping") {
    sendResponse({ pong: true, ts: Date.now() });
    return false;
  }

  // Settings access: sidepanel requests stored keys
  if (msg?.type === "get_settings") {
    var keys = msg.keys || ["zai_api_key", "zai_base_url", "mcp_server_url"];
    chrome.storage.local.get(keys, function (result) {
      if (chrome.runtime.lastError) {
        sendResponse({ error: chrome.runtime.lastError.message });
      } else {
        sendResponse({ settings: result });
      }
    });
    return true; // async response
  }

  return false;
});

// Auto-activate new service workers
self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (e) => e.waitUntil(self.clients.claim()));

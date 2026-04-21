// Trinity Agent Bridge — Bare Service Worker (NO WASM)
// All MCP/UI logic lives in the sidepanel via WASM.
// This file handles only Chrome lifecycle events.

chrome.runtime.onInstalled.addListener(() => {
  console.log("[trios-bg] Extension installed/updated");
  // Enable sidePanel open on action click
  if (chrome.sidePanel?.setPanelBehavior) {
    chrome.sidePanel.setPanelBehavior({ openPanelOnActionClick: true }).catch(() => {});
  }
});

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg === "ping" || msg?.type === "ping") {
    sendResponse({ pong: true, ts: Date.now() });
  }
  return false;
});

// Auto-activate new service workers
self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (e) => e.waitUntil(self.clients.claim()));

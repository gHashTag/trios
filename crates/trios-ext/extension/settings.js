// Trinity Agent Bridge — Settings Popup
// Reads/writes API keys via chrome.storage.local
// No keys ever leave the extension boundary.

(function () {
  "use strict";

  var ZAI_KEY = "zai_api_key";
  var ZAI_BASE = "zai_base_url";
  var MCP_URL = "mcp_server_url";

  var els = {
    zaiKey: document.getElementById("zai-key"),
    zaiBase: document.getElementById("zai-base"),
    mcpUrl: document.getElementById("mcp-url"),
    toggleZai: document.getElementById("toggle-zai"),
    zaiStatus: document.getElementById("zai-status"),
    zaiBaseStatus: document.getElementById("zai-base-status"),
    mcpStatus: document.getElementById("mcp-status"),
  };

  function showStatus(el, msg, isError) {
    el.textContent = msg;
    el.className = "status " + (isError ? "error" : "saved");
    setTimeout(function () {
      el.textContent = "";
    }, 3000);
  }

  function save(key, value, statusEl) {
    var obj = {};
    obj[key] = value;
    chrome.storage.local.set(obj, function () {
      if (chrome.runtime.lastError) {
        showStatus(statusEl, "Error: " + chrome.runtime.lastError.message, true);
      } else {
        var masked = value ? value.slice(0, 6) + "..." + value.slice(-4) : "(empty)";
        showStatus(statusEl, "Saved " + masked, false);
      }
    });
  }

  function load(keys, cb) {
    chrome.storage.local.get(keys, function (result) {
      if (chrome.runtime.lastError) {
        console.error("[settings] load error:", chrome.runtime.lastError);
      }
      cb(result || {});
    });
  }

  // Load existing values
  load([ZAI_KEY, ZAI_BASE, MCP_URL], function (data) {
    if (data[ZAI_KEY]) els.zaiKey.value = data[ZAI_KEY];
    if (data[ZAI_BASE]) els.zaiBase.value = data[ZAI_BASE];
    if (data[MCP_URL]) els.mcpUrl.value = data[MCP_URL];
  });

  // Auto-save on input change (debounced)
  var timers = {};
  function debounce(id, fn, ms) {
    if (timers[id]) clearTimeout(timers[id]);
    timers[id] = setTimeout(fn, ms);
  }

  els.zaiKey.addEventListener("input", function () {
    debounce("zai", function () {
      save(ZAI_KEY, els.zaiKey.value.trim(), els.zaiStatus);
    }, 600);
  });

  els.zaiBase.addEventListener("input", function () {
    debounce("zaiBase", function () {
      save(ZAI_BASE, els.zaiBase.value.trim(), els.zaiBaseStatus);
    }, 600);
  });

  els.mcpUrl.addEventListener("input", function () {
    debounce("mcp", function () {
      save(MCP_URL, els.mcpUrl.value.trim(), els.mcpStatus);
    }, 600);
  });

  // Toggle show/hide API key
  var visible = false;
  els.toggleZai.addEventListener("click", function () {
    visible = !visible;
    els.zaiKey.type = visible ? "text" : "password";
    els.toggleZai.textContent = visible ? "Hide" : "Show";
  });
})();

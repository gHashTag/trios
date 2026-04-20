document.querySelectorAll(".tab").forEach((tab) => {
  tab.addEventListener("click", () => {
    document.querySelectorAll(".tab").forEach((t) => t.classList.remove("active"));
    document.querySelectorAll(".tab-content").forEach((c) => c.classList.remove("active"));
    tab.classList.add("active");
    document.getElementById("tab-" + tab.dataset.tab).classList.add("active");
  });
});

const chatInput = document.getElementById("chat-input");
const messagesEl = document.getElementById("messages");

chatInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && chatInput.value.trim()) {
    const msg = chatInput.value.trim();
    appendMessage("you", msg);
    chatInput.value = "";
    chrome.runtime.sendMessage(
      { type: "MCP_REQUEST", payload: { method: "agents/chat", params: { message: msg } } },
      (response) => {
        if (response && response.type === "MCP_RESPONSE") {
          appendMessage("agent", response.data.response || JSON.stringify(response.data));
        } else {
          appendMessage("error", "Connection failed — is trios-server running?");
        }
      }
    );
  }
});

function appendMessage(role, text) {
  const div = document.createElement("div");
  div.className = "message " + role;
  div.textContent = text;
  messagesEl.appendChild(div);
  messagesEl.scrollTop = messagesEl.scrollHeight;
}

loadAgents();
loadTools();

function loadAgents() {
  chrome.runtime.sendMessage(
    { type: "MCP_REQUEST", payload: { method: "agents/list", params: {} } },
    (response) => {
      const el = document.getElementById("agent-list");
      if (response && response.type === "MCP_RESPONSE") {
        const agents = response.data;
        if (Array.isArray(agents) && agents.length === 0) {
          el.textContent = "No agents connected.";
        } else {
          el.textContent = JSON.stringify(agents, null, 2);
        }
      } else {
        el.textContent = "Server not reachable.";
      }
    }
  );
}

function loadTools() {
  chrome.runtime.sendMessage(
    { type: "MCP_REQUEST", payload: { method: "tools/list", params: {} } },
    (response) => {
      const el = document.getElementById("tool-list");
      if (response && response.type === "MCP_RESPONSE") {
        el.textContent = JSON.stringify(response.data, null, 2);
      } else {
        el.textContent = "Server not reachable.";
      }
    }
  );
}

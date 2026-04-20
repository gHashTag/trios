chrome.sidePanel.setPanelBehavior({ openPanelOnActionClick: true });

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === "MCP_REQUEST") {
    const ws = new WebSocket("ws://localhost:3333");
    ws.onopen = () => {
      ws.send(JSON.stringify(message.payload));
    };
    ws.onmessage = (event) => {
      sendResponse({ type: "MCP_RESPONSE", data: JSON.parse(event.data) });
      ws.close();
    };
    ws.onerror = (err) => {
      sendResponse({ type: "MCP_ERROR", error: "WebSocket connection failed" });
      ws.close();
    };
    return true;
  }
});

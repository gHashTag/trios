// Trios Background Service Worker
// Configures sidepanel behavior for the extension.

// Open sidepanel when the extension icon is clicked
chrome.sidePanel.setPanelBehavior({ openPanelOnActionClick: true });

// Optional: log when sidepanel action is triggered
chrome.action.onClicked.addListener((tab) => {
  console.log('[trios] Extension icon clicked — opening sidepanel for tab', tab.id);
});

console.log('[trios] Background service worker loaded');

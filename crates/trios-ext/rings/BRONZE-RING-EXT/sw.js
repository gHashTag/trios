// Trios Background Service Worker
// Opens the sidepanel when the extension icon is clicked.

chrome.sidePanel
  .setPanelBehavior({ openPanelOnActionClick: true })
  .catch((e) => console.error('[trios] setPanelBehavior failed:', e));

chrome.action.onClicked.addListener((tab) => {
  chrome.sidePanel.open({ tabId: tab.id }).catch((e) =>
    console.error('[trios] sidePanel.open failed:', e)
  );
});

console.log('[trios] Background service worker loaded');

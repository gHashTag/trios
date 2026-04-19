/// Content script for cursor.sh
/// Injects agent output into Cursor editor

const CURSOR_EDITOR_CLASS = '.cm-editor';
const CURSOR_TEXTAREA_SELECTOR = 'textarea[aria-label*="Editor"]';

function findCursorEditor(): HTMLElement | null {
  const editor = document.querySelector(CURSOR_EDITOR_CLASS);
  if (editor) {
    return editor;
  }

  const textarea = document.querySelector(CURSOR_TEXTAREA_SELECTOR);
  return textarea as HTMLElement;
}

function injectText(text: string): boolean {
  const editor = findCursorEditor();
  if (!editor) {
    console.error('[Trinity-Cursor] Cursor editor not found');
    return false;
  }

  if (editor.tagName === 'TEXTAREA') {
    const textarea = editor as HTMLTextAreaElement;
    textarea.focus();

    textarea.value = text;
    textarea.dispatchEvent(new Event('input', { bubbles: true }));
    textarea.dispatchEvent(new Event('change', { bubbles: true }));

    console.log('[Trinity-Cursor] Text injected via textarea');
    return true;
  }

  return injectViaClipboard(text);
}

function injectViaClipboard(text: string): boolean {
  navigator.clipboard.writeText(text).then(() => {
    console.log('[Trinity-Cursor] Text copied to clipboard, user needs to paste manually');
  }).catch(e => {
    console.error('[Trinity-Cursor] Clipboard write failed:', e);
  });
  return false;
}

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'inject_text') {
    const success = injectText(message.text);
    sendResponse({ success });
  }

  return true;
});

console.log('[Trinity-Cursor] Content script loaded');
chrome.runtime.sendMessage({ type: 'content_script_loaded', platform: 'cursor' });

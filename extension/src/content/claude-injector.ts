/// Content script for claude.ai
/// Injects agent output into ProseMirror textareas and auto-submits

const PROSEMIRROR_ATTR = 'data-prosemirror-view';

function findProseMirrorTextarea(): HTMLTextAreaElement | null {
  const textareas = document.querySelectorAll('textarea');
  for (const ta of textareas) {
    if (ta.hasAttribute(PROSEMIRROR_ATTR)) {
      return ta as HTMLTextAreaElement;
    }
  }
  return null;
}

function injectTextClaude(text: string): boolean {
  const textarea = findProseMirrorTextarea();
  if (!textarea) {
    console.error('[Trinity-Claude] ProseMirror textarea not found');
    return false;
  }

  textarea.focus();

  const nativeEvent = new Event('input', { bubbles: true });
  Object.defineProperty(nativeEvent, 'target', { writable: false, value: textarea });

  textarea.value = text;
  textarea.dispatchEvent(nativeEvent);

  textarea.dispatchEvent(new Event('change', { bubbles: true }));

  console.log('[Trinity-Claude] Text injected, length:', text.length);
  return true;
}

function autoSubmit(): boolean {
  const buttons = document.querySelectorAll('button, [role="button"]');
  for (const btn of buttons) {
    const label = btn.getAttribute('aria-label')?.toLowerCase();
    const title = btn.getAttribute('title')?.toLowerCase();
    const text = btn.textContent?.toLowerCase();

    if (
      label?.includes('send') ||
      title?.includes('send') ||
      (text?.includes('send') && !text?.includes('settings'))
    ) {
      if (!(btn as HTMLButtonElement).disabled) {
        (btn as HTMLButtonElement).click();
        console.log('[Trinity-Claude] Auto-submitted');
        return true;
      }
    }
  }

  console.warn('[Trinity-Claude] Submit button not found or disabled');
  return false;
}

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message.type === 'inject_text') {
    const success = injectTextClaude(message.text);

    if (message.autoSubmit && success) {
      setTimeout(() => {
        autoSubmit();
        sendResponse({ success: true });
      }, 100);
    } else {
      sendResponse({ success });
    }
  }

  return true;
});

console.log('[Trinity-Claude] Content script loaded');
chrome.runtime.sendMessage({ type: 'content_script_loaded', platform: 'claude' });

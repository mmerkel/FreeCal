import '@testing-library/jest-dom/vitest';
import 'temporal-polyfill/global';

// jsdom has no modal dialogs. This is just enough of them for component
// tests: opening and closing, and Escape sending "cancel" like a browser.
// Focus, the top layer and inertness are checked in the real app.
if (!HTMLDialogElement.prototype.showModal) {
  HTMLDialogElement.prototype.showModal = function (this: HTMLDialogElement) {
    this.setAttribute('open', '');
    this.addEventListener('keydown', (key) => {
      if (key.key !== 'Escape') return;
      if (this.dispatchEvent(new Event('cancel', { cancelable: true }))) this.close();
    });
  };
  HTMLDialogElement.prototype.close = function (this: HTMLDialogElement) {
    if (!this.hasAttribute('open')) return;
    this.removeAttribute('open');
    this.dispatchEvent(new Event('close'));
  };
}

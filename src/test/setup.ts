import '@testing-library/jest-dom/vitest';
import 'temporal-polyfill/global';
import { afterEach } from 'vitest';
import { systemDarkQuery } from './darkQuery';

// jsdom has no modal dialogs. This is just enough of them for component
// tests: opening and closing, and Escape sending "cancel" to the topmost
// dialog like a browser. Focus, the top layer and inertness are checked in
// the real app.
if (!HTMLDialogElement.prototype.showModal) {
  HTMLDialogElement.prototype.showModal = function (this: HTMLDialogElement) {
    this.setAttribute('open', '');
    this.addEventListener('keydown', (key) => {
      // The keydown bubbles through every dialog it is in; only the
      // innermost one handles it.
      if (key.key !== 'Escape' || key.defaultPrevented) return;
      key.preventDefault();
      if (this.dispatchEvent(new Event('cancel', { cancelable: true }))) this.close();
    });
  };
  HTMLDialogElement.prototype.close = function (this: HTMLDialogElement) {
    if (!this.hasAttribute('open')) return;
    this.removeAttribute('open');
    this.dispatchEvent(new Event('close'));
  };
}

// jsdom has no popovers, and as it never matches `:popover-open`, its own
// stylesheet hides every element with a `popover` attribute. This shows and
// hides them through their inline style; the top layer and anchoring are
// checked in the real app.
HTMLElement.prototype.showPopover ??= function (this: HTMLElement) {
  this.style.display = 'block';
};
HTMLElement.prototype.hidePopover ??= function (this: HTMLElement) {
  this.style.removeProperty('display');
};

// jsdom has no pointer capture. Tests send a drag's moves to the window,
// where they would bubble to from the capturing element.
Element.prototype.setPointerCapture ??= () => {};

// jsdom has no media queries. The only one FreeCal asks is whether the system
// is dark, which tests answer through `systemDarkQuery`.
window.matchMedia = () => systemDarkQuery as unknown as MediaQueryList;
afterEach(() => systemDarkQuery.flip(false));

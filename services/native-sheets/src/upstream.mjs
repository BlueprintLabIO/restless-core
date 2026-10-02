// The upstream Model shares its module with the Owl UI. DOM globals are needed
// at module initialization; no editor or browser is created by the worker.
import { JSDOM } from 'jsdom';
const dom = new JSDOM('<!doctype html><html><body></body></html>', { pretendToBeVisual: true });
for (const name of Object.getOwnPropertyNames(dom.window)) {
  if (!(name in globalThis) && name !== 'localStorage' && name !== 'sessionStorage') {
    Object.defineProperty(globalThis, name, { get: () => dom.window[name], configurable: true });
  }
}
for (const name of ['window', 'document', 'Element', 'HTMLElement', 'Node', 'DOMParser', 'XMLSerializer', 'Document', 'DocumentFragment', 'Event', 'CustomEvent', 'MutationObserver', 'navigator', 'HTMLInputElement', 'HTMLCanvasElement']) {
  Object.defineProperty(globalThis, name, { value: dom.window[name], configurable: true });
}
Object.defineProperty(globalThis, 'requestAnimationFrame', { value: dom.window.requestAnimationFrame.bind(dom.window), configurable: true });
Object.defineProperty(globalThis, 'cancelAnimationFrame', { value: dom.window.cancelAnimationFrame.bind(dom.window), configurable: true });
globalThis.ResizeObserver = class { observe() {} unobserve() {} disconnect() {} };
dom.window.Path2D = class {};
dom.window.HTMLCanvasElement.prototype.getContext = () => ({ measureText: (text) => ({ width: String(text).length * 7 }) });
console.debug = () => {};
export const upstream = await import('@odoo/o-spreadsheet/dist/o_spreadsheet.esm.js');
export const { Model, helpers } = upstream;

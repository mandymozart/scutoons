import { ScutoonPages } from './ScutoonPages.js';
import { initAnimatedFavicon } from './favicon.js';

window.scutoonPages = new ScutoonPages('de');
document.addEventListener('DOMContentLoaded', () => {
  window.scutoonPages.init();
  initAnimatedFavicon();
});

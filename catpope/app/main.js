/**
 * Main entry point for SCUTOONS app
 */
import { Scutoon } from './Scutoon.js';
import { sanitizeNumber } from './utils/template.js';

const scutoon = new Scutoon('de');
window.scutoon = scutoon;

function handleNavigation() {
  const urlParams = new URLSearchParams(window.location.search);
  const view = urlParams.get('view') || 'toon';
  const chapter = sanitizeNumber(urlParams.get('chapter'), 1, 1, 999);
  const song = sanitizeNumber(urlParams.get('song'), 1, 1, 999);

  scutoon.init(view, chapter, song);
}

document.addEventListener('DOMContentLoaded', () => {
  handleNavigation();
});
window.addEventListener('popstate', (event) => {
  handleNavigation();
});

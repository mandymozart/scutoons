/**
 * Main entry point for SCUTOONS app
 */
import { Scutoon } from './Scutoon.js';
import { sanitizeNumber } from './utils/template.js';

// Initialize the Scutoon app and expose globally for onclick handlers
const scutoon = new Scutoon('de');
// TODO: Vunerability, do not expose scutoon to global scope in production
window.scutoon = scutoon;

// Function to handle navigation based on URL params
function handleNavigation() {
  const urlParams = new URLSearchParams(window.location.search);
  const view = urlParams.get('view') || 'toon';
  const chapter = sanitizeNumber(urlParams.get('chapter'), 1, 1, 100);
  const season = sanitizeNumber(urlParams.get('season'), 1, 1, 10);
  const song = sanitizeNumber(urlParams.get('song'), 1, 1, 100);

  // Use init() to properly process QR codes and then navigate
  scutoon.init(view, chapter, season, song);
}

// Initialize on page load
document.addEventListener('DOMContentLoaded', () => {
  handleNavigation();
});

// Handle browser back/forward buttons
window.addEventListener('popstate', (event) => {
  handleNavigation();
});

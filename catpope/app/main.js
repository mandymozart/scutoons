/**
 * Main entry point for SCUTOONS app
 */
import { Scutoon } from './Scutoon.js';

// Initialize the Scutoon app and expose globally for onclick handlers
window.scutoon = new Scutoon('.', 'de');

// Function to handle navigation based on URL params
function handleNavigation() {
  const urlParams = new URLSearchParams(window.location.search);
  const view = urlParams.get('view') || 'toon';
  const chapter = parseInt(urlParams.get('chapter')) || 1;
  const season = parseInt(urlParams.get('season')) || 1;
  const song = parseInt(urlParams.get('song')) || 1;

  // Use init() to properly process QR codes and then navigate
  window.scutoon.init(view, chapter, season, song);
}

// Initialize on page load
document.addEventListener('DOMContentLoaded', () => {
  handleNavigation();
});

// Handle browser back/forward buttons
window.addEventListener('popstate', (event) => {
  handleNavigation();
});

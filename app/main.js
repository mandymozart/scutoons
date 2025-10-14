/**
 * Main entry point for Scutoon Pages App
 */
import { ScutoonPages } from './ScutoonPages.js';

// Initialize the Scutoon Pages app and expose globally for onclick handlers
window.scutoonPages = new ScutoonPages('de');

// Initialize on page load
document.addEventListener('DOMContentLoaded', () => {
  window.scutoonPages.init();
});

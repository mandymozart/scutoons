import { ScutoonPages } from './ScutoonPages.js';
window.scutoonPages = new ScutoonPages('de');
document.addEventListener('DOMContentLoaded', () => window.scutoonPages.init());

/**
 * Header component - Renders site header with toon navigation
 */
import { loadToons } from '../utils/api.js';
import { initHeaderScrollHandler } from '../utils/scroll.js';
import { isItemPublic } from '../utils/template.js';

/**
 * Render the site header with toon navigation
 * @param {string} currentLang - Current language code
 * @returns {string} HTML string for header
 */
export async function renderHeader(currentLang = 'de') {
  let toons = [];
  
  try {
    const allToons = await loadToons();
    // Filter to only published toons
    toons = allToons.filter(toon => isItemPublic(toon));
  } catch (error) {
    console.error('Failed to load toons:', error);
  }

  const toonsHtml = toons.map(toon => {
    const name = toon.i18n[currentLang]?.name || toon.i18n.en?.name || toon.id;
    const avatar = toon.avatar ? `/${toon.avatar}` : '/assets/default-avatar.png';
    const uri = toon.uri || toon.id;
    
    return `
      <li class="nav-item">
        <a href="/${uri}/">
          <img src="${avatar}" alt="${name}" />
          <span class="name">${name}</span>
        </a>
      </li>`;
  }).join('');

  return `
    <header>
      <div class="logo">
        <a href="../">
          <img src="../assets/SCUTOONS-logo.svg" alt="SCUTOONS" />
        </a>
      </div>
      <nav class="nav">
        <ul class="toons">${toonsHtml}</ul>
        <ul class="menu"></ul>
      </nav>
    </header>`;
}

/**
 * Initialize header after rendering
 */
export function initHeader() {
  initHeaderScrollHandler();
}

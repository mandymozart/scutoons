/**
 * SequenceNavigation component - Navigation between sequence items (chapters/songs)
 * Handles navigation within and across seasons, skipping unpublished items
 */
import { escapeHtml, isItemPublic } from '../utils/template.js';
import { asset } from '../utils/cdn.js';

/**
 * Filters sequence items to only include published/available items
 * @param {Array} sequenceItems - Array of loaded sequence items with metadata
 * @returns {Array} - Filtered array of published items only
 */
function filterPublishedItems(sequenceItems) {
  if (!sequenceItems) return [];
  return sequenceItems.filter(item => isItemPublic(item));
}

/**
 * Finds the current item's position in the sequence
 * @param {Array} sequenceItems - Array of loaded sequence items
 * @param {string} itemType - 'chapter' or 'song'
 * @param {number} itemNumber - The item number
 * @returns {number} - Index in sequence or -1 if not found
 */
function findCurrentPosition(sequenceItems, itemType, itemNumber) {
  return sequenceItems.findIndex(item => 
    item.itemType === itemType && item.itemNumber === itemNumber
  );
}

/**
 * Renders the sequence navigation component
 * @param {Object} options - Navigation options
 * @param {Array} options.sequenceItems - Array of loaded sequence items with full metadata
 * @param {string} options.currentType - 'chapter' or 'song'
 * @param {number} options.currentNumber - Current item number
 * @returns {string} - HTML for navigation
 */
export function renderSequenceNavigation({ sequenceItems, currentType, currentNumber }) {
  if (!sequenceItems || sequenceItems.length === 0) {
    return renderBasicNavigation();
  }

  // Filter to only published items
  const publishedItems = filterPublishedItems(sequenceItems);
  
  if (publishedItems.length === 0) {
    return renderBasicNavigation();
  }

  const currentPosition = findCurrentPosition(publishedItems, currentType, currentNumber);
  
  if (currentPosition === -1) {
    // Current item not found in published sequence, show basic navigation
    return renderBasicNavigation();
  }

  // Get previous and next items (only from published items)
  const prevItem = currentPosition > 0 ? publishedItems[currentPosition - 1] : null;
  const nextItem = currentPosition < publishedItems.length - 1 ? publishedItems[currentPosition + 1] : null;

  // Render navigation
  let prevButton = '';
  let nextButton = '';

  if (prevItem) {
    const prevTypeLabel = prevItem.itemType === 'chapter' ? 'Kapitel' : 'Song';
    const prevTitle = prevItem.title || '';
    prevButton = `
      <div onclick="scutoon.navigateTo('${prevItem.itemType}', { ${prevItem.itemType}: ${prevItem.itemNumber} })" class="btn-nav btn-prev" title="${escapeHtml(`${prevTypeLabel} ${prevItem.itemNumber}: ${prevTitle}`)}">
      <span class="nav-icon">←</span>
      <div>
        <span class="nav-type">${escapeHtml(prevTypeLabel)} ${prevItem.itemNumber}</span>
        <span class="nav-title">${escapeHtml(prevTitle)}</span>
        </div>
      </div>
    `;
  } else {
    // At the beginning of published sequence
    prevButton = `
      <div class="btn-nav btn-prev btn-disabled btn-hidden" disabled title="Anfang der Season">
        <span class="nav-icon">←</span>
        <div>
          <span class="nav-label">Anfang</span>
        </div>
      </div>
    `;
  }

  if (nextItem) {
    const nextTypeLabel = nextItem.itemType === 'chapter' ? 'Kapitel' : 'Song';
    const nextTitle = nextItem.title || '';
    nextButton = `
      <div onclick="scutoon.navigateTo('${nextItem.itemType}', { ${nextItem.itemType}: ${nextItem.itemNumber} })" class="btn-nav btn-next" title="${escapeHtml(`${nextTypeLabel} ${nextItem.itemNumber}: ${nextTitle}`)}">
      <div class="nav-image">
        <img src="${asset(nextItem.thumbnail)}" alt="${escapeHtml(nextTitle)}">
      </div>
      <div>
        <span class="nav-type">${escapeHtml(nextTypeLabel)} ${nextItem.itemNumber}</span>
        <span class="nav-title">${escapeHtml(nextTitle)}</span>
      </div>
      <span class="nav-icon">→</span>
      </div>
    `;
  } else {
    // At the end of published sequence
    nextButton = `
      <div class="btn-nav btn-next btn-disabled" disabled title="Ende der Season">
        <div>
          <span class="nav-label">Ende (to be continued)</span>
          </div>
      </div>
    `;
  }

  const overviewButton = `
    <div onclick="scutoon.navigateTo('toon')" class="btn-nav btn-overview" title="Zur Übersicht">
      <span class="nav-icon">⊞</span>
      <span class="nav-label">Overview</span>
    </div>
  `;

  return `
    <nav class="sequence-navigation">
      ${overviewButton}
      <div class="sequence-nav-container">
        ${prevButton}
        ${nextButton}
      </div>
    </nav>
  `;
}

/**
 * Renders basic navigation when sequence data is not available
 * @returns {string} - HTML for basic navigation
 */
function renderBasicNavigation() {
  return `
    <nav class="sequence-navigation">
      <div class="sequence-nav-container">
        <div onclick="scutoon.navigateTo('toon')" class="btn-nav btn-overview" title="Zur Übersicht">
          <span class="nav-icon">⊞</span>
          <span class="nav-label">Übersicht</span>
        </div>
      </div>
    </nav>
  `;
}

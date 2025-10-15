/**
 * SequenceNavigation component - Navigation between sequence items (chapters/songs)
 * Handles navigation within and across seasons, skipping unpublished items
 */
import { escapeHtml } from '../utils/template.js';

/**
 * Filters sequence items to only include published/available items
 * @param {Array} sequenceItems - Array of loaded sequence items with metadata
 * @returns {Array} - Filtered array of published items only
 */
function filterPublishedItems(sequenceItems) {
  if (!sequenceItems) return [];
  return sequenceItems.filter(item => {
    // Include items that are published or don't have a status field
    return !item.status || item.status === 'published';
  });
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
    const prevLabel = prevItem.itemType === 'chapter' ? `Kapitel ${prevItem.itemNumber}` : `Song ${prevItem.itemNumber}`;
    prevButton = `
      <div onclick="scutoon.navigateTo('${prevItem.itemType}', { ${prevItem.itemType}: ${prevItem.itemNumber} })" class="btn-nav btn-prev" title="${escapeHtml(prevLabel)}">
        <span class="nav-icon">←</span>
        <span class="nav-label">${escapeHtml(prevLabel)}</span>
      </div>
    `;
  } else {
    // At the beginning of published sequence
    prevButton = `
      <div class="btn-nav btn-prev btn-disabled" disabled title="Anfang der Season">
        <span class="nav-icon">←</span>
        <span class="nav-label">Anfang</span>
      </div>
    `;
  }

  if (nextItem) {
    const nextLabel = nextItem.itemType === 'chapter' ? `Kapitel ${nextItem.itemNumber}` : `Song ${nextItem.itemNumber}`;
    nextButton = `
      <div onclick="scutoon.navigateTo('${nextItem.itemType}', { ${nextItem.itemType}: ${nextItem.itemNumber} })" class="btn-nav btn-next" title="${escapeHtml(nextLabel)}">
        <span class="nav-label">${escapeHtml(nextLabel)}</span>
        <span class="nav-icon">→</span>
      </div>
    `;
  } else {
    // At the end of published sequence
    nextButton = `
      <div class="btn-nav btn-next btn-disabled" disabled title="Ende der Season">
        <span class="nav-label">Ende</span>
        <span class="nav-icon">→</span>
      </div>
    `;
  }

  const overviewButton = `
    <div onclick="scutoon.navigateTo('toon')" class="btn-nav btn-overview" title="Zur Übersicht">
      <span class="nav-icon">⊞</span>
      <span class="nav-label"></span>
    </div>
  `;

  return `
    <nav class="sequence-navigation">
      <div class="sequence-nav-container">
        ${prevButton}
        ${overviewButton}
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
        <button onclick="scutoon.navigateTo('toon')" class="btn-nav btn-overview" title="Zur Übersicht">
          <span class="nav-icon">⊞</span>
          <span class="nav-label">Übersicht</span>
        </button>
      </div>
    </nav>
  `;
}

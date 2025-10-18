/**
 * API utilities for loading content
 */
import { API_BASE, TOON_ID } from '../config.js';
import { sanitizeNumber } from './template.js';

/**
 * Validates language code to prevent path traversal
 * @param {string} lang - Language code
 * @returns {string} - Validated language code or default 'de'
 */
function validateLang(lang) {
  // Only allow lowercase letters (ISO 639-1 language codes)
  if (typeof lang === 'string' && /^[a-z]{2}$/.test(lang)) {
    return lang;
  }
  console.warn('Invalid language code, defaulting to "de":', lang);
  return 'de';
}

export async function loadToon(lang) {
  const safeLang = validateLang(lang);
  return fetch(`${API_BASE}/${TOON_ID}/${safeLang}/toon.json`)
    .then(response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    });
}

export async function loadChapter(lang, chapterNumber) {
  const safeLang = validateLang(lang);
  const safeNumber = sanitizeNumber(chapterNumber, 1, 1, 100);
  return fetch(`${API_BASE}/${TOON_ID}/${safeLang}/chapter${safeNumber}.json`)
    .then(response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    });
}

export async function loadSeason(lang, number) {
  const safeLang = validateLang(lang);
  const safeNumber = sanitizeNumber(number, 1, 1, 10);
  return fetch(`${API_BASE}/${TOON_ID}/${safeLang}/season${safeNumber}.json`)
    .then(response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    });
}

export async function loadSong(lang, songNumber) {
  const safeLang = validateLang(lang);
  const safeNumber = sanitizeNumber(songNumber, 1, 1, 100);
  return fetch(`${API_BASE}/${TOON_ID}/${safeLang}/song${safeNumber}.json`)
    .then(response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    });
}

export async function loadCredits(lang) {
  const safeLang = validateLang(lang);
  return fetch(`${API_BASE}/${TOON_ID}/${safeLang}/credits.json`)
    .then(response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    });
}

// Load songs metadata for player
export async function loadSongs(lang) {
  const safeLang = validateLang(lang);
  const songPromises = [];
  for (let i = 1; i <= 6; i++) {
    songPromises.push(
      fetch(`${API_BASE}/${TOON_ID}/${safeLang}/song${i}.json`)
        .then(response => {
          if (!response.ok) throw new Error(`HTTP ${response.status}`);
          return response.json();
        })
        .then(data => ({ id: i, ...data }))
    );
  }
  return Promise.all(songPromises);
}

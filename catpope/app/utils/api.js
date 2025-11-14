/**
 * @typedef {import('../types').Toon} Toon
 * @typedef {import('../types').Chapter} Chapter
 * @typedef {import('../types').Season} Season
 * @typedef {import('../types').Song} Song
 * @typedef {import('../types').SongWithId} SongWithId
 * @typedef {import('../types').SongCredits} SongCredits
 * @typedef {import('../types').FullCredits} FullCredits
 * @typedef {import('../types').Character} Character
 * @typedef {import('../types').Group} Group
 * @typedef {import('../types').Interaction} InteractionType
 * @typedef {import('../types').Renderer} RendererType
 */

import { API_BASE, TOON_ID } from '../config.js';
import { sanitizeNumber } from './template.js';
import { Renderer, Interaction } from '../constants.js';

/**
 * @param {string} path
 * @returns {Promise<any>}
 */
export async function loadJSON(path) {
  return fetch(path)
    .then(response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    });
}

/**
 * @param {string} lang
 * @returns {string}
 */
function validateLang(lang) {
  // Only allow lowercase letters (ISO 639-1 language codes)
  if (typeof lang === 'string' && /^[a-z]{2}$/.test(lang)) {
    return lang;
  }
  console.warn('Invalid language code, defaulting to "de":', lang);
  return 'de';
}

/**
 * @param {string} lang
 * @returns {Promise<Toon>}
 */
export async function loadToon(lang) {
  const safeLang = validateLang(lang);
  return fetch(`${API_BASE}/${TOON_ID}/${safeLang}/toon.json`)
    .then(response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    });
}

/**
 * @param {Group} group
 * @returns {Group}
 */
export function castGroupTypes(group) {
  if (group.renderer && typeof group.renderer === 'string') {
    group.renderer = /** @type {RendererType} */ (Renderer[group.renderer.toUpperCase()]);
  }
  if (group.interaction && typeof group.interaction === 'string') {
    const interactionKey = group.interaction.toUpperCase().replace(/-/g, '_');
    group.interaction = /** @type {InteractionType} */ (Interaction[interactionKey]);
  }
  return group;
}

/**
 * @param {string} lang
 * @param {number} chapterNumber
 * @returns {Promise<Chapter>}
 */
export async function loadChapter(lang, chapterNumber) {
  const safeLang = validateLang(lang);
  const safeNumber = sanitizeNumber(chapterNumber, 1, 1, 100);
  return fetch(`${API_BASE}/${TOON_ID}/${safeLang}/chapter${safeNumber}.json`)
    .then(response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    })
    .then(chapter => {
      // Cast string renderer/interaction values to enum types
      if (chapter.groups && Array.isArray(chapter.groups)) {
        chapter.groups = chapter.groups.map(group => {
          // Only cast inline groups, not GroupRef objects
          if ('renderer' in group) {
            return castGroupTypes(group);
          }
          return group;
        });
      }
      return chapter;
    });
}

/**
 * @param {string} lang
 * @param {number} number
 * @returns {Promise<Season>}
 */
export async function loadSeason(lang, number) {
  const safeLang = validateLang(lang);
  const safeNumber = sanitizeNumber(number, 1, 1, 10);
  return fetch(`${API_BASE}/${TOON_ID}/${safeLang}/season${safeNumber}.json`)
    .then(response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    });
}

/**
 * @param {string} lang
 * @param {number} songNumber
 * @returns {Promise<Song>}
 */
export async function loadSong(lang, songNumber) {
  const safeLang = validateLang(lang);
  const safeNumber = sanitizeNumber(songNumber, 1, 1, 100);
  return fetch(`${API_BASE}/${TOON_ID}/${safeLang}/song${safeNumber}.json`)
    .then(response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    });
}

/**
 * @param {string} lang
 * @returns {Promise<FullCredits>}
 */
export async function loadCredits(lang) {
  const safeLang = validateLang(lang);
  return fetch(`${API_BASE}/${TOON_ID}/${safeLang}/credits.json`)
    .then(response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    });
}

/**
 * @param {string} lang
 * @returns {Promise<SongWithId[]>}
 */
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

/** @returns {Promise<Toon[]>} */
export async function loadToons() {
  return fetch(`/api/static/toons.json`)
    .then(response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    });
}

/**
 * @param {string} lang
 * @returns {Promise<Character[]>}
 */
export async function loadCharacters(lang) {
  return fetch(`${API_BASE}/${TOON_ID}/${lang}/characters.json`)
    .then(response => {
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    });
}

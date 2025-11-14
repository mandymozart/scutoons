/**
 * @typedef {import('../../types').Character} Character
 */

import { escapeAttr } from '../../utils/template.js';

/**
 * @param {Character} character
 * @returns {string}
 */
export function renderCharacter(character) {
  return `
    <div class="vn-character" data-name="${escapeAttr(character.name)}">
      ${escapeAttr(character.name)}
    </div>
  `;
}
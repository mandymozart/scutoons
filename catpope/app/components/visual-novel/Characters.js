import { renderCharacter } from './Character.js';

/**
 * @typedef {import('../../types').Character} Character
 */

/**
 * @param {Array<{character: Character, imageSrc: string}>} characters
 * @returns {string}
 */
export function renderCharacters(characters) {
  const charactersHtml = characters
    .map(({ character, imageSrc }) => renderCharacter(character, imageSrc))
    .join('');
  
  return `
    <div class="vn-characters">
      ${charactersHtml}
    </div>
  `;
}

/**
 * 
 * Concept for the characters
 * 
 * * the need to be in front of the message box, or behind, 
 * in front they use the avatar, behind the large variant.
 * 
 * * they need to adjust their mood
 * * they can be stacked in both positions
 */
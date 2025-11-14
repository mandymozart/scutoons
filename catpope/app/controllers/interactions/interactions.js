/**
 * @typedef {import('../../types').Group} Group
 * @typedef {import('../../types').IScutoon} IScutoon
 * @typedef {import('../../types').IInteraction} IInteraction
 */

import { WebtoonInteraction } from './WebtoonInteraction.js';
import { VisualNovelInteraction } from './VisualNovelInteraction.js';
import { createRenderer } from '../../renderers/renderers.js';

/**
 * @param {IScutoon} scutoon
 * @param {Group} group
 * @returns {IInteraction | null}
 */
export function createInteraction(scutoon, group) {
  const renderer = createRenderer(scutoon, group.renderer);
  
  switch (group.interaction) {
    case 'webtoon':
      return new WebtoonInteraction(scutoon, group, renderer);
    case 'visual-novel':
      return new VisualNovelInteraction(scutoon, group, renderer);
    default:
      console.error(`Unknown interaction type: ${group.interaction}`);
      return null;
  }
}
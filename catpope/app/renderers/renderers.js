/**
 * @typedef {import('../types').Renderer} Renderer
 * @typedef {import('../types').IScutoon} IScutoon
 * @typedef {import('../types').IRenderer} IRenderer
 */

import { VerticalScrollRenderer } from './VerticalScrollRenderer.js';
import { HorizontalScrollRenderer } from './HorizontalScrollRenderer.js';

const rendererMap = {
  'vertical': VerticalScrollRenderer,
  'horizontal': HorizontalScrollRenderer
};

/**
 * @param {IScutoon} scutoon
 * @param {Renderer} rendererType
 * @returns {IRenderer}
 */
export function createRenderer(scutoon, rendererType) {
  const RendererClass = rendererMap[rendererType];
  
  if (!RendererClass) {
    throw new Error(`Unknown renderer type: ${rendererType}. Must be 'vertical' or 'horizontal'.`);
  }
  
  return new RendererClass(scutoon);
}

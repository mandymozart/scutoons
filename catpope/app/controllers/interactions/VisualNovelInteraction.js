/**
 * @typedef {import('../../types').Slice} Slice
 * @typedef {import('../../types').Group} Group
 * @typedef {import('../../types').IScutoon} IScutoon
 * @typedef {import('../../types').IRenderer} IRenderer
 * @typedef {import('../../types').VNEvent} VNEvent
 * @typedef {import('../../renderers/types').RenderOptions} RenderOptions
 */

import { EventProcessor } from '../events/EventProcessor';

export class VisualNovelInteraction {
  /**
   * @param {IScutoon} scutoon
   * @param {Group} group
   * @param {IRenderer} renderer
   */
  constructor(scutoon, group, renderer) {
    this.scutoon = scutoon;
    this.renderer = renderer;
    this.group = group;

    this.eventProcessor = null
    
    console.log("VisualNovelInteraction initialized for group:", group.label);
  }
  
  /** @returns {Promise<void>} */
  async init() {
    console.log("Starting visual novel interaction");
    await this.renderBackground();
    this.eventProcessor = new EventProcessor(this.scutoon, this.group.events);
  }
  
  /** @returns {Promise<void>} */
  async renderBackground() {
    if (!this.renderer) {
      console.error('No renderer provided to visual novel interaction');
      return;
    }
    
    await this.renderer.render({
      slices: this.group.slices
    });
  }

  /** @returns {Promise<void>} */
  async cleanup() {
    console.log("Cleaning up visual novel interaction");
    
    if (this.scutoon.contentEl) {
      this.scutoon.contentEl.removeAttribute('data-interaction');
      this.scutoon.contentEl.removeAttribute('data-renderer');
    }
    this.eventProcessor.cleanup();
  }
}

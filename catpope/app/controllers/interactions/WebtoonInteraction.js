/**
 * @typedef {import('../../types').Group} Group
 * @typedef {import('../../types').IScutoon} IScutoon
 * @typedef {import('../../types').IRenderer} IRenderer
 */

export class WebtoonInteraction {
  /**
   * @param {IScutoon} scutoon
   * @param {Group} group
   * @param {IRenderer} renderer
   */
  constructor(scutoon, group, renderer) {
    this.scutoon = scutoon;
    this.group = group;
    this.renderer = renderer;
    this.onComplete = null;
  }

  /** @returns {Promise<void>} */
  async init() {
    console.log('Webtoon interaction (default passive rendering)');
    
    if (this.renderer && this.group.slices && this.group.slices.length > 0) {
      await this.renderer.render({
        slices: this.group.slices
      });
    }
    
    if (this.onComplete) {
      this.onComplete(null);
    }
  }

  /** @returns {Promise<void>} */
  async cleanup() {
    console.log('Webtoon interaction cleanup');
  }
}

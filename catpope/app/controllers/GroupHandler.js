/**
 * @typedef {import('../types').Group} Group
 * @typedef {import('../types').Chapter} Chapter
 * @typedef {import('../types').Interaction} Interaction
 * @typedef {import('../types').Renderer} Renderer
 * @typedef {import('../types').RenderOptions} RenderOptions
 * @typedef {import('../types').IScutoon} IScutoon
 * @typedef {import('../types').IRenderer} IRenderer
 */

import { 
  isLastGroup, 
  hasEvents, 
  isInteractiveGroup,
  getGroupRenderer,
  flattenGroupSlices,
  collectNonInteractiveGroups
} from '../utils/groups.js';
import { EventProcessor } from './events/EventProcessor.js';
import { createInteraction } from './interactions/interactions.js';
import { createRenderer } from '../renderers/renderers.js';

/**
 * GroupHandler - Coordinates processing of chapter groups
 * Decides between webtoon, event, or interaction handling
 */
export class GroupHandler {
  /**
   * @param {IScutoon} scutoon
   * @param {Chapter} chapter
   * @param {Group[]} groups
   */
  constructor(scutoon, chapter, groups) {
    this.scutoon = scutoon;
    this.chapter = chapter;
    this.groups = groups;
    
    /** @type {IRenderer | null} */
    this.currentRenderer = null;
    /** @type {import('../types').IInteraction | null} */
    this.currentInteraction = null;
  }

  /**
   * @param {number} groupIndex
   * @returns {Promise<string | null>}
   */
  async processGroup(groupIndex) {
    const group = this.groups[groupIndex];
    const isLast = isLastGroup(groupIndex, this.groups.length);
    
    // Check if group has events
    if (hasEvents(group)) {
      return await this.processEventGroup(group, isLast);
    }
    
    // Check if group has interaction
    if (isInteractiveGroup(group)) {
      return await this.processInteractionGroup(group, isLast);
    }
    
    // Default: webtoon (non-interactive) group
    return await this.processWebtoonGroup(groupIndex, isLast);
  }

  /**
   * @param {Group} group
   * @param {boolean} isLast
   * @returns {Promise<string | null>}
   */
  async processEventGroup(group, isLast) {
    console.log(`Processing events for group: ${group.label}`);
    
    // Render slices if present
    if (group.slices && group.slices.length > 0) {
      await this.render(group.renderer, {
        slices: group.slices
      });
    }
    
    // Process events
    const eventProcessor = new EventProcessor(this.scutoon, group.events);
    const nextLabel = await eventProcessor.run();
    eventProcessor.cleanup();
    
    // Handle completion
    if (nextLabel) {
      console.log(`Events complete, jumping to: ${nextLabel}`);
      return nextLabel;
    } else if (!isLast) {
      console.error('⚠️ Events ended without next label (not last group)');
      return null;
    } else {
      console.log('Events complete, chapter ended (last group)');
      return null;
    }
  }

  /**
   * @param {Group} group
   * @param {boolean} isLast
   * @returns {Promise<string | null>}
   */
  async processInteractionGroup(group, isLast) {
    const interactionType = group.interaction;
    
    console.log('Starting interaction:', {
      label: group.label,
      type: interactionType
    });
    
    const interaction = createInteraction(this.scutoon, group);
    this.currentInteraction = interaction;
    
    if (!interaction) {
      console.error(`Failed to create interaction: ${interactionType}`);
      return null;
    }
    
    // Set up completion handler
    return new Promise((resolve) => {
      interaction.onComplete = async (jumpToLabel) => {
        if (jumpToLabel) {
          console.log(`Interaction complete, jumping to: ${jumpToLabel}`);
          resolve(jumpToLabel);
        } else if (!isLast) {
          console.error('⚠️ Interaction ended without jumpTo label (not last group)');
          alert('Configuration Error: Interaction must end with a jumpTo label or explicit end.');
          resolve(null);
        } else {
          console.log('Interaction complete, chapter ended (last group)');
          resolve(null);
        }
      };
      
      interaction.init();
    });
  }

  /**
   * @param {number} startIndex
   * @param {boolean} isLast
   * @returns {Promise<string | null>}
   */
  async processWebtoonGroup(startIndex, isLast) {
    // Collect consecutive non-interactive groups
    const { groups, nextInteractionIndex } = collectNonInteractiveGroups(
      this.groups,
      startIndex
    );
    
    if (groups.length === 0) {
      return null;
    }
    
    const slices = flattenGroupSlices(groups);
    console.log(`Rendering ${slices.length} stitched slices from ${groups.length} groups`);
    
    const rendererType = getGroupRenderer(groups[0]);
    await this.render(rendererType, {
      slices
    });
    
    // No next label for webtoon groups
    return null;
  }

  /**
   * @param {Renderer} rendererType
   * @param {RenderOptions} options
   * @returns {Promise<void>}
   */
  async render(rendererType, options) {
    // Cleanup previous renderer if exists
    if (this.currentRenderer) {
      await this.currentRenderer.cleanup();
    }
    
    // Create new renderer instance
    this.currentRenderer = createRenderer(this.scutoon, rendererType);
    
    // Merge with instance defaults
    const renderOptions = {
      chapter: this.chapter,
      ...options
    };
    
    // Render
    await this.currentRenderer.render(renderOptions);
  }

  /**
   * @returns {Promise<void>}
   */
  async cleanup() {
    if (this.currentInteraction && this.currentInteraction.cleanup) {
      await this.currentInteraction.cleanup();
      this.currentInteraction = null;
    }
    
    if (this.currentRenderer) {
      await this.currentRenderer.cleanup();
      this.currentRenderer = null;
    }
  }
}

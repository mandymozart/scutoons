import { StrollInteraction } from './interactions/StrollInteraction.js';
export class InteractionManager {
  constructor(interactions) {
    this.interactions = interactions || [];
    this.currentInteraction = null;
    // Load only the interactions needed asynchronously
    // currently only stroll interaction is implemented
  }
}
/**
 * @typedef {import('../../types.js').VNEvent} VNEvent
 * @typedef {import("../../types.js").IScutoon} IScutoon
 */

import { handleShowMessage } from "../../components/visual-novel/MessageBox.js";
import { handleShowChoices } from "../../components/visual-novel/Choices.js";
import { handleAddChoice } from "@/components/visual-novel/Choice.js";
/** */
export class EventProcessor {
  /**
   * @param {IScutoon} scutoon - Scutoon instance
   * @param {VNEvent[]} events - Events to process
   */
  constructor(scutoon, events) {
    this.scutoon = scutoon;
    this.events = events || [];
    this.currentEventIndex = 0;

    // UI Elements
    this.messageBoxEl = null;
    this.choicesEl = null;
    this.characterSlotFrontLeftEl = null;
    this.characterSlotFrontRightEl = null;
    this.characterSlotBackLeftEl = null;
    this.characterSlotBackRightEl = null;
  }

  /**
   * @returns {Promise<string|null>} Next label to jump to, or null
   */
  async run() {
    console.log(`Processing ${this.events.length} events`);

    while (this.currentEventIndex < this.events.length) {
      const event = this.events[this.currentEventIndex];

      console.log(
        `Event ${this.currentEventIndex + 1}/${this.events.length}: ${
          event.type
        }`
      );

      const result = await this.processEvent(event);

      // If event returns jumpTo, exit early
      if (result && result.jumpTo) {
        console.log(`Event returned jumpTo: ${result.jumpTo}`);
        return result.jumpTo;
      }

      this.currentEventIndex++;
    }

    console.log("All events processed");
    return null;
  }

  /**
   * Process a single event
   * @param {VNEvent} event - Event to process
   * @returns {Promise<{jumpTo?: string}|void>}
   */
  async processEvent(event) {
    switch (event.type) {
      // Visual Novel Events
      case "showMessage":
        return await handleShowMessage(this.scutoon, this.messageBoxEl, event);

      case "showChoices":
        return await handleShowChoices(this.scutoon, this.choicesEl, event);

      case "addChoice":
        return await handleAddChoice(this.scutoon, this.choicesEl, event);

      case "joinScene":
        return await this.vnEvents.joinScene(event);

      case "leaveScene":
        return await this.vnEvents.leaveScene(event);

      // Flow Control
      case "end":
        return { jumpTo: event.jumpTo };

      default:
        console.warn(`Unknown event type: ${/** @type {any} */ (event).type}`);
        return;
    }
  }

  /**
   * Cleanup event processor
   */
  cleanup() {
    // TODO double check which handlers need clean up.
  }
}

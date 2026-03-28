/**
 * @typedef {import('../types').Chapter} Chapter
 * @typedef {import('../types').Group} Group
 * @typedef {import('../types').IScutoon} IScutoon
 * @typedef {import('../types').IChapterView} IChapterView
 */

import { loadChapter } from '../utils/api.js';
import { MembershipRequiredView } from './MembershipRequiredView.js';
import {
  populateGroups,
  buildGroupMap,
  getFirstGroupLabel,
  getGroupByLabel
} from '../utils/groups.js';
import { updateURL } from '../utils/navigation.js';
import { GroupHandler } from '../controllers/GroupHandler.js';

/** @implements {IChapterView} */
export class ChapterView {
  scutoon;
  currentChapter;
  groups;
  groupMap;
  groupHandler;

  /** @param {IScutoon} scutoon */
  constructor(scutoon) {
    this.scutoon = scutoon;
    this.currentChapter = null;
    this.groups = null;
    this.groupMap = null;
    this.groupHandler = null;
  }

  /**
   * @param {number} [chapterNumber=1]
   * @param {string | null} [label=null]
   * @returns {Promise<Chapter | null>}
   */
  async render(chapterNumber = 1, label = null) {
    // Check membership before rendering chapter
    if (!this.scutoon.membershipStore.hasValidMembership()) {
      const membershipView = new MembershipRequiredView(this.scutoon);
      membershipView.render(chapterNumber);
      return null;
    }

    const data = await loadChapter(this.scutoon.navigationStore.currentLang, chapterNumber);
    this.currentChapter = data;

    // Populate groups (load external group files if referenced via src)
    const groupsArray = data.groups || [];
    this.groups = await populateGroups(groupsArray, data.language, data.number);
    
    console.log(`Loaded ${this.groups.length} groups`);
    
    // Build label map
    this.groupMap = buildGroupMap(this.groups);
    
    // Create group handler
    this.groupHandler = new GroupHandler(
      this.scutoon,
      data,
      this.groups
    );
    
    // Determine start label
    const startLabel = label || getFirstGroupLabel(this.groups);
    if (!startLabel) {
      console.error('No labeled groups found in chapter');
      return data;
    }
    
    // Start rendering from label
    await this.renderFromLabel(startLabel);

    return data;
  }


  /**
   * @param {string} startLabel
   * @returns {Promise<void>}
   */
  async renderFromLabel(startLabel) {
    console.log(`Rendering from label: ${startLabel}`);
    
    // Find starting group by label
    const startInfo = getGroupByLabel(this.groupMap, startLabel);
    if (!startInfo) {
      console.error(`Label not found: ${startLabel}`);
      return;
    }
    
    // Process the group and get next label (if any)
    const nextLabel = await this.groupHandler.processGroup(startInfo.index);
    
    // If there's a next label, jump to it
    if (nextLabel) {
      await this.jumpToLabel(nextLabel);
    }
  }
  
  /**
   * @param {string} label
   * @returns {Promise<void>}
   */
  async jumpToLabel(label) {
    console.log(`Jumping to label: ${label}`);
    
    updateURL({ 
      view: 'chapter', 
      chapter: this.currentChapter.number,
      label: label,
      lang: this.scutoon.navigationStore.currentLang
    });
    
    // Render from the labeled group
    await this.renderFromLabel(label);
  }

  /** @returns {Promise<void>} */
  async cleanup() {
    // Cleanup group handler
    if (this.groupHandler) {
      await this.groupHandler.cleanup();
    }

    console.log('ChapterView cleaned up');
  }
}

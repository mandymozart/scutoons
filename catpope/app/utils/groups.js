/**
 * @typedef {import('../types').Group} Group
 * @typedef {import('../types').GroupRef} GroupRef
 * @typedef {import('../types').Renderer} Renderer
 */

import { loadJSON, castGroupTypes } from './api.js';

/**
 * Populate groups by loading external group files
 * @param {(Group|GroupRef)[]} groups - Array of groups or group references
 * @param {string} lang - Language code
 * @param {number} chapterNumber - Chapter number for path resolution
 * @returns {Promise<Group[]>} Array of populated groups
 */
export async function populateGroups(groups, lang, chapterNumber) {
  const populatedGroups = [];
  
  for (const item of groups) {
    // Type guard: check if item is GroupRef (has src property)
    if ('src' in item && item.src) {
      // External group reference
      console.log(`Loading external group from: ${item.src}`);
      try {
        const groupData = await loadJSON(`api/static/catpope/${lang}/${item.src}`);
        // Cast string renderer/interaction values to enum types
        populatedGroups.push(castGroupTypes(groupData));
      } catch (error) {
        console.error(`Failed to load group from ${item.src}:`, error);
      }
    } else if ('renderer' in item) {
      // Inline group (already cast in loadChapter)
      populatedGroups.push(/** @type {Group} */ (item));
    }
  }
  
  return populatedGroups;
}

/**
 * Build label-to-group map for fast lookup
 * @param {Group[]} groups - Array of groups
 * @returns {Map<string, {group: Group, index: number}>}
 */
export function buildGroupMap(groups) {
  const map = new Map();
  groups.forEach((group, index) => {
    if (group.label) {
      map.set(group.label, { group, index });
    }
  });
  console.log(`Built group map with ${map.size} labeled groups`);
  return map;
}

/**
 * Get label of first group
 * @param {Group[]} groups - Array of groups
 * @returns {string|null}
 */
export function getFirstGroupLabel(groups) {
  const firstGroup = groups.find(g => g.label);
  return firstGroup?.label || null;
}

/**
 * Get group info by label from map (O(1) lookup)
 * @param {Map<string, {group: Group, index: number}>} groupMap - Group map
 * @param {string} label - Label to find
 * @returns {{group: Group, index: number}|null}
 */
export function getGroupByLabel(groupMap, label) {
  return groupMap.get(label) || null;
}

/**
 * Check if group index is the last group
 * @param {number} index - Current group index
 * @param {number} totalGroups - Total number of groups
 * @returns {boolean}
 */
export function isLastGroup(index, totalGroups) {
  return index === totalGroups - 1;
}

/**
 * Check if group has events
 * @param {Group} group - Group to check
 * @returns {boolean}
 */
export function hasEvents(group) {
  return Boolean(group.events && group.events.length > 0);
}

/**
 * Check if group has interaction
 * @param {Group} group - Group to check
 * @returns {boolean}
 */
export function isInteractiveGroup(group) {
  return Boolean(group.interaction);
}

/**
 * Collect consecutive non-interactive groups starting from index
 * @param {Group[]} groups - Array of groups
 * @param {number} startIndex - Starting index
 * @returns {{groups: Group[], nextInteractionIndex: number|null}}
 */
export function collectNonInteractiveGroups(groups, startIndex) {
  const collectedGroups = [];
  let currentIndex = startIndex;
  
  while (currentIndex < groups.length) {
    const group = groups[currentIndex];
    
    // If this group has interaction, stop collecting
    if (isInteractiveGroup(group)) {
      // If we haven't collected any groups yet, this IS our interaction
      if (collectedGroups.length === 0) {
        return {
          groups: [],
          nextInteractionIndex: currentIndex
        };
      }
      // Otherwise, return what we've collected
      return {
        groups: collectedGroups,
        nextInteractionIndex: currentIndex
      };
    }
    
    // Add non-interactive group
    collectedGroups.push(group);
    currentIndex++;
  }
  
  // No more interactions found
  return {
    groups: collectedGroups,
    nextInteractionIndex: null
  };
}

/**
 * Flatten group slices for rendering
 * @param {Group[]} groups - Array of groups to flatten
 * @returns {Array} Flattened slices
 */
export function flattenGroupSlices(groups) {
  const flattened = [];
  for (const group of groups) {
    if (group.slices && Array.isArray(group.slices)) {
      flattened.push(...group.slices);
    }
  }
  return flattened;
}

/**
 * Get renderer type for group (required field)
 * @param {Group} group - Group to get renderer for
 * @returns {Renderer}
 */
export function getGroupRenderer(group) {
  if (!group.renderer) {
    throw new Error(`Group "${group.label}" missing required "renderer" field`);
  }
  return group.renderer;
}

/**
 * Collect consecutive groups with the same interaction type
 * Used for stroll where multiple groups are part of one interaction
 * @param {Group[]} groups - Array of groups
 * @param {number} startIndex - Starting index
 * @param {string} interactionType - Interaction type to collect
 * @returns {{groups: Group[], nextIndex: number|null}}
 */
export function collectInteractionGroups(groups, startIndex, interactionType) {
  const collectedGroups = [];
  let currentIndex = startIndex;

  while (currentIndex < groups.length) {
    const group = groups[currentIndex];

    // Stop if this group has a different interaction or no interaction
    if (group.interaction !== interactionType) {
      break;
    }

    // Add group with matching interaction
    collectedGroups.push(group);
    currentIndex++;
  }

  return {
    groups: collectedGroups,
    nextIndex: currentIndex < groups.length ? currentIndex : null
  };
}

/**
 * @typedef {import('./types').IGameStore} IGameStore
 * @typedef {import('./types').GameState} GameState
 */

/**
 * TODO:
 * Objective: global gamestore which is shared between all toons
 * use as a save game to track the progress of the user (id by qr code)
 * I want to persist this in a database, so that the user can continue where he left off
 * first implementation is the localstorage.
 * second i need to write a file when used in an electron app
 * third i want to sync that gamestate with a mysql db, and this would require
 * a new api endpoint. (ask me for db access, i got a mysql basic setup in php on a different compositiory,
 * where I handle the membership and qr code validations)
 *
 * the game state is for a single toon. the db stores the gamestate for each toon
 * so eventually the savegame is a json string in the db
 * in a table called savegame with a single occurance of each toon for qr code.
 * so the savegame has an key pair qrCode: savegame that is unique.
 *
 *
 */

/**
 * @implements {IGameStore}
 */
export class GameStore {
  qrCode = null; // get from membershipStore
  /** @type {GameState} */
  state = {
    toonId: null,
    progress: {}, // track chapters, scenes, labels, songs, videos, playback times, etc.
    lastCheckpoint: null,
    variables: {}, // stores all game variables (choices, scores like cucumbers, etc.)
    updated_at: null,
    created_at: null,
    platform: "web",
  };

  /** @param {string} toonId */
  constructor(toonId) {
    this.state.toonId = toonId;
    this.state.platform = this.detectPlatform();
    this.loadState();
    
    // Set created_at if this is a new state
    if (!this.state.created_at) {
      this.state.created_at = new Date().toISOString();
      this.saveState();
    }
  }

  loadState() {
    const savedState = localStorage.getItem(`gameState-${this.state.toonId}`);
    if (savedState) {
      this.state = JSON.parse(savedState);
    }
  }

  saveState() {
    localStorage.setItem(`gameState-${this.state.toonId}`, JSON.stringify(this.state));
  }


  detectPlatform() {
    if (typeof window === "undefined") {
      return "app";
    }
    return "web";
  }

  /**
   * @param {string} key
   * @returns {string | number | undefined}
   */
  get(key) {
    return this.state.variables[key];
  }

  /**
   * @param {string} key
   * @param {string | number} value
   * @returns {void}
   */
  set(key, value) {
    this.state.variables[key] = value;
    this.state.updated_at = new Date().toISOString();
    this.saveState();
  }

  /**
   * @param {string} name
   * @returns {string | number | undefined}
   */
  getVariable(name) {
    return this.state.variables[name];
  }

  /**
   * @param {string} name
   * @param {string | number} value
   * @returns {void}
   */
  setVariable(name, value) {
    this.state.variables[name] = value;
    this.state.updated_at = new Date().toISOString();
    this.saveState();
  }
}

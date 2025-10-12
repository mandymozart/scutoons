/**
 * PlayerView - Music player
 */
import { loadSongs } from '../utils/api.js';
import { initPlayer } from '../utils/player.js';

export class PlayerView {
  constructor(scutoon) {
    this.scutoon = scutoon;
  }

  async render() {
    this.scutoon.navigationStore.setView('player');
    this.scutoon.navigationStore.updateURL({ view: 'player' });

    const songs = await loadSongs(this.scutoon.toonName, this.scutoon.lang);

    if (this.scutoon.contentEl) {
      this.scutoon.contentEl.innerHTML = `
      <div class="view player-view">
        <div class="c-container">
          <div class="music-container">
            <section class="music-player">
              <h1 class="music-player__title"></h1>
              <h2 class="music-player__author">Cat Pope</h2>
              <div class="music-time">
                <p class="music-time__current"></p>
                <p class="music-time__last"></p>
              </div>
              <div class="music-bar" id="progress">
                <div id="length"></div>
              </div>
              <div class="music-order">
                <div class="music-order__loop is-loop" id="loop">
                  <img src="https://snowleo208.github.io/100-Days-of-Code/7.%20Music%20Player/img/loop.svg" alt="Loop music">
                </div>
                <div class="music-order__shuffle" id="shuffle">
                  <img src="https://snowleo208.github.io/100-Days-of-Code/7.%20Music%20Player/img/shuffle.svg" alt="Shuffle music">
                </div>
              </div>
              <div class="music-control">
                <div class="arrow left" id="prev">
                <img src="https://snowleo208.github.io/100-Days-of-Code/7.%20Music%20Player/img/arrow_left.svg" alt="Previous Music">
              </div>
                <div class="music-control__play" id="play">
                  <img src="https://snowleo208.github.io/100-Days-of-Code/7.%20Music%20Player/img/play.svg" alt="Play" class="play">
                </div>
                <div class="arrow right" id="next">
                <img src="https://snowleo208.github.io/100-Days-of-Code/7.%20Music%20Player/img/arrow_right.svg" alt="Next Music">
              </div>
              </div>
            </section>
          </div>
        </div>
      </div>
      `;

      // Initialize the player after DOM is ready
      initPlayer(songs);
    }
  }
}

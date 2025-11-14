/**
 * @typedef {import('./types').ICreditsView} ICreditsView
 * @typedef {import('../types').FullCredits} FullCredits
 */

import { loadCredits } from '../utils/api.js';
import { escapeHtml, escapeAndJoin } from '../utils/template.js';

/** @implements {ICreditsView} */
export class CreditsView {
  constructor(scutoon) {
    this.scutoon = scutoon;
  }

  async render() {
    const credits = await loadCredits(this.scutoon.navigationStore.currentLang);

    if (this.scutoon.contentEl) {
      this.scutoon.contentEl.innerHTML = `
      <div class="view credits-view">
        <h1>Credits</h1>
        
        <section>
        <h2>General Crew</h2>
        <table>
          ${credits.general.map(person => `
            <tr><td>${escapeHtml(person.name)}</td><td>${escapeAndJoin(person.roles)}</td></tr>
          `).join('')}
        </table>
        </section>
        
        <section>
        <h2>Songs</h2>
        ${credits.songs.map(song => `
          <div class="song-credits">
            <h3>${escapeHtml(song.title)}</h3>
            <table>
              ${song.credits.map(person => `
                <tr><td>${escapeHtml(person.name)}</td><td>${escapeAndJoin(person.roles)}</td></tr>
              `).join('')}
            </table>
          </div>
        `).join('')}
        </section>
        
        <section>
        <h2>Performance Casts</h2>
        ${credits.performances.map(cast => `
          <div class="cast-info">
            <h3>${escapeHtml(cast.name)}</h3>
            <p><strong>Venue:</strong> ${escapeHtml(cast.venue)}</p>
            <p><strong>Date:</strong> ${escapeHtml(cast.date)}</p>
            ${cast.conductor ? `<p><strong>Conductor:</strong> ${escapeHtml(cast.conductor)}</p>` : ''}
            <table>
              ${cast.members.map(member => `
                <tr><td>${escapeHtml(member.name)}</td><td>${escapeAndJoin(member.roles)}</td></tr>
              `).join('')}
            </table>
          </div>
        `).join('')}
        </section>
        
        <section>
        <h2>Special Thanks</h2>
        <p>${escapeAndJoin(credits.specialThanks)}</p>
        </section>
      </div>
      `;
    }
  }
}

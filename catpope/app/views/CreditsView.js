/**
 * CreditsView - Full credits page
 */
import { loadCredits } from '../utils/api.js';
import { escapeHtml, escapeAndJoin } from '../utils/template.js';

export class CreditsView {
  constructor(scutoon) {
    this.scutoon = scutoon;
  }

  async render() {
    this.scutoon.navigationStore.setView('credits');
    this.scutoon.navigationStore.updateURL({ view: 'credits' });

    const credits = await loadCredits(this.scutoon.lang);

    if (this.scutoon.contentEl) {
      this.scutoon.contentEl.innerHTML = `
      <div class="view credits-view">
        <h1>Credits</h1>
        
        <section>
        <h2>Crew</h2>
        <table>
          ${credits.crew.map(person => `
            <tr><td>${escapeHtml(person.name)}</td><td>${escapeHtml(person.role)}</td></tr>
          `).join('')}
        </table>
        </section>
        
        <section>
        <h2>Choir</h2>
        <p><strong>${escapeHtml(credits.choir.name)}</strong></p>
        </section>
        
        <section>
        <h2>Performance Casts</h2>
        ${credits.casts.map(cast => `
          <div class="cast-info">
            <h3>${escapeHtml(cast.name)}</h3>
            <p><strong>Venue:</strong> ${escapeHtml(cast.venue)}</p>
            <p><strong>Date:</strong> ${escapeHtml(cast.date)}</p>
            ${cast.conductor ? `<p><strong>Conductor:</strong> ${escapeHtml(cast.conductor)}</p>` : ''}
            <table>
              ${cast.members.map(member => `
                <tr><td>${escapeHtml(member.name)}</td><td>${escapeHtml(member.voice)}</td></tr>
              `).join('')}
            </table>
          </div>
        `).join('')}
        </section>
        
        <section>
        <h2>Songs</h2>
        ${credits.songs.map(song => `
          <div class="song-credits">
            <h3>${escapeHtml(song.title)}</h3>
            <table>
              ${song.credits.text ? `<tr><td>Text</td><td>${escapeAndJoin(song.credits.text)}</td></tr>` : ''}
              ${song.credits.composition ? `<tr><td>Composition</td><td>${escapeAndJoin(song.credits.composition)}</td></tr>` : ''}
              ${song.credits.production ? `<tr><td>Production</td><td>${escapeAndJoin(song.credits.production)}</td></tr>` : ''}
              ${song.credits.recording_engineer ? `<tr><td>Recording Engineer</td><td>${escapeAndJoin(song.credits.recording_engineer)}</td></tr>` : ''}
              ${song.credits.interpretation ? `<tr><td>Interpretation</td><td>${escapeAndJoin(song.credits.interpretation)}</td></tr>` : ''}
              ${song.credits.musicians ? `<tr><td>Musicians</td><td>${escapeAndJoin(song.credits.musicians)}</td></tr>` : ''}
              ${song.credits.choir ? `<tr><td>Choir</td><td>${escapeAndJoin(song.credits.choir)}</td></tr>` : ''}
            </table>
          </div>
        `).join('')}
        </section>
        
        <section>
        <h2>Special Thanks</h2>
        <p>${escapeAndJoin(credits.special_thanks)}</p>
        </section>
      </div>
      `;
    }
  }
}

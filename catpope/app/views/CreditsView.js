/**
 * CreditsView - Full credits page
 */
import { loadCredits } from '../utils/api.js';

export class CreditsView {
  constructor(scutoon) {
    this.scutoon = scutoon;
  }

  async render() {
    this.scutoon.navigationStore.setView('credits');
    this.scutoon.navigationStore.updateURL({ view: 'credits' });

    const credits = await loadCredits(this.scutoon.toonName, this.scutoon.lang);

    if (this.scutoon.contentEl) {
      this.scutoon.contentEl.innerHTML = `
      <div class="view credits-view">
        <h1>Credits</h1>
        
        <section>
        <h2>Crew</h2>
        <table>
          ${credits.crew.map(person => `
            <tr><td>${person.name}</td><td>${person.role}</td></tr>
          `).join('')}
        </table>
        </section>
        
        <section>
        <h2>Choir</h2>
        <p><strong>${credits.choir.name}</strong></p>
        </section>
        
        <section>
        <h2>Performance Casts</h2>
        ${credits.casts.map(cast => `
          <div class="cast-info">
            <h3>${cast.name}</h3>
            <p><strong>Venue:</strong> ${cast.venue}</p>
            <p><strong>Date:</strong> ${cast.date}</p>
            ${cast.conductor ? `<p><strong>Conductor:</strong> ${cast.conductor}</p>` : ''}
            <table>
              ${cast.members.map(member => `
                <tr><td>${member.name}</td><td>${member.voice}</td></tr>
              `).join('')}
            </table>
          </div>
        `).join('')}
        </section>
        
        <section>
        <h2>Songs</h2>
        ${credits.songs.map(song => `
          <div class="song-credits">
            <h3>${song.title}</h3>
            <table>
              ${song.credits.text ? `<tr><td>Text</td><td>${song.credits.text.join(', ')}</td></tr>` : ''}
              ${song.credits.composition ? `<tr><td>Composition</td><td>${song.credits.composition.join(', ')}</td></tr>` : ''}
              ${song.credits.production ? `<tr><td>Production</td><td>${song.credits.production.join(', ')}</td></tr>` : ''}
              ${song.credits.recording_engineer ? `<tr><td>Recording Engineer</td><td>${song.credits.recording_engineer.join(', ')}</td></tr>` : ''}
              ${song.credits.interpretation ? `<tr><td>Interpretation</td><td>${song.credits.interpretation.join(', ')}</td></tr>` : ''}
              ${song.credits.musicians ? `<tr><td>Musicians</td><td>${song.credits.musicians.join(', ')}</td></tr>` : ''}
              ${song.credits.choir ? `<tr><td>Choir</td><td>${song.credits.choir.join(', ')}</td></tr>` : ''}
            </table>
          </div>
        `).join('')}
        </section>
        
        <section>
        <h2>Special Thanks</h2>
        <p>${credits.special_thanks.join(', ')}</p>
        </section>
      </div>
      `;
    }
  }
}

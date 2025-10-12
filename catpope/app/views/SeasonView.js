/**
 * SeasonView - Season details page
 */
import { loadSeason } from '../utils/api.js';

export class SeasonView {
  constructor(scutoon) {
    this.scutoon = scutoon;
  }

  async render(seasonNumber = 1) {
    const data = await loadSeason(this.scutoon.toonName, this.scutoon.lang, seasonNumber);
    this.scutoon.season = data;
    this.scutoon.navigationStore.setView('season', { season: seasonNumber });

    // Update URL
    this.scutoon.navigationStore.updateURL({ view: 'season', season: seasonNumber });

    const content = `
      <div class="season-view">
        <h1>Staffel ${data.number}: ${data.title}</h1>
        <p class="authors">von ${data.authors ? data.authors.join(' & ') : ''}</p>
        
        <p class="description">${data.description}</p>
        ${data.cover ? `
          <div class="cover-images">
            ${data.cover.square ? `<img src="${data.cover.square}" alt="Cover" class="cover-square">` : ''}
            ${data.cover.portrait ? `<img src="${data.cover.portrait}" alt="Cover" class="cover-portrait">` : ''}
          </div>
        ` : ''}
        ${data.credits ? `
          <div class="credits">
            <h3>Credits</h3>
            <p><strong>Künstler:</strong> ${data.credits.artist ? data.credits.artist.join(', ') : ''}</p>
            <p><strong>Autor:</strong> ${data.credits.writer}</p>
            <p><strong>Musik:</strong> ${data.credits.music ? data.credits.music.join(', ') : ''}</p>
            <p><strong>Sound:</strong> ${data.credits.sound ? data.credits.sound.join(', ') : ''}</p>
          </div>
        ` : ''}
        <button onclick="scutoon.navigateTo('credits')">Complete Credits</button>
        <div class="navigation-buttons">
          <button onclick="scutoon.navigateTo('toon')" class="btn-back">Zurück zur Übersicht</button>
          <button onclick="scutoon.navigateTo('chapter', {chapter: 1})" class="btn-chapter">Kapitel 1 lesen</button>
        </div>
      </div>
    `;

    if (this.scutoon.contentEl) this.scutoon.contentEl.innerHTML = content;

    return data;
  }
}

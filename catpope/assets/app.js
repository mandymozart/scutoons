/**
 * SCUTOONS Base App, used by all SCUTOONS
 */
class Scutoon {
  toonName;
  lang;
  toon;
  episode;
  season;
  currentView = 'toon'; // toon, season, chapter
  currentChapter = 1;
  currentSeason = 1;
  
  constructor(toonName, lang) {
    this.toonName = toonName;
    this.lang = lang;
  }

  async loadToon() {
    return fetch(`${this.toonName}/${this.lang}/toon.json`)
      .then(response => response.json())
      .then(data => {
        console.log('Toon data:', data);
        this.toon = data;
        return data;
      });
  }

  async loadEpisode(number) {
    return fetch(`${this.toonName}/${this.lang}/chapter${number}.json`)
      .then(response => response.json())
      .then(data => {
        console.log('Episode data:', data);
        this.episode = data;
        this.currentChapter = number;
        return data;
      });
  }

  async loadSeason(number) {
    return fetch(`${this.toonName}/${this.lang}/season${number}.json`)
      .then(response => response.json())
      .then(data => {
        console.log('Season data:', data);
        this.season = data;
        this.currentSeason = number;
        return data;
      });
  }

  // Template helper to replace placeholders with data
  renderTemplate(template, data) {
    return template.replace(/\{\{(\w+)\}\}/g, (match, key) => {
      if (key === 'authors' && Array.isArray(data[key])) {
        return data[key].join(', ');
      }
      return data[key] || '';
    });
  }

  // Render toon view (main/overview with chapters list)
  async renderToonView() {
    const data = await this.loadToon();
    this.currentView = 'toon';
    
    // For now we have one chapter, but this can be expanded
    // You could make this dynamic by checking which chapter files exist
    const availableChapters = [
      { number: 1, title: "Das Erwachen", status: "available" }
      // Add more chapters here as they become available
      // { number: 2, title: "Chapter 2 Title", status: "coming-soon" }
    ];
    
    const chaptersHtml = availableChapters.map(chapter => {
      if (chapter.status === 'available') {
        return `
          <div class="chapter-card available">
            <h3>Kapitel ${chapter.number}</h3>
            <p>${chapter.title}</p>
            <button onclick="scutoon.renderChapterView(${chapter.number})" class="btn-read">Lesen</button>
          </div>
        `;
      } else {
        return `
          <div class="chapter-card coming-soon">
            <h3>Kapitel ${chapter.number}</h3>
            <p>${chapter.title}</p>
            <span class="status">Demnächst</span>
          </div>
        `;
      }
    }).join('');
    
    const content = `
      <div class="toon-overview">
        <div class="toon-header">
          <h1>${data.title}</h1>
          <p class="authors">von ${data.authors ? data.authors.join(' & ') : ''}</p>
          <p class="description">${data.description}</p>
          <div class="meta-info">
            <span class="language">🌍 ${data.language.toUpperCase()}</span>
            <span class="release">📅 ${data.release}</span>
          </div>
        </div>
        
        <div class="chapters-section">
          <h2>Verfügbare Kapitel</h2>
          <div class="chapters-grid">
            ${chaptersHtml}
          </div>
        </div>
      </div>
    `;
    
    const contentDiv = document.querySelector('.content');
    if (contentDiv) {
      contentDiv.innerHTML = content;
      contentDiv.classList.remove('chapter-mode');
    }
    
    return data;
  }

  // Render season view
  async renderSeasonView(seasonNumber = 1) {
    const data = await this.loadSeason(seasonNumber);
    this.currentView = 'season';
    
    const content = `
      <div class="season-view">
        <h1>Staffel ${data.episode}: ${data.title}</h1>
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
        <div class="navigation-buttons">
          <button onclick="scutoon.renderToonView()" class="btn-back">Zurück zur Übersicht</button>
          <button onclick="scutoon.renderChapterView(1)" class="btn-chapter">Kapitel 1 lesen</button>
        </div>
      </div>
    `;
    
    const contentDiv = document.querySelector('.content');
    if (contentDiv) contentDiv.innerHTML = content;
    
    return data;
  }

  // Render chapter view (scrollable webtoon)
  async renderChapterView(chapterNumber = 1) {
    const data = await this.loadEpisode(chapterNumber);
    this.currentView = 'chapter';
    
    let slicesHtml = '';
    if (data.slices && data.slices.length > 0) {
      slicesHtml = data.slices.map((slice, index) => `
        <div class="webtoon-slice" data-slice="${index + 1}">
          <img src="${slice.image}" alt="Slice ${index + 1}" loading="lazy">
        </div>
      `).join('');
    }
    
    const content = `
      <div class="chapter-view">
        <div class="webtoon-container">
          ${slicesHtml}
        </div>
      </div>
    `;
    
    const contentDiv = document.querySelector('.content');
    if (contentDiv) {
      contentDiv.innerHTML = content;
      // Make content div fullscreen for chapter view
      contentDiv.classList.add('chapter-mode');
    }
    
    return data;
  }

  // Helper function to scroll to top
  scrollToTop() {
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  // Initialize the app with a specific view
  async init(initialView = 'toon', initialChapter = 1, initialSeason = 1) {
    switch(initialView) {
      case 'season':
        await this.renderSeasonView(initialSeason);
        break;
      case 'chapter':
        await this.renderChapterView(initialChapter);
        break;
      default:
        await this.renderToonView();
    }
  }
}

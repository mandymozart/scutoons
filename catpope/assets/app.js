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
    this.lastScrollTop = 0;
    this.initScrollHandler();
  }

  // Initialize scroll handler for header
  initScrollHandler() {
    let lastScrollTop = 0;
    let scrollThreshold = 5; // Minimum scroll amount to trigger
    const header = document.querySelector('header');
    
    window.addEventListener('scroll', () => {
      const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
      
      // Skip if scroll change is too small
      if (Math.abs(scrollTop - lastScrollTop) <= scrollThreshold) {
        return;
      }
      
      if (scrollTop > lastScrollTop && scrollTop > 80) {
        // Scrolling down & past header height
        header?.classList.add('header-hidden');
      } else {
        // Scrolling up
        header?.classList.remove('header-hidden');
      }
      
      lastScrollTop = scrollTop;
    });
  }

  // Simple router - update URL without page reload
  updateURL(params) {
    const currentParams = new URLSearchParams(window.location.search);
    
    // Update or set new parameters
    Object.keys(params).forEach(key => {
      if (params[key] !== undefined && params[key] !== null) {
        currentParams.set(key, params[key]);
      }
    });
    
    // Update URL without reload
    const newURL = window.location.pathname + '?' + currentParams.toString();
    window.history.pushState({ ...params }, '', newURL);
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

  async loadChapter(chapterNumber) {
    return fetch(`${this.toonName}/${this.lang}/chapter${chapterNumber}.json`)
      .then(response => response.json())
      .then(data => {
        console.log('Chapter data:', data);
        this.episode = data;
        this.currentChapter = chapterNumber;
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
    
    // Update URL
    this.updateURL({ view: 'toon' });
    
    // Load chapter details based on the number of chapters
    const availableChapters = [];
    const chapterCount = data.chapters || 0;
    
    for (let i = 1; i <= chapterCount; i++) {
      try {
        const chapterData = await this.loadChapter(i);
        availableChapters.push({
          number: i,
          title: chapterData.title || `Kapitel ${i}`,
          description: chapterData.description || '',
          status: 'available'
        });
      } catch (error) {
        console.error(`Failed to load chapter ${i}:`, error);
        // Still show the chapter card but mark it as unavailable
        availableChapters.push({
          number: i,
          title: `Kapitel ${i}`,
          description: 'Nicht verfügbar',
          status: 'error'
        });
      }
    }
    
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
    
    // Update URL
    this.updateURL({ view: 'season', season: seasonNumber });
    
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
    const data = await this.loadChapter(chapterNumber);
    this.currentView = 'chapter';
    
    // Update URL
    this.updateURL({ view: 'chapter', chapter: chapterNumber });
    
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

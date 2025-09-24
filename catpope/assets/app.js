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
  membership = null; // Store membership data
  
  constructor(toonName, lang) {
    this.toonName = toonName;
    this.lang = lang;
    this.lastScrollTop = 0;
    this.initScrollHandler();
    this.initMembership(); // Initialize membership system
  }

  // Initialize membership system
  initMembership() {
    const storedMembership = localStorage.getItem('scutoons_membership');
    if (storedMembership) {
      try {
        this.membership = JSON.parse(storedMembership);
        console.log('Found stored membership:', this.membership);
        if (this.isMembershipExpired(this.membership)) {
          console.log('Stored membership has expired, removing...');
          this.clearMembership();
        }
      } catch (error) {
        console.error('Error parsing stored membership:', error);
        this.clearMembership();
      }
    }
  }

  // Check if membership has expired
  isMembershipExpired(membership) {
    if (!membership) return true;
    
    // Calculate expiry based on created_at or validatedAt
    // Default: memberships are valid for 30 days from creation/validation
    const baseDate = membership.created_at || membership.validatedAt;
    if (!baseDate) return false; // If no date, consider it valid
    
    const createdDate = new Date(baseDate);
    const expiryDate = new Date(createdDate);
    expiryDate.setDate(expiryDate.getDate() + 30); // 30 days validity
    
    return expiryDate < new Date();
  }

  // Store membership to localStorage
  storeMembership(membershipData) {
    try {
      if (membershipData.type === 'membership') {
        this.membership = membershipData;
        localStorage.setItem('scutoons_membership', JSON.stringify(membershipData));
        console.log('Membership stored successfully:', membershipData);
        return true;
      }
      return false;
    } catch (error) {
      console.error('Error storing membership:', error);
      return false;
    }
  }

  // Clear membership from localStorage
  clearMembership() {
    this.membership = null;
    localStorage.removeItem('scutoons_membership');
    console.log('Membership cleared');
  }

  // Check if user has valid membership
  hasValidMembership() {
    if (!this.membership) return false;
    if (this.isMembershipExpired(this.membership)) {
      this.clearMembership();
      return false;
    }
    return true;
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
          status: chapterData.status || 'unknown'
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
      if (chapter.status === 'published') {
        return `
          <div class="chapter-card published">
          <div class="chapter-number">#${chapter.number}</div>
          <div class="chapter-image"><img src="${data.thumbnail || 'assets/images/default-thumbnail.jpg'}" alt="Kapitel ${chapter.number} Thumbnail" loading="lazy"></div>
            <h3>Kapitel ${chapter.number}</h3>
            <p>${chapter.title}</p>
            <button onclick="scutoon.renderChapterView(${chapter.number})" class="btn-read">Lesen</button>
          </div>
        `;
      } 
      if (chapter.status === 'coming-soon') {
        return `
          <div class="chapter-card coming-soon">
            <h3>Kapitel ${chapter.number}</h3>
            <p>${chapter.title}</p>
            <span class="status">Demnächst</span>
          </div>
        `;
      } 
      // skip error or unknown or draft
    }).join('');
    
    const content = `
      <div class="toon-overview">
        <div class="toon-header">
          <img src="${data.cover.portrait}" alt="Cover" class="Cover" />
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
    // Check membership before rendering chapter
    if (!this.hasValidMembership()) {
      this.renderMembershipRequired(chapterNumber);
      return null;
    }

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

  // Render membership required message
  renderMembershipRequired(chapterNumber) {
    const content = `
      <div class="membership-required">
        <div class="membership-card">
          <h2>🔒 Mitgliedschaft erforderlich</h2>
          <p>Um Kapitel ${chapterNumber} zu lesen, benötigen Sie eine gültige SCUTOONS-Mitgliedschaft.</p>
          
          <div class="membership-info">
            <h3>So erhalten Sie Zugang:</h3>
            <ol>
              <li>Scannen Sie einen gültigen SCUTOONS QR-Code</li>
              <li>Sie werden automatisch zu dieser Seite weitergeleitet</li>
              <li>Ihre Mitgliedschaft wird gespeichert und Sie können alle Kapitel lesen</li>
            </ol>
          </div>
          
          <div class="membership-actions">
            <button onclick="scutoon.checkForNewCode()" class="btn-check">
              🔄 Erneut prüfen
            </button>
            <button onclick="scutoon.renderToonView()" class="btn-back">
              ← Zurück zur Übersicht
            </button>
          </div>
          
          ${this.membership && this.membership.type !== 'membership' ? `
            <div class="membership-error">
              <p>⚠️ Der gescannte QR-Code ist kein gültiger Mitgliedschaftscode.</p>
            </div>
          ` : ''}
          
          ${this.membership && this.membership.requires_claim && !this.membership.is_claimed ? `
            <div class="membership-error">
              <p>⚠️ Diese Mitgliedschaft muss zuerst auf qr.scutoons.com aktiviert werden.</p>
              <a href="https://qr.scutoons.com/claim?code=${this.membership.code}" target="_blank" class="btn-claim">
                Mitgliedschaft aktivieren →
              </a>
            </div>
          ` : ''}
        </div>
      </div>
    `;
    
    const contentDiv = document.querySelector('.content');
    if (contentDiv) {
      contentDiv.innerHTML = content;
      contentDiv.classList.remove('chapter-mode');
    }
  }

  // Validate QR code with the API
  async validateQRCode(code) {
    if (!code) {
      console.log('No QR code provided');
      return null;
    }

    try {
      const response = await fetch('https://qr.scutoons.com/api/validate/', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({ code: code })
      });

      const data = await response.json();
      
      if (data.status === 'success' && data.qr) {
        console.log('QR validation successful:', data.qr);
        
        // Check if it's a membership type
        if (data.qr.type === 'membership') {
          // Check if membership requires claiming and hasn't been claimed
          if (data.qr.requires_claim && !data.qr.is_claimed) {
            console.log('Membership requires claiming first');
            return { 
              success: false, 
              message: 'Membership muss zuerst aktiviert werden', 
              requiresClaim: true,
              data: data.qr 
            };
          }
          
          // Store the membership with validation timestamp
          const membershipData = {
            ...data.qr,
            code: code,
            validatedAt: new Date().toISOString()
          };
          
          this.storeMembership(membershipData);
          return { success: true, membership: membershipData };
        } else {
          console.log('QR code is not a membership type:', data.qr.type);
          return { success: false, message: 'Not a membership code', data: data.qr };
        }
      } else {
        console.error('QR validation failed:', data.message);
        return { success: false, message: data.message || 'Validation failed' };
      }
    } catch (error) {
      console.error('Error validating QR code:', error);
      return { success: false, message: 'Network error', error };
    }
  }

  // Get membership expiry date based on creation/validation
  getMembershipExpiryDate(membership) {
    if (!membership) return null;
    
    const baseDate = membership.created_at || membership.validatedAt;
    if (!baseDate) return null;
    
    const createdDate = new Date(baseDate);
    const expiryDate = new Date(createdDate);
    expiryDate.setDate(expiryDate.getDate() + 30); // 30 days validity
    
    return expiryDate.toISOString();
  }

  // Check and process QR code from URL
  async processQRCodeFromURL() {
    const urlParams = new URLSearchParams(window.location.search);
    const code = urlParams.get('code');
    
    if (code) {
      console.log('Found QR code in URL:', code);
      
      if (this.membership && this.membership.code === code) {
        console.log('Code already validated and stored');
        return this.membership;
      }
      
      const result = await this.validateQRCode(code);
      
      // Clean up URL by removing the code parameter
      urlParams.delete('code');
      const newURL = window.location.pathname + 
        (urlParams.toString() ? '?' + urlParams.toString() : '');
      window.history.replaceState({}, '', newURL);
      
      return result;
    }
    
    return null;
  }

  // Check for new QR code (manual trigger)
  async checkForNewCode() {
    const urlParams = new URLSearchParams(window.location.search);
    const code = urlParams.get('code');
    
    if (code) {
      const result = await this.validateQRCode(code);
      if (result && result.success) {
        // If validation successful, try to render the chapter again
        const chapter = parseInt(urlParams.get('chapter')) || 1;
        this.renderChapterView(chapter);
      } else {
        alert('Kein gültiger Mitgliedschaftscode gefunden.');
      }
    } else {
      alert('Kein QR-Code in der URL gefunden. Bitte scannen Sie einen gültigen SCUTOONS QR-Code.');
    }
  }

  // Get membership status (for debugging/display)
  getMembershipStatus() {
    if (!this.membership) {
      return { status: 'none', message: 'Keine Mitgliedschaft' };
    }
    
    if (this.isMembershipExpired(this.membership)) {
      return { status: 'expired', message: 'Mitgliedschaft abgelaufen', data: this.membership };
    }
    
    const expiryDate = this.getMembershipExpiryDate(this.membership);
    return { 
      status: 'active', 
      message: 'Aktive Mitgliedschaft',
      data: this.membership,
      validUntil: expiryDate,
      claimedBy: this.membership.email || 'Unknown',
      claimedAt: this.membership.claimed_at
    };
  }

  // Debug function to test membership validation
  debugMembership() {
    const status = this.getMembershipStatus();
    console.group('🔍 Membership Debug Info');
    console.log('Status:', status.status);
    console.log('Message:', status.message);
    if (status.data) {
      console.log('Type:', status.data.type);
      console.log('Code:', status.data.code);
      console.log('Is Claimed:', status.data.is_claimed);
      console.log('Requires Claim:', status.data.requires_claim);
      console.log('Email:', status.data.email || 'Not claimed');
      console.log('Valid Until:', status.validUntil);
    }
    console.log('Has Valid Membership:', this.hasValidMembership());
    console.log('localStorage data:', localStorage.getItem('scutoons_membership'));
    console.groupEnd();
    return status;
  }

  // Helper function to scroll to top
  scrollToTop() {
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  // Initialize the app with a specific view
  async init(initialView = 'toon', initialChapter = 1, initialSeason = 1) {
    // First process any QR code from URL
    const qrResult = await this.processQRCodeFromURL();
    
    if (qrResult && qrResult.success) {
      console.log('Successfully validated membership from QR code');
    }
    
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

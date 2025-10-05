/**
 * SCUTOONS Base App, used by all SCUTOONS
 */
class Scutoon {
  toonName;
  lang;
  toon;
  chapter;
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
        // Validate that stored membership is actually a membership type
        if (!this.membership || !this.isValidMembershipType(this.membership.type) || this.isMembershipExpired(this.membership.expires_at)) {
          console.log('Invalid stored membership type, clearing...');
          this.clearMembership();
        }
      } catch (error) {
        console.error('Error parsing stored membership:', error);
        this.clearMembership();
      }
    }
  }


  // Store membership to localStorage
  storeMembership(membershipData) {
    try {
      // Double validation: only store if it's truly a  valid membership type
      // TODO: Refactor membership type validation to own function returning a boolean
      // Question nessecity of this validation
      if (membershipData && this.isValidMembershipType(membershipData.type) && membershipData.code) {
        this.membership = membershipData;
        localStorage.setItem('scutoons_membership', JSON.stringify(membershipData));
        console.log('Membership stored successfully:', membershipData);
        return true;
      } else {
        console.log('Attempted to store invalid membership:', membershipData);
        return false;
      }
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

  isValidMembershipType(type) {
    return type === 'membership' || type === 'promo';
  }

  isMembershipExpired(expires_at = null) {
    console.log(expires_at);
    if(expires_at === null) {
      return false;
    }
    const exp = new Date(expires_at.replace(' ', 'T') + 'Z'); // "YYYY-MM-DDTHH:mm:ssZ"
    return Date.now() > exp.getTime();
  }

  // Check if user has valid membership
  hasValidMembership() {
    // Must have membership object, must be type 'membership' or 'promo', and must have a code
    return !!(this.membership && !this.isMembershipExpired(this.membership.expires_at) && 
             this.isValidMembershipType(this.membership.type) && 
             this.membership.code);
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

  // Navigate to a specific view through the router
  navigateTo(view, params = {}) {
    const navigationParams = { view, ...params };
    
    // Update URL first
    this.updateURL(navigationParams);
    
    // Then render the appropriate view
    switch(view) {
      case 'season':
        this.renderSeasonView(params.season || 1);
        break;
      case 'chapter':
        this.renderChapterView(params.chapter || 1);
        break;
      case 'account':
        this.renderAccountView();
        break;
      case 'credits':
        this.renderCreditsView();
        break;
      default:
        this.renderToonView();
    }
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
        this.chapter = data;
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

    // Display download options
    let downloadHtml = ``;
    if(data.download.music && this.hasValidMembership()){
      downloadHtml = `<div class="toon-download"><a class="btn" href="${data.download.music}" target="_blank">Download as MP3</a></div>`
    } 
    
    // Load chapter details based on the number of chapters
    const availableChapters = [];
    const chapterCount = data.chapters || 0;
    
    for (let i = 1; i <= chapterCount; i++) {
      try {
        const chapterData = await this.loadChapter(i);
        availableChapters.push(chapterData);
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
      console.log(data.thumbnail, chapter.thumbnail)
      if (chapter.status === 'published') {
        return `
          <div class="chapter-item published" onclick="scutoon.navigateTo('chapter', {chapter: ${chapter.number}})">
            <div class="chapter-image"><img src="${chapter.thumbnail || 'assets/images/default-thumbnail.jpg'}" alt="Kapitel ${chapter.number} Thumbnail" loading="lazy"></div>
            <div class="chapter-title">${chapter.title}</div>
            <div class="chapter-release">${chapter.release}</div>
            <div class="chapter-number">#${chapter.number}</div>
          </div>
        `;
      } 
      if (chapter.status === 'coming-soon') {
        return `
          <div class="chapter-item coming-soon">
            <div class="chapter-image"><img src="${chapter.thumbnail || 'assets/images/default-thumbnail.jpg'}" alt="Kapitel ${chapter.number} Thumbnail" loading="lazy"></div>
            <div class="chapter-title">${chapter.title}</div>
            <div class="chapter-release">${chapter.release}</div>
            <div class="chapter-number">#${chapter.number}</div>
          </div>
        `;
      } 
      // skip error or unknown or draft
    }).join('');
    
    const content = `
      <div class="toon-overview">
        <div class="toon-header">
          <img src="${data.cover.square}" alt="Cover" class="Cover" />
          <h1>${data.title}</h1>
          <p class="description">${data.description}</p>
          ${downloadHtml}
        </div>
        
        <div class="chapters-section">
          <div class="chapters-list">
            ${chaptersHtml}
          </div>
        </div>
                  <button onclick="scutoon.navigateTo('credits')">Credits</button>
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
        <button onclick="scutoon.navigateTo('credits')">Complete Credits</button>
        <div class="navigation-buttons">
          <button onclick="scutoon.navigateTo('toon')" class="btn-back">Zurück zur Übersicht</button>
          <button onclick="scutoon.navigateTo('chapter', {chapter: 1})" class="btn-chapter">Kapitel 1 lesen</button>
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
    
    // Create placeholder HTML for images
    let slicesHtml = '';
    if (data.slices && data.slices.length > 0) {
      slicesHtml = data.slices.map((slice, index) => `
        <div class="webtoon-slice lazy" data-slice="${index + 1}">
          <div class="slice-placeholder">Loading image ${index + 1}...</div>
          <img data-src="${slice.image}" alt="Slice ${index + 1}" style="display: none;">
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
    
    // Start sequential lazy loading
    if (data.slices && data.slices.length > 0) {
      this.loadImagesSequentially();
    }
    
    return data;
  }

  // Load images one by one in sequence
  loadImagesSequentially() {
    const slices = document.querySelectorAll('.webtoon-slice.lazy');
    let currentIndex = 0;

    const loadNextImage = () => {
      if (currentIndex >= slices.length) {
        console.log('All images loaded');
        return;
      }

      const slice = slices[currentIndex];
      const img = slice.querySelector('img');
      const placeholder = slice.querySelector('.slice-placeholder');
      const src = img.getAttribute('data-src');

      if (!src) {
        currentIndex++;
        loadNextImage();
        return;
      }

      console.log(`Loading image ${currentIndex + 1}/${slices.length}`);

      // Create a new image to preload
      const tempImg = new Image();
      
      tempImg.onload = () => {
        // Image loaded successfully
        img.src = src;
        img.style.display = 'block';
        if (placeholder) {
          placeholder.style.display = 'none';
        }
        slice.classList.remove('lazy');
        slice.classList.add('loaded');
        
        console.log(`Image ${currentIndex + 1} loaded successfully`);
        currentIndex++;
        
        // Load next image
        loadNextImage();
      };

      tempImg.onerror = () => {
        // Image failed to load
        console.error(`Failed to load image ${currentIndex + 1}: ${src}`);
        if (placeholder) {
          placeholder.textContent = `Failed to load image ${currentIndex + 1}`;
          placeholder.style.color = '#e74c3c';
        }
        slice.classList.add('error');
        currentIndex++;
        
        // Continue with next image even if this one failed
        loadNextImage();
      };

      // Start loading
      tempImg.src = src;
    };

    // Start loading the first image
    loadNextImage();
  }

  // Render membership required message
  renderMembershipRequired(chapterNumber) {
    const content = `
    <div class="view membership-required-view">
      <div class="membership-required">
        <div class="membership-card">
          <h2>🔒 Mitgliedschaft erforderlich</h2>
          <p>Um Kapitel ${chapterNumber} zu lesen, benötigen Sie eine gültige SCUTOONS-Mitgliedschaft.</p>
          
          <div class="membership-info">
            <h3>So erhalts du Zugang:</h3>
            <ol>
              <li>Scannen Sie einen gültigen SCUTOONS QR-Code oder geben sie einen Code auf <a href="https://qr.scutoons.com">qr.scutoons.com</a> ein.</li>
              <li>Sie werden automatisch zu dieser Seite weitergeleitet</li>
              <li>QR Codes findest du auf allen SCUTOONS Limited Editions</li>
            </ol>
          </div>
          
          <div class="membership-actions">
            <button onclick="scutoon.navigateTo('toon')" class="btn-back">
              ← Zurück zur Übersicht
            </button>
          </div>
          
          ${this.membership && !this.isValidMembershipType(this.membership.type) ? `
            <div class="membership-error">
              <p>⚠️ Der gescannte QR-Code ist kein gültiger Mitgliedschaftscode.</p>
            </div>
          ` : ''}
        </div>
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
        console.log('QR validation successful:', data.qr, this.isMembershipExpired(data.qr.expires_at));
        
        // Check if it's a membership type
        if (this.isValidMembershipType(data.qr.type)) {
          if (this.isMembershipExpired(data.qr.expires_at)) { // Expired when NOW is AFTER expiration
            console.log('QR code is expired');
            return { success: false, message: 'QR code is expired', data: data.qr };
          }
          // Allow all membership types (claimed or unclaimed)
          // Store the membership with validation timestamp
          const membershipData = {
            ...data.qr,
            code: code,
            validated_at: new Date().toISOString(),
            expires_at: data.qr.expires_at
          };
          
          this.storeMembership(membershipData);
          console.log('Membership validated and stored');
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
    
    if (!this.isValidMembershipType(this.membership.type)) {
      return { status: 'invalid', message: 'Ungültiger Mitgliedschaftstyp', data: this.membership };
    }
    
    return { 
      status: 'active', 
      message: 'Aktive Mitgliedschaft',
      data: this.membership,
      claimedBy: this.membership.email || 'Unknown',
      claimedAt: this.membership.claimed_at,
      expires_at: this.membership.expires_at
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
      console.log('Created:', status.data.created_at);
      console.log('Claimed:', status.data.claimed_at);
    }
    console.log('Has Valid Membership:', this.hasValidMembership());
    console.log('localStorage data:', localStorage.getItem('scutoons_membership'));
    console.groupEnd();
    return status;
  }

  // Manual test to bypass attempt (for security testing)
  testBypassAttempt(fakeData) {
    console.group('🔒 Testing Bypass Attempt');
    console.log('Attempting to store:', fakeData);
    const stored = this.storeMembership(fakeData);
    console.log('Store result:', stored);
    console.log('Has valid membership after attempt:', this.hasValidMembership());
    console.groupEnd();
    return stored;
  }

  // Helper function to scroll to top
  scrollToTop() {
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  renderAccountNavItem() {
    const nav = document.querySelector('.nav');
    const accountNavItem = document.createElement('li');
    accountNavItem.classList.add('nav-item', 'account-nav');
    accountNavItem.innerHTML = `
      <a href="#" class="nav-btn" onclick="scutoon.navigateTo('account'); return false;">
        <img src="../assets/icons/account.svg" alt="Account"/>
        <span>My Account</span>
      </a>
    `;
    nav.querySelector('ul').appendChild(accountNavItem);
  }

  removeAccountNavItem() {
    const nav = document.querySelector('.nav');
    // check if account nav item exists
    if (nav.querySelector('.account-nav')) {
      nav.querySelector('.account-nav').remove();
    }
  }

  renderClaimNavItem() {
    const nav = document.querySelector('.nav');
    const claimNavItem = document.createElement('li');
    claimNavItem.classList.add('nav-item', 'claim-nav');
    claimNavItem.innerHTML = `
      <a href="https://qr.scutoons.com/" class="nav-btn">
      <img src="../assets/icons/qr.svg" alt="QR"/>
        <span>Claim Membership</span>
      </a>
    `;
    nav.querySelector('ul').appendChild(claimNavItem);
  }

  removeClaimNavItem() {
    const nav = document.querySelector('.nav');
    // check if claim nav item exists
    if (nav.querySelector('.claim-nav')) {
      nav.querySelector('.claim-nav').remove();
    }
  }

  renderAccountNavigation() {
    const nav = document.querySelector('.nav');
    // Check if membership exists
    if (this.membership) {
      this.renderAccountNavItem();
      this.removeClaimNavItem();
    } else {
      this.removeAccountNavItem();
      this.renderClaimNavItem();
    }
  }

  renderAccountView() {
    const contentDiv = document.querySelector('.content');
    if (contentDiv) {
      const expiryText = this.membership.expires_at || 'Lifetime';
      const claimedText = this.membership.claimed_at || 'Not claimed';
      
      contentDiv.innerHTML = `
      <div class="view account-view">
        <h1>My Account</h1>
        
        ${!this.membership.is_claimed ? `
        <div class="notification">
          <p>🔔 Reminder: If you claim your membership, we will be able to send you emails about updates and you can make changes to your membership! After all, you did buy this QR code.</p>
          <p><a href="https://qr.scutoons.com/?c=${this.membership.code}" target="_blank" class="btn">Claim Membership Now</a></p>
        </div>
        ` : ''}
        
        <section>
        <h2>Membership Information</h2>
        <table>
          <tr><td>Type</td><td>${this.membership.type}</td></tr>
          <tr><td>Status</td><td>${this.membership.is_claimed ? 'Claimed' : 'Unclaimed'}</td></tr>
          <tr><td>Valid Until</td><td>${expiryText}</td></tr>
          <tr><td>Code</td><td>${this.membership.code}</td></tr>
        </table>
        </section>
        
        <section>
        <h2>Timeline</h2>
        <table>
          <tr><td>Created</td><td>${this.membership.created_at}</td></tr>
          <tr><td>Claimed</td><td>${claimedText}</td></tr>
          <tr><td>Last Validated</td><td>${this.membership.validated_at}</td></tr>
        </table>
        </section>
        
        ${this.membership.email ? `
        <section>
        <h2>Account Details</h2>
        <table>
          <tr><td>Email</td><td>${this.membership.email}</td></tr>
        </table>
        </section>
        ` : ''}
        
        <section>
        <h2>Technical Information</h2>
        <table>
          ${this.membership.redirect_to ? `<tr><td>Content URL</td><td><a href="${this.membership.redirect_to}" target="_blank">${this.membership.redirect_to}</a></td></tr>` : ''}
        </table>
        </section>
      </div>
      `;
    }
  }

  async renderCreditsView(){
    const credits = await fetch(`${this.toonName}/${this.lang}/credits.json`)
      .then(response => response.json());
    
    const contentDiv = document.querySelector('.content');
    if (contentDiv) {
      contentDiv.innerHTML = `
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

  // Initialize the app with a specific view
  async init(initialView = 'toon', initialChapter = 1, initialSeason = 1) {
    // First process any QR code from URL
    const qrResult = await this.processQRCodeFromURL();
    
    if (qrResult && qrResult.success) {
      console.log('Successfully validated membership from QR code');
    }

    this.renderAccountNavigation();
    
    switch(initialView) {
      case 'season':
        await this.renderSeasonView(initialSeason);
        break;
      case 'chapter':
        await this.renderChapterView(initialChapter);
        break;
      case 'account':
        await this.renderAccountView();
        break;
      case 'credits':
        await this.renderCreditsView();
        break;
      default:
        await this.renderToonView();
    }
  }
}

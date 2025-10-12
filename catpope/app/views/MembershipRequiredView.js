/**
 * MembershipRequiredView - Locked content message
 */
export class MembershipRequiredView {
  constructor(scutoon) {
    this.scutoon = scutoon;
  }

  render(chapterNumber = null, songNumber = null) {
    const contentType = songNumber ? 'Song' : 'Kapitel';
    const contentNumber = songNumber || chapterNumber;
    const actionText = songNumber ? 'zu hören' : 'zu lesen';

    const content = `
    <div class="view membership-required-view">
      <div class="membership-required">
        <div class="membership-card">
          <h2>🔒 Mitgliedschaft erforderlich</h2>
          <p>Um ${contentType} ${contentNumber} ${actionText}, benötigen Sie eine gültige SCUTOONS-Mitgliedschaft.</p>
          
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
          
          ${this.scutoon.membershipStore.membership && !this.scutoon.membershipStore.isValidMembershipType(this.scutoon.membershipStore.membership.type) ? `
            <div class="membership-error">
              <p>⚠️ Der gescannte QR-Code ist kein gültiger Mitgliedschaftscode.</p>
            </div>
          ` : ''}
        </div>
      </div>
    </div>
    `;

    if (this.scutoon.contentEl) {
      this.scutoon.contentEl.innerHTML = content;
      this.scutoon.contentEl.classList.remove('chapter-mode');
    }
  }
}

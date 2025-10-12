/**
 * AccountView - User account and membership details
 */
export class AccountView {
  constructor(scutoon) {
    this.scutoon = scutoon;
  }

  render() {
    this.scutoon.navigationStore.setView('account');
    this.scutoon.navigationStore.updateURL({ view: 'account' });

    if (this.scutoon.contentEl) {
      const membership = this.scutoon.membershipStore.membership;
      const expiryText = membership.expires_at || 'Lifetime';
      const claimedText = membership.claimed_at || 'Not claimed';

      this.scutoon.contentEl.innerHTML = `
      <div class="view account-view">
        <h1>My Account</h1>
        
        ${!membership.is_claimed ? `
        <div class="notification">
          <p>🔔 Reminder: If you claim your membership, we will be able to send you emails about updates and you can make changes to your membership! After all, you did buy this QR code.</p>
          <p><a href="https://qr.scutoons.com/?c=${membership.code}" target="_blank" class="btn">Claim Membership Now</a></p>
        </div>
        ` : ''}
        
        <section>
        <h2>Membership Information</h2>
        <table>
          <tr><td>Type</td><td>${membership.type}</td></tr>
          <tr><td>Status</td><td>${membership.is_claimed ? 'Claimed' : 'Unclaimed'}</td></tr>
          <tr><td>Valid Until</td><td>${expiryText}</td></tr>
          <tr><td>Code</td><td>${membership.code}</td></tr>
        </table>
        </section>
        
        <section>
        <h2>Timeline</h2>
        <table>
          <tr><td>Created</td><td>${membership.created_at}</td></tr>
          <tr><td>Claimed</td><td>${claimedText}</td></tr>
          <tr><td>Last Validated</td><td>${membership.validated_at}</td></tr>
        </table>
        </section>
        
        ${membership.email ? `
        <section>
        <h2>Account Details</h2>
        <table>
          <tr><td>Email</td><td>${membership.email}</td></tr>
        </table>
        </section>
        ` : ''}
        
        <section>
        <h2>Technical Information</h2>
        <table>
          ${membership.redirect_to ? `<tr><td>Content URL</td><td><a href="${membership.redirect_to}" target="_blank">${membership.redirect_to}</a></td></tr>` : ''}
        </table>
        </section>
      </div>
      `;
    }
  }
}

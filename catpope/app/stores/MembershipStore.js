/**
 * MembershipStore - Handles membership state and validation
 */
export class MembershipStore {
  membership = null;

  constructor() {
    this.initMembership();
  }

  // Initialize membership system
  initMembership() {
    const storedMembership = localStorage.getItem('scutoons_membership');
    if (storedMembership) {
      try {
        this.membership = JSON.parse(storedMembership);
        console.log('Found stored membership:');
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
      // Double validation: only store if it's truly a valid membership type
      if (membershipData && this.isValidMembershipType(membershipData.type) && membershipData.code) {
        this.membership = membershipData;
        localStorage.setItem('scutoons_membership', JSON.stringify(membershipData));
        console.log('Membership stored successfully.');
        return true;
      } else {
        console.log('Attempted to store invalid membership.');
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
    if (!expires_at) {
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
        console.log('QR validation successful.');

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

}

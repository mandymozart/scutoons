/**
 * Navigation components - renders nav items
 */

export function renderAccountNavItem() {
  const menuEl = document.querySelector('.nav .menu');
  const accountNavItem = document.createElement('li');
  accountNavItem.classList.add('nav-item', 'account-nav');
  accountNavItem.innerHTML = `
    <a href="#" class="nav-btn" onclick="scutoon.navigateTo('account'); return false;">
      <img src="../assets/icons/account.svg" alt="Account"/>
      <span>My Account</span>
    </a>
  `;
  menuEl.appendChild(accountNavItem);
}

export function removeAccountNavItem() {
  const menuEl = document.querySelector('.nav .menu');
  const accountNav = menuEl?.querySelector('.account-nav');
  if (accountNav) {
    accountNav.remove();
  }
}

export function renderClaimNavItem() {
  const menuEl = document.querySelector('.nav .menu');
  const claimNavItem = document.createElement('li');
  claimNavItem.classList.add('nav-item', 'claim-nav');
  claimNavItem.innerHTML = `
    <a href="https://qr.scutoons.com/" class="nav-btn">
    <img src="../assets/icons/qr.svg" alt="QR"/>
      <span>Claim Membership</span>
    </a>
  `;
  menuEl.appendChild(claimNavItem);
}

export function removeClaimNavItem() {
  const menuEl = document.querySelector('.nav .menu');
  const claimNav = menuEl?.querySelector('.claim-nav');
  if (claimNav) {
    claimNav.remove();
  }
}

export function renderAccountNavigation(hasMembership) {
  const menuEl = document.querySelector('.nav .menu');
  menuEl.innerHTML = '';
  
  if (hasMembership) {
    renderAccountNavItem();
    removeClaimNavItem();
  } else {
    removeAccountNavItem();
    renderClaimNavItem();
  }
}

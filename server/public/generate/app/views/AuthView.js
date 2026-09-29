import {
    clearToken,
    getToken,
    loginWithPassword,
    validateToken,
} from '../utils/auth.js';

export default class AuthView {
  constructor() {
    this.el = null;
  }

  // Resolves with token when authenticated; rejects if user cannot auth.
  async require() {
    const token = getToken();
    if (token && (await validateToken(token))) return token;
    clearToken();
    return this._show();
  }

  _show() {
    return new Promise((resolve, reject) => {
      this.el = document.createElement('div');
      this.el.className = 'auth-view';
      this.el.style.cssText = [
        'position:fixed',
        'inset:0',
        'display:flex',
        'align-items:center',
        'justify-content:center',
        'background:var(--color-bg, #fff)',
        'z-index:9999',
      ].join(';');

      this.el.innerHTML = `
                <div style="width:320px">
                    <h1 style="margin:0 0 .25rem"><img src="https://scutoons.com/SCUTOONS-logo.svg" style="width:100%" alt="SCUTOONS" />                    </h1>
                    <p style="margin:0 0 1.5rem;opacity:.5;font-size:.875rem">Generate Tool</p>
                    <p id="_authErr" style="color:#dc3545;font-size:.875rem;display:none;margin:0 0 .75rem"></p>
                    <form id="_authForm">
                        <div class="form-group">
                            <label>Password</label>
                            <input type="password" id="_authPw" autofocus>
                        </div>
                        <button type="submit" style="width:100%;margin-top:.5rem">Enter</button>
                    </form>
                </div>`;

      document.body.appendChild(this.el);

      document
        .getElementById('_authForm')
        .addEventListener('submit', async (e) => {
          e.preventDefault();
          const pw = document.getElementById('_authPw').value;
          const errEl = document.getElementById('_authErr');
          errEl.style.display = 'none';
          try {
            const token = await loginWithPassword(pw);
            this.destroy();
            resolve(token);
          } catch (err) {
            errEl.textContent = err.message;
            errEl.style.display = 'block';
            document.getElementById('_authPw').value = '';
            document.getElementById('_authPw').focus();
          }
        });
    });
  }

  destroy() {
    this.el?.remove();
    this.el = null;
  }
}

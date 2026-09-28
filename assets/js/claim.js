/**
 * QR landing page claim flow (index.php): "Add to my inventory" → email → one-time code → claimed → redirect.
 * With a stored session (localStorage `scutoons_session`, same key as the viewer) the code is claimed directly.
 * Also handles the magic link (`?login=<token>`). Talks to the same-origin API (/api/, docs/ACCOUNTS.md).
 */
(() => {
  const cfg = window.QR_PAGE;
  if (!cfg) return;
  const T = cfg.texts;
  const SESSION_KEY = 'scutoons_session';
  const $ = (id) => document.getElementById(id);
  const steps = ['step-start', 'step-email', 'step-otp', 'step-status'];
  let email = '';

  const session = {
    get() { try { return localStorage.getItem(SESSION_KEY); } catch { return null; } },
    set(token) { try { localStorage.setItem(SESSION_KEY, token); } catch { /* private mode: session only for this page */ } },
    clear() { try { localStorage.removeItem(SESSION_KEY); } catch { /* ignore */ } },
  };

  async function api(path, body, token) {
    const headers = { 'Content-Type': 'application/json' };
    if (token) headers.Authorization = 'Bearer ' + token;
    try {
      const res = await fetch(cfg.api + path, { method: 'POST', headers, body: JSON.stringify(body || {}) });
      let data = {};
      try { data = await res.json(); } catch { /* empty body */ }
      return { ok: res.ok, status: res.status, data };
    } catch {
      return { ok: false, status: 0, data: {} };
    }
  }

  function show(step) {
    steps.forEach((id) => { const el = $(id); if (el) el.hidden = id !== step; });
  }

  function showError(code) {
    const el = $('error');
    el.textContent = (code && T.errors[code]) || T.error;
    el.hidden = false;
  }

  function clearError() { $('error').hidden = true; }

  function status(title, hint, link) {
    $('status-title').textContent = title;
    $('status-hint').textContent = hint || '';
    const a = $('status-link');
    if (link) { a.href = link.href; a.textContent = link.text; a.hidden = false; } else { a.hidden = true; }
    show('step-status');
  }

  function busy(form, on) {
    form.querySelectorAll('button, input').forEach((el) => { el.disabled = on; });
  }

  /** The chosen item/variant ({} when the code needs no choice, null when a choice is missing) */
  function choice() {
    if (!cfg.needsChoice) return {};
    const input = document.querySelector('input[name=choice]:checked');
    return input ? { item_id: input.dataset.item, variant: input.dataset.variant || null } : null;
  }

  function done() {
    status(T.done, T.done_hint);
    setTimeout(() => { location.href = cfg.inventoryUrl; }, 1200);
  }

  function dropLoginParam() {
    const url = new URL(location.href);
    if (!url.searchParams.has('login')) return;
    url.searchParams.delete('login');
    history.replaceState(null, '', url.pathname + url.search);
  }

  async function claimWithSession(token) {
    status(T.working);
    const r = await api('/inventory/claim/', { code: cfg.code, ...(choice() || {}) }, token);
    if (r.ok) return done();
    if (r.status === 401) {
      session.clear();
      show('step-email');
      return;
    }
    show('step-start');
    showError(r.data.error);
  }

  /** After /auth/verify/: claimed in the same step, or claim now / show why not */
  function afterVerify(data) {
    session.set(data.session_token);
    if (Array.isArray(data.claimed) && data.claimed.includes(cfg.code)) return done();
    if (data.claim_error) {
      show('step-start');
      showError(data.claim_error.error);
      return;
    }
    claimWithSession(data.session_token);
  }

  if (cfg.mode === 'login') {
    // Magic link without a code: just sign in
    status(T.signing_in);
    api('/auth/verify/', { token: cfg.loginToken }).then((r) => {
      dropLoginParam();
      if (r.ok) {
        session.set(r.data.session_token);
        status(T.signed_in, r.data.user ? r.data.user.email : '', { href: cfg.appUrl, text: T.open_app });
      } else {
        status(T.errors[r.data.error] || T.error);
      }
    });
    return;
  }

  $('add').addEventListener('click', () => {
    clearError();
    if (choice() === null) return showError('choose_item');
    const token = session.get();
    if (token) return claimWithSession(token);
    show('step-email');
    $('email').focus();
  });

  $('step-email').addEventListener('submit', async (event) => {
    event.preventDefault();
    clearError();
    const form = event.currentTarget;
    email = $('email').value.trim();
    if (!email || !$('email').checkValidity()) return showError('invalid_email');
    busy(form, true);
    const r = await api('/auth/request/', { email, code: cfg.code, locale: cfg.locale, ...(choice() || {}) });
    busy(form, false);
    if (!r.ok) return showError(r.data.error);
    $('otp-hint').textContent = T.otp_hint + ' ' + email + '.';
    $('otp').value = '';
    show('step-otp');
    $('otp').focus();
  });

  $('step-otp').addEventListener('submit', async (event) => {
    event.preventDefault();
    clearError();
    const form = event.currentTarget;
    const otp = $('otp').value.replace(/\D/g, '');
    if (otp.length !== 6) return showError('invalid_otp');
    busy(form, true);
    const r = await api('/auth/verify/', { email, otp, ...(choice() || {}) });
    busy(form, false);
    if (!r.ok) return showError(r.data.error);
    afterVerify(r.data);
  });

  $('other-email').addEventListener('click', () => {
    clearError();
    show('step-email');
    $('email').focus();
  });

  // Magic link for this code (…/?login=<token>&c=<code>): sign in and claim right away
  if (cfg.loginToken) {
    status(T.signing_in);
    api('/auth/verify/', { token: cfg.loginToken }).then((r) => {
      dropLoginParam();
      if (r.ok) return afterVerify(r.data);
      show('step-start');
      showError(r.data.error);
    });
  }
})();

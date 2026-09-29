/**
 * Users view: reader accounts by email, their roles (toggle member / admin / creator), the signup code and the
 * marketing opt-in. Data: users.php (search), user-roles.php (grant/revoke) – admin token added by the fetch patch
 * in main.js. CLI alternative: php scripts/roles.php grant|revoke|list <email> [role].
 */
const esc = (v) =>
  String(v ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
const day = (iso) => (iso ? String(iso).slice(0, 10) : '—');

export default class UsersView {
  constructor() {
    this.panelEl = document.getElementById('usersMode');
    this.users = [];
    this.roles = [];
    this.total = 0;
    this.q = '';
    this.loaded = false;
    this.timer = null;
    document.querySelector('.mode-option[data-mode="users"]')?.addEventListener('click', () => {
      if (!this.loaded) {
        this.renderShell();
        this.load();
      }
    });
  }

  renderShell() {
    this.panelEl.innerHTML = `
      <h2>Users</h2>
      <div class="stats-toolbar">
        <label>Email <input id="usersQuery" type="search" placeholder="search by email" autocomplete="off"></label>
        <button type="button" id="usersReload">Reload</button>
      </div>
      <p class="muted" id="usersCount"></p>
      <div class="stats-table-wrap"><table class="batch-table users-table">
        <thead><tr><th>Email</th><th>Created</th><th>Roles</th><th>Signup code</th><th>News opt-in</th></tr></thead>
        <tbody id="usersBody"><tr><td colspan="5">Loading…</td></tr></tbody>
      </table></div>`;
    this.panelEl.querySelector('#usersQuery').addEventListener('input', (e) => {
      this.q = e.target.value.trim();
      clearTimeout(this.timer);
      this.timer = setTimeout(() => this.load(), 250);
    });
    this.panelEl.querySelector('#usersReload').addEventListener('click', () => this.load());
  }

  async load() {
    try {
      const res = await fetch(`users.php?q=${encodeURIComponent(this.q)}`);
      const data = await res.json();
      if (!data.success) throw new Error(data.message || res.status);
      this.users = data.users;
      this.roles = data.roles;
      this.total = data.total;
      this.loaded = true;
      this.renderRows();
    } catch (err) {
      this.panelEl.querySelector('#usersBody').innerHTML = `<tr><td colspan="5" class="err">Users failed to load: ${esc(err.message)}</td></tr>`;
    }
  }

  renderRows() {
    this.panelEl.querySelector('#usersCount').textContent =
      `${this.users.length} of ${this.total} ${this.q ? 'matching ' : ''}accounts${this.users.length < this.total ? ' (refine the search)' : ''}`;
    this.panelEl.querySelector('#usersBody').innerHTML =
      this.users
        .map(
          (u) => `<tr data-id="${esc(u.id)}">
          <td>${esc(u.email)}</td>
          <td>${day(u.created_at)}</td>
          <td class="users-roles">${this.roles
            .map(
              (r) => `<label title="${esc(r.description)}" style="white-space:nowrap;margin-right:0.75rem">
                <input type="checkbox" data-role="${esc(r.name)}" ${u.roles.includes(r.name) ? 'checked' : ''}> ${esc(r.name)}</label>`,
            )
            .join('')}</td>
          <td>${esc(u.signup_code ?? '—')}</td>
          <td>${u.marketing_opt_in ? `yes ${day(u.marketing_opt_in_at)}` : '—'}</td>
        </tr>`,
        )
        .join('') || '<tr><td colspan="5">No accounts found</td></tr>';
    this.panelEl.querySelectorAll('.users-roles input[type=checkbox]').forEach((box) =>
      box.addEventListener('change', () => this.toggle(box.closest('tr').dataset.id, box.dataset.role, box.checked, box)),
    );
  }

  async toggle(userId, role, on, box) {
    const user = this.users.find((u) => u.id === userId);
    if (!on && role === 'member' && !confirm(`Remove "member" from ${user?.email}?`)) {
      box.checked = true;
      return;
    }
    box.disabled = true;
    try {
      const res = await fetch('user-roles.php', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ user_id: userId, role, action: on ? 'grant' : 'revoke' }),
      });
      const data = await res.json().catch(() => ({}));
      if (!data.success) throw new Error(data.message || res.status);
      if (user) Object.assign(user, data.user);
    } catch (err) {
      box.checked = !on;
      alert(`Could not change the role: ${err.message}`);
    } finally {
      box.disabled = false;
    }
  }
}

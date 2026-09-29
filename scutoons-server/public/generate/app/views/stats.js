/**
 * Stats view: activity per code (scans, unique devices, accesses, logins, refused claims), a daily chart of
 * scans/unlocks, CSV export and revoke/unrevoke per code. Data: stats.php, code-events.php, revoke.php
 * (admin token added by the fetch patch in main.js). Never shows emails.
 */
const esc = (v) =>
  String(v ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
const day = (iso) => (iso ? String(iso).slice(0, 10) : '—');

const COLUMNS = [
  ['code', 'Code'],
  ['edition_id', 'Edition'],
  ['edition_number', '#'],
  ['type', 'Type'],
  ['scans', 'Scans'],
  ['devices', 'Devices'],
  ['accesses', 'Unlocks'],
  ['logins', 'Logins'],
  ['refused', 'Refused'],
  ['first_seen', 'First seen'],
  ['last_seen', 'Last seen'],
  ['claimed', 'Claimed'],
  ['revoked_at', 'Revoked'],
];

export default class StatsView {
  constructor() {
    this.panelEl = document.getElementById('statsMode');
    this.codes = [];
    this.daily = [];
    this.days = 60;
    this.sort = { key: 'devices', dir: -1 }; // spread first
    this.filter = { edition: '', type: '', q: '' };
    this.loaded = false;
    // Load when the tab is opened the first time
    document.querySelector('.mode-option[data-mode="stats"]')?.addEventListener('click', () => {
      if (!this.loaded) this.load();
    });
  }

  async load() {
    this.panelEl.innerHTML = '<p>Loading…</p>';
    try {
      const res = await fetch(`stats.php?days=${this.days}`);
      const data = await res.json();
      if (!data.success) throw new Error(data.message || res.status);
      this.codes = data.codes;
      this.daily = data.daily;
      this.loaded = true;
      this.render();
    } catch (err) {
      this.panelEl.innerHTML = `<p class="error">Stats failed to load: ${esc(err.message)}</p>`;
    }
  }

  filtered() {
    const { edition, type, q } = this.filter;
    const { key, dir } = this.sort;
    return this.codes
      .filter((c) => (!edition || (c.edition_id ?? '') === edition) && (!type || c.type === type))
      .filter((c) => !q || c.code.toLowerCase().includes(q.toLowerCase()))
      .sort((a, b) => {
        const x = a[key] ?? '';
        const y = b[key] ?? '';
        return (x > y ? 1 : x < y ? -1 : 0) * dir;
      });
  }

  render() {
    const editions = [...new Set(this.codes.map((c) => c.edition_id).filter(Boolean))].sort();
    const types = [...new Set(this.codes.map((c) => c.type))].sort();
    const option = (v, cur, label) => `<option value="${esc(v)}" ${v === cur ? 'selected' : ''}>${esc(label ?? (v || 'all'))}</option>`;
    this.panelEl.innerHTML = `
      <h2>Code activity</h2>
      <div class="stats-toolbar">
        <label>Edition <select id="statsEdition">${option('', this.filter.edition)}${editions.map((e) => option(e, this.filter.edition)).join('')}</select></label>
        <label>Type <select id="statsType">${option('', this.filter.type)}${types.map((t) => option(t, this.filter.type)).join('')}</select></label>
        <label>Code <input id="statsQuery" type="text" value="${esc(this.filter.q)}" placeholder="search"></label>
        <label>Chart <select id="statsDays">${[14, 30, 60, 180, 365].map((d) => option(String(d), String(this.days), `${d} days`)).join('')}</select></label>
        <button type="button" id="statsCsv">CSV</button>
        <button type="button" id="statsReload">Reload</button>
      </div>
      <div class="stats-chart">${this.chart()}</div>
      <div id="statsDetail"></div>
      <p class="muted" id="statsCount"></p>
      <div class="stats-table-wrap"><table class="batch-table stats-table">
        <thead><tr>${COLUMNS.map(([k, l]) => `<th data-key="${k}" class="sortable">${esc(l)}${this.sort.key === k ? (this.sort.dir < 0 ? ' ↓' : ' ↑') : ''}</th>`).join('')}</tr></thead>
        <tbody id="statsBody"></tbody>
      </table></div>`;
    // Edition "" means all; editions list contains "" for codes without edition – treat as all
    this.renderRows();

    this.panelEl.querySelector('#statsEdition').addEventListener('change', (e) => { this.filter.edition = e.target.value; this.renderRows(); });
    this.panelEl.querySelector('#statsType').addEventListener('change', (e) => { this.filter.type = e.target.value; this.renderRows(); });
    this.panelEl.querySelector('#statsQuery').addEventListener('input', (e) => { this.filter.q = e.target.value.trim(); this.renderRows(); });
    this.panelEl.querySelector('#statsDays').addEventListener('change', (e) => { this.days = Number(e.target.value); this.load(); });
    this.panelEl.querySelector('#statsReload').addEventListener('click', () => this.load());
    this.panelEl.querySelector('#statsCsv').addEventListener('click', () => this.downloadCsv());
    this.panelEl.querySelectorAll('th.sortable').forEach((th) =>
      th.addEventListener('click', () => {
        const key = th.dataset.key;
        this.sort = { key, dir: this.sort.key === key ? -this.sort.dir : -1 };
        this.render();
      }),
    );
  }

  renderRows() {
    const rows = this.filtered();
    this.panelEl.querySelector('#statsCount').textContent = `${rows.length} of ${this.codes.length} codes`;
    this.panelEl.querySelector('#statsBody').innerHTML = rows
      .map(
        (c) => `<tr data-code="${esc(c.code)}" class="${c.revoked_at ? 'revoked' : ''}">
          <td><a href="#" class="stats-code">${esc(c.code)}</a></td>
          <td>${esc(c.edition_id ?? '—')}</td><td>${esc(c.edition_number ?? '—')}</td><td>${esc(c.type)}</td>
          <td>${c.scans}</td><td>${c.devices}</td><td>${c.accesses}</td><td>${c.logins}</td><td>${c.refused}</td>
          <td>${day(c.first_seen)}</td><td>${day(c.last_seen)}</td>
          <td>${c.claimed ? `yes ${day(c.claimed_at)}` : '—'}</td>
          <td>${c.revoked_at ? `<span class="err">${esc(day(c.revoked_at))}</span>` : '—'}</td>
        </tr>`,
      )
      .join('');
    this.panelEl.querySelectorAll('.stats-code').forEach((a) =>
      a.addEventListener('click', (e) => {
        e.preventDefault();
        this.showDetail(a.closest('tr').dataset.code);
      }),
    );
  }

  /** Daily bars: scans (outline) and unlocks (filled), plain SVG */
  chart() {
    const byDay = new Map(this.daily.map((d) => [d.day, d]));
    const days = [];
    for (let i = this.days - 1; i >= 0; i--) {
      const d = new Date(Date.now() - i * 86400000).toISOString().slice(0, 10);
      days.push(byDay.get(d) ?? { day: d, scans: 0, accesses: 0 });
    }
    const max = Math.max(1, ...days.map((d) => Math.max(d.scans, d.accesses)));
    const w = 720, h = 160, pad = 24, bw = (w - pad * 2) / days.length;
    const y = (v) => h - pad - (v / max) * (h - pad * 2);
    const bars = days
      .map((d, i) => {
        const x = pad + i * bw;
        return `<g><title>${d.day}: ${d.scans} scans, ${d.accesses} unlocks</title>
          <rect x="${x + 1}" y="${y(d.scans)}" width="${Math.max(1, bw / 2 - 1)}" height="${h - pad - y(d.scans)}" fill="none" stroke="#000" stroke-width="1"/>
          <rect x="${x + bw / 2}" y="${y(d.accesses)}" width="${Math.max(1, bw / 2 - 1)}" height="${h - pad - y(d.accesses)}" fill="#00d1ff"/></g>`;
      })
      .join('');
    return `<svg viewBox="0 0 ${w} ${h}" width="100%" role="img" aria-label="Daily scans and unlocks">
      <line x1="${pad}" y1="${h - pad}" x2="${w - pad}" y2="${h - pad}" stroke="#000" stroke-width="2"/>
      <text x="${pad}" y="14" font-size="11">max ${max}/day · outline = scans · cyan = unlocks (unique devices/day)</text>
      <text x="${pad}" y="${h - 6}" font-size="10">${days[0].day}</text>
      <text x="${w - pad}" y="${h - 6}" font-size="10" text-anchor="end">${days[days.length - 1].day}</text>
      ${bars}</svg>`;
  }

  async showDetail(code) {
    const el = this.panelEl.querySelector('#statsDetail');
    el.innerHTML = '<p>Loading…</p>';
    try {
      const res = await fetch(`code-events.php?code=${encodeURIComponent(code)}`);
      const data = await res.json();
      if (!data.success) throw new Error(data.message || res.status);
      const c = data.code;
      el.innerHTML = `
        <div class="stats-detail">
          <h3>${esc(c.code)} <small>${esc(c.type)} · ${esc(c.edition_id ?? 'no edition')}${c.edition_number ? ' #' + esc(c.edition_number) : ''}</small></h3>
          <p>Created ${esc(c.created_at)} · ${c.claimed ? 'claimed ' + esc(c.claimed_at) : 'not claimed'} · ${c.expires_at ? 'expires ' + esc(c.expires_at) : 'no expiry'}</p>
          <p>${c.revoked_at ? `<span class="err">Revoked ${esc(c.revoked_at)}${c.revoked_reason ? ' – ' + esc(c.revoked_reason) : ''}</span>` : 'Active'}</p>
          <div class="stats-revoke">
            ${c.revoked_at
              ? '<button type="button" id="statsUnrevoke">Unrevoke</button>'
              : '<input type="text" id="statsReason" placeholder="Reason (e.g. shared publicly)" maxlength="255"> <button type="button" id="statsRevoke">Revoke</button>'}
            <button type="button" id="statsClose">Close</button>
          </div>
          <h4>Timeline (${data.events.length})</h4>
          <table class="batch-table"><thead><tr><th>When (UTC)</th><th>Event</th><th>Device</th><th>Account</th><th>Toon</th></tr></thead>
          <tbody>${data.events
            .map((e) => `<tr><td>${esc(e.created_at)}</td><td>${esc(e.event)}</td><td>${esc(e.device ?? '—')}</td><td>${e.account ? 'yes' : '—'}</td><td>${esc(e.toon ?? '—')}</td></tr>`)
            .join('') || '<tr><td colspan="5">No events yet</td></tr>'}</tbody></table>
        </div>`;
      el.querySelector('#statsClose').addEventListener('click', () => (el.innerHTML = ''));
      el.querySelector('#statsRevoke')?.addEventListener('click', () => this.revoke(code, 'revoke', el.querySelector('#statsReason').value));
      el.querySelector('#statsUnrevoke')?.addEventListener('click', () => this.revoke(code, 'unrevoke', ''));
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
    } catch (err) {
      el.innerHTML = `<p class="error">${esc(err.message)}</p>`;
    }
  }

  async revoke(code, action, reason) {
    if (action === 'revoke' && !confirm(`Revoke ${code}? It will unlock nothing any more.`)) return;
    const res = await fetch('revoke.php', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code, action, reason }),
    });
    const data = await res.json().catch(() => ({}));
    if (!data.success) return alert(data.message || 'Failed');
    const row = this.codes.find((c) => c.code === code);
    if (row) Object.assign(row, { revoked_at: data.revoked_at, revoked_reason: data.revoked_reason });
    this.renderRows();
    this.showDetail(code);
  }

  async downloadCsv() {
    const res = await fetch('stats.php?format=csv');
    if (!res.ok) return alert('CSV export failed');
    const url = URL.createObjectURL(await res.blob());
    const a = Object.assign(document.createElement('a'), { href: url, download: `scutoons-code-stats-${new Date().toISOString().slice(0, 10)}.csv` });
    document.body.append(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  }
}

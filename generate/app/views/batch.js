import BatchProgressTable from '../components/BatchProgressTable.js';
import { QR_BASE_URL } from '../config.js';
import { generateFingerprint, signQRCode } from '../utils/crypto.js';
import { generateCoaPdf } from '../utils/pdf.js';
import { generatePrintSheet } from '../utils/printSheet.js';

export default class BatchView {
  constructor() {
    this.panelEl = document.getElementById('batchMode');
    this.manifest = null;
    this.progressTable = null;
    this.editions = [];
    this.isRunning = false;
    this.isPaused = false;
    this.shouldAbort = false;

    this.init();
  }

  // ─── Boot ────────────────────────────────────────────────────────────────

  async init() {
    this.panelEl.innerHTML = '<p>Loading…</p>';

    try {
      console.log('[batch] init start');

      const [batchesRes, editionsRes, toonsRes] = await Promise.all([
        fetch('get-batches.php')
          .then((r) => {
            if (!r.ok) throw new Error(`get-batches.php ${r.status}`);
            return r.json();
          })
          .catch((e) => {
            console.warn('[batch] get-batches.php:', e.message);
            return { batches: [] };
          }),

        fetch('../api/editions/index.php')
          .then((r) => {
            if (!r.ok) throw new Error(`editions ${r.status}`);
            return r.json();
          })
          .catch((e) => {
            console.warn('[batch] editions:', e.message);
            return [];
          }),

        fetch('/api/toons/index.php')
          .then((r) => {
            if (!r.ok) throw new Error(`toons ${r.status}`);
            return r.json();
          })
          .catch((e) => {
            console.warn('[batch] toons:', e.message);
            return [];
          }),
      ]);

      this.editions = Array.isArray(editionsRes)
        ? editionsRes
        : Object.values(editionsRes);
      this.toons = Array.isArray(toonsRes) ? toonsRes : [];
      this.batches = batchesRes.batches || [];

      console.log(
        `[batch] loaded — editions: ${this.editions.length}, toons: ${this.toons.length}, batches: ${this.batches.length}`,
      );
      this.renderShell(this.batches);
    } catch (err) {
      console.error('[batch] init failed:', err);
      this.panelEl.innerHTML = `<p class="error">Batch view failed to load: ${err.message}</p>`;
    }
  }

  // ─── Shell (history always visible at top) ────────────────────────────────

  buildHistoryHtml(batches) {
    if (batches.length === 0)
      return '<p class="muted">No previous batches.</p>';

    const rows = batches
      .map((b) => {
        const date = b.created_at
          ? new Date(b.created_at).toLocaleDateString()
          : '—';
        const badge = b.complete
          ? '<span class="badge badge-done">COMPLETE</span>'
          : `<span class="badge badge-warn">INCOMPLETE</span>`;
        const resumeBtn = !b.complete
          ? `<button class="btn-resume" data-file="${b.filename}">Resume</button>`
          : '';
        const downloadBtn =
          b.done > 0
            ? `<button class="btn-download" onclick="window.location.href='download-batch.php?file=${encodeURIComponent(b.filename)}'">Download ZIP</button>`
            : '';
        const printBtn =
          b.done > 0
            ? `<button class="btn-print-sheet" data-file="${b.filename}">Print Sheet</button>`
            : '';
        return `<tr>
                <td>${date}</td>
                <td>${b.edition}</td>
                <td>${b.done}&thinsp;/&thinsp;${b.total}</td>
                <td>${b.errors > 0 ? `<span class="err">${b.errors} errors</span>` : '—'}</td>
                <td>${badge}</td>
                <td>${resumeBtn}${downloadBtn}${printBtn}</td>
            </tr>`;
      })
      .join('');

    return `
            <table class="batch-table batch-history">
                <thead><tr>
                    <th>Date</th><th>Edition</th><th>Progress</th>
                    <th>Errors</th><th>Status</th><th></th>
                </tr></thead>
                <tbody>${rows}</tbody>
            </table>`;
  }

  async refreshHistory() {
    const res = await fetch('get-batches.php')
      .then((r) => r.json())
      .catch(() => ({ batches: [] }));
    this.batches = res.batches || [];
    const historyEl = document.getElementById('batchHistory');
    if (!historyEl) return;
    historyEl.innerHTML = this.buildHistoryHtml(this.batches);
    historyEl.querySelectorAll('.btn-resume').forEach((btn) => {
      btn.addEventListener('click', () => this.resumeBatch(btn.dataset.file));
    });

    historyEl.querySelectorAll('.btn-print-sheet').forEach((btn) => {
      btn.addEventListener('click', () =>
        this.downloadPrintSheet(btn.dataset.file),
      );
    });
  }

  renderShell(batches) {
    this.panelEl.innerHTML = `
            <div id="batchHistory">${this.buildHistoryHtml(batches)}</div>
            <div id="batchConfig" style="display:none;"></div>
            <div id="batchProgress" style="display:none;"></div>
            <button id="newBatchBtn" style="margin-top:1rem;">New Batch</button>
        `;

    document
      .getElementById('newBatchBtn')
      .addEventListener('click', () => this.renderConfig());

    this.panelEl.querySelectorAll('.btn-resume').forEach((btn) => {
      btn.addEventListener('click', () => this.resumeBatch(btn.dataset.file));
    });

    this.panelEl.querySelectorAll('.btn-print-sheet').forEach((btn) => {
      btn.addEventListener('click', () =>
        this.downloadPrintSheet(btn.dataset.file),
      );
    });
  }

  // ─── Config form ─────────────────────────────────────────────────────────

  renderConfig() {
    const configEl = document.getElementById('batchConfig');

    const options = this.editions
      .map(
        (e) =>
          `<option value="${e.id}">${e.name} — ${e.edition_count} editions</option>`,
      )
      .join('');

    configEl.innerHTML = `
            <hr>
            <h3>New Batch</h3>
            <form id="batchConfigForm">
                <div class="form-group">
                    <label>Edition</label>
                    <select id="batchEdition">${options}</select>
                </div>
                <div id="editionWarning" style="display:none;" class="message error"></div>
                <div class="form-group">
                    <label>Count</label>
                    <input type="number" id="batchCount" value="" min="1" max="2000"
                           placeholder="defaults to edition count">
                </div>
                <div class="form-group">
                    <label>Starting number</label>
                    <input type="number" id="batchStartNumber" value="1" min="1">
                </div>
                <button type="submit" id="startBatchBtn">Start Batch</button>
                <button type="button" id="cancelConfigBtn" style="margin-left:.5rem;">Cancel</button>
            </form>
        `;
    configEl.style.display = 'block';

    document.getElementById('cancelConfigBtn').addEventListener('click', () => {
      configEl.style.display = 'none';
    });

    const editionSelect = document.getElementById('batchEdition');
    const countInput = document.getElementById('batchCount');
    const startBtn = document.getElementById('startBatchBtn');
    const warningEl = document.getElementById('editionWarning');

    const checkEdition = () => {
      const ed = this.editions.find((e) => e.id === editionSelect.value);
      const existing = (this.batches || []).find(
        (b) => b.editionId === editionSelect.value && b.complete,
      );

      if (ed) countInput.value = ed.edition_count;

      if (existing) {
        const date = new Date(existing.created_at).toLocaleDateString();
        warningEl.textContent = `Batch already completed for this edition (${existing.done} codes, ${date}). Add a new edition to editions.json to generate more.`;
        warningEl.style.display = 'block';
        startBtn.disabled = true;
      } else {
        warningEl.style.display = 'none';
        startBtn.disabled = false;
      }
    };

    editionSelect.addEventListener('change', checkEdition);
    checkEdition();

    document
      .getElementById('batchConfigForm')
      .addEventListener('submit', async (e) => {
        e.preventDefault();
        if (startBtn.disabled) return;
        const edition = this.editions.find(
          (ed) => ed.id === editionSelect.value,
        );
        if (!edition) return;
        const count = parseInt(countInput.value) || edition.edition_count;
        const startNumber =
          parseInt(document.getElementById('batchStartNumber').value) || 1;
        configEl.style.display = 'none';
        await this.startBatch(edition, count, startNumber);
      });
  }

  // ─── Start / Resume ───────────────────────────────────────────────────────

  async startBatch(edition, count, startNumber) {
    const codes = [];
    for (let i = 0; i < count; i++) {
      codes.push({
        n: startNumber + i,
        code: window.nanoid(8),
        status: 'pending',
        completedSteps: [],
      });
    }

    const ts = new Date().toISOString().replace(/[:.]/g, '').slice(0, 15);
    const filename = `BATCH-${ts}-${edition.id}.json`;

    this.manifest = {
      filename,
      edition: edition.name,
      editionId: edition.id,
      productname: edition.name,
      edition_number: edition.edition_number ?? 1,
      edition_count: edition.edition_count,
      excluded_number: edition.excluded_number ?? 0,
      creators: edition.creators ?? '',
      redirectUrl:
        this.toons.find((t) => t.id === edition.toon)?.redirectUrl ?? '',
      total: count,
      created_at: new Date().toISOString(),
      codes,
    };

    await this.saveManifest();
    this.mountProgressTable();
    await this.runLoop();
    await this.refreshHistory();
  }

  async resumeBatch(filename) {
    const data = await fetch(
      `get-batch.php?file=${encodeURIComponent(filename)}`,
    ).then((r) => r.json());
    this.manifest = data;
    this.mountProgressTable();
    await this.runLoop();
    await this.refreshHistory();
  }

  mountProgressTable() {
    this.progressTable = new BatchProgressTable(
      document.getElementById('batchProgress'),
      this.manifest,
      {
        onTogglePause: () => this.togglePause(),
        onAbort: () => this.abort(),
      },
    );
    this.progressTable.render();
  }

  // ─── Main loop ────────────────────────────────────────────────────────────

  async runLoop() {
    this.isRunning = true;
    this.isPaused = false;
    this.shouldAbort = false;

    for (const item of this.manifest.codes) {
      if (item.status === 'done') continue;

      while (this.isPaused && !this.shouldAbort) {
        await this.sleep(300);
      }
      if (this.shouldAbort) break;

      this.progressTable.markStatus(item.n, 'running');

      try {
        await this.processCode(item);
        item.status = 'done';
        this.progressTable.markStatus(item.n, 'done');
      } catch (err) {
        console.error(`[batch] code ${item.code} failed:`, err);
        item.status = 'error';
        this.progressTable.markStatus(item.n, 'error');
      }

      this.progressTable.updateSummary();
      await this.saveManifest();
    }

    this.isRunning = false;
    this.progressTable.finalizeControls();
  }

  // ─── Per-code pipeline ────────────────────────────────────────────────────

  async processCode(item) {
    const qrUrl = QR_BASE_URL + item.code;

    const qrData = {
      code: item.code,
      type: 'membership',
      redirect_to: this.manifest.redirectUrl,
      url: qrUrl,
      created_at: new Date().toISOString(),
      expires_at: null,
      productname: this.manifest.productname,
      edition_number: item.n,
      edition_count: this.manifest.edition_count,
      excluded_number: this.manifest.excluded_number,
      creators: this.manifest.creators,
      receiver: '',
    };

    // 1 · Sign
    const signatureData = await signQRCode(qrData);
    qrData.signature = signatureData;
    qrData.fingerprint = generateFingerprint(signatureData);
    this.progressTable.markStep(item.n, 'sign', true);

    // 2 · Register in DB
    if (!item.completedSteps.includes('db')) {
      const fd = new FormData();
      fd.append('code', item.code);
      fd.append('redirectUrl', this.manifest.redirectUrl);
      fd.append('type', 'membership');
      fd.append('email', '');
      fd.append('edition_id', this.manifest.editionId);

      const res = await fetch('generate.php', { method: 'POST', body: fd });
      const result = await res.json();

      if (!result.success && !result.message?.includes('already exists')) {
        throw new Error('DB: ' + result.message);
      }
      item.completedSteps.push('db');
      this.progressTable.markStep(item.n, 'db', true);
    } else {
      this.progressTable.markStep(item.n, 'db', true);
    }

    // 3 · Generate SVG
    const svgEl = new QRCode({
      msg: qrUrl,
      dim: 256,
      pad: 4,
      ecl: 'H',
      pal: ['#000000', '#FFFFFF'],
    });

    // 4 · Save SVG
    if (!item.completedSteps.includes('svg')) {
      const svgString = new XMLSerializer().serializeToString(svgEl);
      await this.saveFile(
        svgString,
        `SCUTOONS-QR-MEMBERSHIP-${item.code}.svg`,
        'svgs',
      );
      item.completedSteps.push('svg');
      this.progressTable.markStep(item.n, 'svg', true);
    } else {
      this.progressTable.markStep(item.n, 'svg', true);
    }

    // 5 · Generate + save CoA PDF
    if (!item.completedSteps.includes('pdf')) {
      const templateData = {
        productname: this.manifest.productname,
        edition_number: item.n,
        edition_count: this.manifest.edition_count,
        excluded_number: this.manifest.excluded_number,
        creators: this.manifest.creators,
        receiver: '',
      };

      const doc = await generateCoaPdf(qrData, templateData, svgEl);
      const pdfData = doc.output('datauristring');
      await this.saveFile(
        pdfData,
        `SCUTOONS-COA-${item.code}.pdf`,
        'certificates',
      );
      item.completedSteps.push('pdf');
      this.progressTable.markStep(item.n, 'pdf', true);
    } else {
      this.progressTable.markStep(item.n, 'pdf', true);
    }

    // 6 · Save signature JSON + DB
    if (!item.completedSteps.includes('sig')) {
      const sigJson = JSON.stringify(
        {
          code: item.code,
          type: 'membership',
          signature: signatureData,
          fingerprint: qrData.fingerprint,
          created_at: qrData.created_at,
        },
        null,
        2,
      );

      await this.saveFile(sigJson, `SIGNATURE-${item.code}.json`, 'signatures');

      await fetch('save-signature.php', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ code: item.code, signature: signatureData }),
      });

      item.completedSteps.push('sig');
      this.progressTable.markStep(item.n, 'sig', true);
    } else {
      this.progressTable.markStep(item.n, 'sig', true);
    }
  }

  // ─── Helpers ─────────────────────────────────────────────────────────────

  async downloadPrintSheet(filename) {
    const btn = document.querySelector(
      `.btn-print-sheet[data-file="${filename}"]`,
    );
    const original = btn?.textContent;
    if (btn) {
      btn.textContent = 'Generating…';
      btn.disabled = true;
    }

    try {
      const data = await fetch(
        `get-batch.php?file=${encodeURIComponent(filename)}`,
      ).then((r) => r.json());
      const doc = await generatePrintSheet(data);
      doc.save(filename.replace('.json', '-PRINT.pdf'));
    } catch (err) {
      console.error('[printSheet]', err);
      alert('Print sheet generation failed: ' + err.message);
    } finally {
      if (btn) {
        btn.textContent = original;
        btn.disabled = false;
      }
    }
  }

  async saveFile(content, filename, type) {
    const fd = new FormData();
    fd.append('doc', content);
    fd.append('filename', filename);
    fd.append('type', type);

    const res = await fetch('files.php', { method: 'POST', body: fd });
    const result = await res.json();
    if (!result.success)
      throw new Error(`Save failed (${filename}): ${result.error}`);
  }

  async saveManifest() {
    const clean = JSON.stringify(this.manifest, null, 2);
    await this.saveFile(clean, this.manifest.filename, 'batch');
  }

  togglePause() {
    this.isPaused = !this.isPaused;
    this.progressTable?.setPaused(this.isPaused);
  }

  abort() {
    this.shouldAbort = true;
  }

  sleep(ms) {
    return new Promise((r) => setTimeout(r, ms));
  }
}

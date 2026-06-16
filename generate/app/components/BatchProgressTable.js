export default class BatchProgressTable {
    constructor(containerEl, manifest, { onTogglePause, onAbort }) {
        this.containerEl   = containerEl;
        this.manifest      = manifest;
        this.onTogglePause = onTogglePause;
        this.onAbort       = onAbort;
    }

    render() {
        const rows = this.manifest.codes.map(item => this.buildRow(item)).join('');

        this.containerEl.innerHTML = `
            <hr>
            <h3>${this.manifest.edition} — ${this.manifest.total} codes</h3>
            <div id="batchControls" style="margin:.5rem 0;">
                <button id="pauseBtn">Pause</button>
                <button id="abortBtn" style="margin-left:.5rem;">Abort</button>
            </div>
            <p id="batchSummary" class="muted">Progress: 0 / ${this.manifest.total} &nbsp;|&nbsp; Errors: 0</p>
            <div class="progress-wrap">
                <table class="batch-table batch-progress">
                    <thead><tr>
                        <th>#</th><th>CODE</th>
                        <th>SIGN</th><th>DB</th><th>SVG</th><th>PDF</th><th>SIG</th>
                        <th>STATUS</th>
                    </tr></thead>
                    <tbody id="progressBody">${rows}</tbody>
                </table>
            </div>
        `;
        this.containerEl.style.display = 'block';

        document.getElementById('pauseBtn').addEventListener('click', () => this.onTogglePause());
        document.getElementById('abortBtn').addEventListener('click', () => this.onAbort());
    }

    buildRow(item) {
        const isDone = item.status === 'done';
        const steps  = ['sign', 'db', 'svg', 'pdf', 'sig'];
        const cells  = steps.map(s => {
            const done = isDone || item.completedSteps?.includes(s);
            return `<td id="step-${s}-${item.n}" class="${done ? 'step-done' : 'step-pending'}">${done ? '✓' : '—'}</td>`;
        }).join('');

        const statusText  = { done: '✓ DONE', error: '✗ ERROR', pending: '—', partial: '…' }[item.status] || '—';
        const statusClass = item.status === 'done' ? 'step-done' : (item.status === 'error' ? 'step-error' : '');

        return `<tr id="row-${item.n}">
            <td>${item.n}</td>
            <td class="mono">${item.code}</td>
            ${cells}
            <td id="status-${item.n}" class="${statusClass}">${statusText}</td>
        </tr>`;
    }

    markStep(n, step, ok) {
        const el = document.getElementById(`step-${step}-${n}`);
        if (!el) return;
        el.textContent = ok ? '✓' : '✗';
        el.className   = ok ? 'step-done' : 'step-error';
    }

    markStatus(n, status) {
        const el = document.getElementById(`status-${n}`);
        if (!el) return;
        const map  = { running: '…', done: '✓ DONE', error: '✗ ERROR' };
        el.textContent = map[status] ?? status;
        el.className   = status === 'done' ? 'step-done' : (status === 'error' ? 'step-error' : '');
        if (status === 'running') {
            document.getElementById(`row-${n}`)?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
        }
    }

    updateSummary() {
        const done   = this.manifest.codes.filter(c => c.status === 'done').length;
        const errors = this.manifest.codes.filter(c => c.status === 'error').length;
        const el = document.getElementById('batchSummary');
        if (el) el.textContent = `Progress: ${done} / ${this.manifest.total}   |   Errors: ${errors}`;
    }

    setPaused(isPaused) {
        const btn = document.getElementById('pauseBtn');
        if (btn) btn.textContent = isPaused ? 'Resume' : 'Pause';
    }

    finalizeControls() {
        const done = this.manifest.codes.filter(c => c.status === 'done').length;
        const el   = document.getElementById('batchControls');
        if (!el) return;
        el.innerHTML = done === this.manifest.total
            ? '<strong>✓ Batch complete!</strong>'
            : '<strong>Batch stopped. Reload this view to resume.</strong>';
    }
}

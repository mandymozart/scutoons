import { QR_BASE_URL } from '../config.js';

// A4 portrait: 210 × 297 mm, grid centered
// 4×40 + 3×5 = 175 mm  →  (210−175)/2 = 17.5 mm L/R
// 6×40 + 5×5 = 265 mm  →  (297−265)/2 = 16 mm  T/B
const COLS = 4;
const ROWS = 6;
const PER_PAGE = COLS * ROWS; // 24
const STICKER_MM = 40;
const GAP = 6;
const PAGE_W = 210;
const PAGE_H = 297;
const MARGIN_X = (PAGE_W - (COLS * STICKER_MM + (COLS - 1) * GAP)) / 2; // 17.5 mm
const MARGIN_Y = (PAGE_H - (ROWS * STICKER_MM + (ROWS - 1) * GAP)) / 2; // 16 mm
const QR_MM = 26;
const TEXT_GAP = 1.5;

export async function generatePrintSheet(manifest) {
  const { jsPDF } = window.jspdf;
  const doc = new jsPDF({ unit: 'mm', format: [PAGE_W, PAGE_H] });

  const codes = manifest.codes.filter((c) => c.status === 'done');

  for (let i = 0; i < codes.length; i++) {
    const item = codes[i];
    const posOnPage = i % PER_PAGE;

    if (i > 0 && posOnPage === 0) doc.addPage();

    const col = posOnPage % COLS;
    const row = Math.floor(posOnPage / COLS);
    const x = MARGIN_X + col * (STICKER_MM + GAP);
    const y = MARGIN_Y + row * (STICKER_MM + GAP);
    const cx = x + STICKER_MM / 2;
    const cy = y + STICKER_MM / 2;

    // Circle outline
    // doc.setDrawColor(180);
    // doc.setLineWidth(0.1);
    // doc.circle(cx, cy, STICKER_MM / 2 - 0.5);

    // QR centered in circle
    const svgEl = new QRCode({
      msg: QR_BASE_URL + item.code,
      dim: 256,
      pad: 4,
      ecl: 'H',
      pal: ['#000000', '#FFFFFF'],
    });
    const qrX = cx - QR_MM / 2;
    const qrY = cy - QR_MM / 2;
    await svgToPdfImage(doc, svgEl, qrX, qrY, QR_MM, QR_MM);

    doc.setFont('courier', 'normal');
    doc.setTextColor(60);

    // "n / total" above the QR
    doc.setFontSize(10);
    doc.text(`${item.n} / ${manifest.total}`, cx, qrY - TEXT_GAP + 3, {
      align: 'center',
    });

    // short code below the QR
    doc.setFontSize(10);
    doc.text(item.code, cx, qrY + QR_MM + TEXT_GAP, { align: 'center' });
  }

  doc.setTextColor(0);
  return doc;
}

function svgToPdfImage(doc, svgEl, x, y, w, h) {
  return new Promise((resolve, reject) => {
    const canvas = document.createElement('canvas');
    canvas.width = 256;
    canvas.height = 256;
    const ctx = canvas.getContext('2d');
    const img = new Image();
    const svg = new XMLSerializer().serializeToString(svgEl);
    const blob = new Blob([svg], { type: 'image/svg+xml;charset=utf-8' });
    const url = URL.createObjectURL(blob);

    img.onload = () => {
      ctx.drawImage(img, 0, 0);
      doc.addImage(canvas.toDataURL('image/png'), 'PNG', x, y, w, h);
      URL.revokeObjectURL(url);
      resolve();
    };
    img.onerror = reject;
    img.src = url;
  });
}

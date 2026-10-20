// Money on the Storyboard page's Generate panel (#121): every price in euro (owner 2026-10-20 "preise in euro": de-DE,
// an estimate with ≈), the provider's own figures (credits, USD) and the rate in the tooltip – the server's rate (the
// ECB's euro reference rate, EuroRates); the budget gate's 402 as one notice, a missing key's 409. Pure (tested in
// test/storyboard-generate.test.ts).
import type { EuroRate, Estimate, GeneratorModel } from './storyboard-generations.ts';

// ── Money (euro, de-DE) ──────────────────────────────────────────────────────────────────────────────────────

const EUR = new Intl.NumberFormat('de-DE', { style: 'currency', currency: 'EUR' });
const EUR_FINE = new Intl.NumberFormat('de-DE', { style: 'currency', currency: 'EUR', maximumFractionDigits: 4 });
const EUR_WHOLE = new Intl.NumberFormat('de-DE', { style: 'currency', currency: 'EUR', maximumFractionDigits: 0 });

/** a provider amount in euro at the server's rate (a currency without a rate: null) */
export function toEuro(amount: number, currency: string | null, rate: EuroRate | null): number | null {
	const c = (currency ?? 'USD').toUpperCase();
	if (c === 'EUR') return amount;
	if (c === 'USD' && rate) return amount * rate.eur_per_usd;
	return null;
}

/** "0,04 €" – an amount under half a cent gets up to 4 decimals (never "0,00 €"); `whole` for budgets */
export function euro(eur: number, opts: { whole?: boolean } = {}): string {
	if (opts.whole && Number.isInteger(eur)) return EUR_WHOLE.format(eur);
	return (eur !== 0 && Math.abs(eur) < 0.005 ? EUR_FINE : EUR).format(eur);
}

/** a provider amount as euro text ("≈ 0,04 €" for an estimate); '–' without a rate */
export function money(amount: number | null, currency: string | null, rate: EuroRate | null, approx = false): string {
	if (amount === null) return '';
	const eur = toEuro(amount, currency, rate);
	return eur === null ? `${amount} ${currency ?? ''}`.trim() : `${approx ? '≈ ' : ''}${euro(eur)}`;
}

/** the words of the provider's figures (i18n: the panel passes its messages) */
export interface FigureWords {
	credits: (count: number) => string;
	rate: (eurPerUsd: string, date: string) => string;
	fallback: (eurPerUsd: string, date: string) => string;
}
export const ENGLISH_FIGURES: FigureWords = {
	credits: (n) => `${n} credit${n === 1 ? '' : 's'}`,
	rate: (r, d) => `1 USD = ${r} € (ECB ${d})`,
	fallback: (r, d) => `1 USD ≈ ${r} € (the fixed fallback of ${d} – no newer ECB rate)`
};

/** an amount's tooltip: the provider's own figures + the rate – "25 credits = 0.25 USD · 1 USD = 0.8909 € (ECB 2026-10-19)" */
export function figuresTitle(amount: number | null, currency: string | null, rate: EuroRate | null, m: GeneratorModel | null = null, words: FigureWords = ENGLISH_FIGURES): string {
	if (amount === null) return '';
	const c = currency ?? 'USD';
	const credit = typeof m?.price.credit === 'number' ? m.price.credit : null;
	const figure = `${credit ? `${words.credits(Math.round(amount / credit))} = ` : ''}${Number(amount.toFixed(4))} ${c}`;
	if (!rate || c === 'EUR') return figure;
	const r = rate.eur_per_usd.toFixed(4).replace('.', ',');
	return `${figure} · ${rate.source === 'ecb' ? words.rate(r, rate.date) : words.fallback(r, rate.date)}`;
}

/** the estimate's line: "≈ 0,04 € · 9:16 · 5 s" */
export function estimateLine(e: Estimate, rate: EuroRate | null): string {
	return [money(e.amount, e.currency, rate ?? e.budget?.rate ?? null, true), e.ratio, e.duration ? `${Number(e.duration.toFixed(1))} s` : null].filter(Boolean).join(' · ');
}

// ── The budget gate (402) ────────────────────────────────────────────────────────────────────────────────────

/** an error as @scutoons/api's ApiError carries it (status, code, the JSON body) */
interface ErrorLike {
	status?: number;
	code?: string;
	body?: Record<string, unknown> | null;
}

/** the gate's refusal as one notice: no budget set, or used up (euro figures from the server) – null = another error */
export type BudgetNotice = { kind: 'none' } | { kind: 'exhausted'; spent: number; reserved: number; budget: number; estimate: number };
export function budgetNotice(e: unknown): BudgetNotice | null {
	const err = e as ErrorLike | null;
	if (!err || err.status !== 402) return null;
	if (err.code === 'no_budget') return { kind: 'none' };
	const n = (k: string) => Number(err.body?.[k] ?? 0);
	return { kind: 'exhausted', spent: n('spent'), reserved: n('reserved'), budget: n('budget'), estimate: n('estimate') };
}

/** 409 generator_not_configured: the setting the admin adds (else null) */
export const missingSetting = (e: unknown): string | null => {
	const err = e as ErrorLike | null;
	return err?.status === 409 && err.code === 'generator_not_configured' ? String(err.body?.setting ?? '') : null;
};

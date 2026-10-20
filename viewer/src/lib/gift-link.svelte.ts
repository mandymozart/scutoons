// The gift link of a newsletter (#66, agents/research/CAMPAIGN-GIFTS.md): `?gift=<token>` (after the one-click sign-in
// of `?signin=`, lib/newsletter-signin.ts) opens the gift sheet (components/items/gift-sheet.svelte, mounted once in
// the root layout).
let token = $state<string | null>(null);

export const giftLink = {
	get token() {
		return token;
	},
	open(next: string) {
		token = next;
	},
	close() {
		token = null;
	}
};

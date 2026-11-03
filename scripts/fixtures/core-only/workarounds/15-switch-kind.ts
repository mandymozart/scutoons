// workaround 15: a switch on the node kind
export function gainOf(n: { kind: string }) {
	switch (n.kind) {
		case 'mixer':
			return 1;
		case 'fm':
			return 0.5;
	}
	return 0;
}

// workaround 5: a node() helper hides the kind key
const node = (kind: string, id: string) => ({ id, type: kind });
export const NODES = [node('fm', 'voice'), node('mixer', 'mix')];

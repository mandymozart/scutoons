// The creators' use cases and their tutorials (/docs), read from creator/docs/tutorials/<phase>/*.md at build time.
import { parseGroups } from './markdown.ts';

const files = import.meta.glob('../../../docs/tutorials/*/*.md', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

export const GROUPS = parseGroups(files);

// The catalog kinds the admin edits ("Shop & codes", owner 2026-10-12 – CATALOG-IN-CREATOR D6): items, recipes,
// albums, products and editions are the creator's (its catalog page, every entry its own document); the featured
// products stay on the Homepage page.
import type { EditedKind } from '@scutoons/catalog';

export const ADMIN_KINDS = ['toons', 'permissions', 'code-types', 'meta'] as const satisfies readonly EditedKind[];
export type AdminKind = (typeof ADMIN_KINDS)[number];

// Types of vectors-core.mjs for the SCUTOONS MCP (tools/mcp/src/vectors.ts)
export declare const VECTORS_DIR: string;
export declare function shrink(svg: string): string;
export interface VectorShapeBody {
	title: string;
	tags: string[];
	shape: { tile: 'x' | 'y' | null; mask: boolean; colors: string[]; source: Record<string, unknown> | null; derived: { from: string; rotate?: number } | null };
}
export interface VectorImportIO {
	find(key: string): Promise<null | { id: string; sha256: string | null; title: string; tags: string[]; shape: object | null }>;
	upload(bytes: Buffer, key: string, title: string, tags: string[]): Promise<string>;
	replaceFile?: (bytes: Buffer, key: string) => Promise<void>;
	save(id: string, body: VectorShapeBody): Promise<void>;
	dryRun?: boolean;
	only?: Set<string>;
	limit?: number;
	log?: (line: string) => void;
}
export interface VectorImportResult {
	shapes: number;
	upload: number;
	replace: number;
	update: number;
	unchanged: number;
	conflict: string[];
	collision: string[];
	kb: { before: number; after: number };
	written: number;
	failed: string[];
	left: number;
}
export declare function importVectors(io: VectorImportIO): Promise<VectorImportResult>;

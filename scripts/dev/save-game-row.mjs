// The test user's save game (progress row `scutoons`, agents/DATA.md "The save game") as it was, put back exactly – for
// the browser checks that read as the member. Through the database: PUT /api/progress/scutoons/ MERGES (checkpoints and
// history are never removed by a write), so an API write can't restore a state.
import { execFileSync } from 'node:child_process';

const DB = process.env.SCUTOONS_DB_NAME ?? 'scutoons';
const CONTAINER = process.env.SCUTOONS_DB_CONTAINER ?? 'scutoons-mariadb-1';
const sql = (query) => execFileSync('docker', ['exec', '-i', CONTAINER, 'sh', '-c', `mariadb -uroot -p"$MARIADB_ROOT_PASSWORD" -N -r ${DB}`], { input: query }).toString().trim();
const quote = (s) => (s === null ? 'NULL' : `'${String(s).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`);

/** the member's save game row now (null = none) – `restore()` puts exactly it back; `clear()` = a fresh reader */
export function saveGameRow(email) {
	const userId = sql(`SELECT id FROM users WHERE email = ${quote(email)}`);
	const read = () => sql(`SELECT CONCAT(updated_at, CHAR(9), state) FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`) || null;
	const before = read();
	const del = () => sql(`DELETE FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
	return {
		userId,
		/** the stored save game (parsed) or null */
		state: () => {
			const row = read();
			return row ? JSON.parse(row.slice(row.indexOf('\t') + 1)) : null;
		},
		/** no save game: the next read is a fresh reader's */
		clear: del,
		/** sets the save game (a fresh one's fields filled in) */
		set(state, updatedAt = Date.now()) {
			del();
			sql(`INSERT INTO progress (user_id, toon_id, state, updated_at) VALUES (${quote(userId)}, 'scutoons', ${quote(JSON.stringify(state))}, ${Number(updatedAt)})`);
		},
		/** puts the row back as it was; true when it is */
		restore() {
			del();
			if (before) {
				const tab = before.indexOf('\t');
				sql(`INSERT INTO progress (user_id, toon_id, state, updated_at) VALUES (${quote(userId)}, 'scutoons', ${quote(before.slice(tab + 1))}, ${Number(before.slice(0, tab))})`);
			}
			return read() === before;
		}
	};
}

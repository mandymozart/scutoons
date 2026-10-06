# SCUTOONS – project entry point

Read `agents/HANDOVER.md` first (short: current state, working rules, UI rules, open decisions). Then only the agents/
file for what you touch: PLAN.md (the plan), CUTOVER.md (production cutover + post-beta list 10.x), DESIGN.md (UI),
PLATFORM.md / DATA.md / ITEMS.md / ASSETS.md (system), DOCKING.md (panels, sidebar, dividers). `agents/MEMORY.md` is long – grep it, don't read it whole.

Keep context small: prefer targeted reads/greps over whole files; delegate broad work to agents on their own branches.

**Local dev and tests run on MariaDB in Docker – the only database, no SQLite** (owner 2026-10-05; production is MariaDB
10.11): `npm run dev:db` (container + migrate + production seed `server/db/prod-seed.sql` + sandbox + API), PHPUnit with
`npm run dev:db test` (database `<name>_test`), migrations as their `.mysql.sql`. A worktree's own database:
`SCUTOONS_DB_NAME=scutoons_<x>`. Don't reset or reseed the owner's running database without asking.

# SCUTOONS – project entry point

Read `agents/HANDOVER.md` first (short: current state, working rules, UI rules, open decisions). Then only the agents/
file for what you touch: PLAN.md (the plan), CUTOVER.md (production cutover + post-beta list 10.x), DESIGN.md (UI),
PLATFORM.md / DATA.md / ITEMS.md / ASSETS.md (system). `agents/MEMORY.md` is long – grep it, don't read it whole.

Keep context small: prefer targeted reads/greps over whole files; delegate broad work to agents on their own branches.

**Local dev and tests run on MariaDB in Docker, not SQLite** (owner 2026-10-05: "important for all local dev"; production
is MariaDB 10.11): `npm run dev:db` (start + migrate + switch the stack), PHPUnit with `npm run dev:db test` (database
`scutoons_test`, doesn't switch the running stack), migrations as their `.mysql.sql`. SQLite only as a fallback when
Docker isn't available – say so in the hand-back. Don't switch the owner's running stack without asking.

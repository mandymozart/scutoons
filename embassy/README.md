# embassy – embassy.scutoons.com

The partner network of Top Toy / SCUTOONS (plan and strategy: [`agents/EMBASSY.md`](../agents/EMBASSY.md)):
organisations (venues, festivals, radios, record shops, agents, press …), their people, one lead per organisation ×
track (live, vinyl, booking, promo) with stage / owner / next check date, and the history of mails, calls and notes.

Static SvelteKit SPA like the admin: single sign-on (`@scutoons/auth` `AuthGate`, email + one-time code), role gate `liaison` / `ambassador` (`admin`
counts as ambassador), phone first (navigation at the bottom below 768px). API: `/api/embassy/*` of the same server
(api.scutoons.com, `server/docs/API.md` "Embassy").

| Route | What |
|---|---|
| `/` | Today: leads due today or earlier (mine / nobody's / everyone's / a team member's), tap-to-call, one tap per call outcome |
| `/leads` | all organisations, search + filters in the URL (track, stage, type, country, tier, owner, due) |
| `/orgs/new` | new organisation + tracks (duplicate check: same name in the same city) |
| `/orgs/<id>` | facts, leads per track (stage, next date, owner), log mail/call/DM/meeting/note, people, history |
| `/import` | ambassadors: upload the file of `scripts/embassy/toplist_import.py`, dry run, import |

Dev: `npm run dev:embassy` → http://localhost:13000. `.env.development` (gitignored, see `.env.example`):
`VITE_API_URL` (default `http://localhost:9000/api`), `VITE_VIEWER_URL`. The API's `ALLOWED_ORIGINS` needs
`http://localhost:13000`. Give an account a role: admin app → Users, or `php server/bin/console roles grant <email> liaison`.

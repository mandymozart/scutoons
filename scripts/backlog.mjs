// The backlog (agents/RULES.md "Backlog"): open work = GitHub issues, priority = the status on the project board
// "SCUTOONS". Needs `gh` signed in with the project scope (once: `gh auth refresh -s project`).
// Statuses (owner 2026-10-20, #105): Backlog · Proposals · In progress · Review on staging · Deployed · Done.
//   npm run backlog                      In progress, Review on staging, Backlog in the board's order + how many wait
//                                        in Proposals (an open issue without a status counts there)
//   npm run backlog -- all               every status (Deployed / Done = closed issues on the board)
//   npm run backlog -- set <N> <status>  puts issue #N on the board with that status ("none" clears it)
import { execFileSync } from 'node:child_process';

const OWNER = 'mandymozart';
const REPO = `${OWNER}/scutoons`;
const BOARD = 'SCUTOONS';
const ORDER = ['Backlog', 'Proposals', 'In progress', 'Review on staging', 'Deployed', 'Done'];
const SHORT = ['In progress', 'Review on staging', 'Backlog'];
const CLOSED = ['Deployed', 'Done'];
const INBOX = 'Proposals';

const gh = (...args) => execFileSync('gh', args, { encoding: 'utf8', maxBuffer: 64 << 20 }).trim();
const ghJson = (...args) => JSON.parse(gh(...args, '--format', 'json'));

function board() {
  let projects;
  try {
    projects = ghJson('project', 'list', '--owner', OWNER).projects;
  } catch {
    throw new Error('gh cannot read projects – run once: gh auth refresh -s project');
  }
  const p = projects.find((x) => x.title === BOARD);
  if (!p) throw new Error(`no project "${BOARD}" for ${OWNER}`);
  const status = ghJson('project', 'field-list', String(p.number), '--owner', OWNER).fields.find((f) => f.name === 'Status');
  return { number: String(p.number), id: p.id, url: p.url, status };
}

function set(n, wanted) {
  const b = board();
  const item = ghJson('project', 'item-add', b.number, '--owner', OWNER, '--url', `https://github.com/${REPO}/issues/${n}`);
  const edit = ['project', 'item-edit', '--id', item.id, '--project-id', b.id, '--field-id', b.status.id];
  let name = 'no status';
  if (wanted.toLowerCase() === 'none') {
    gh(...edit, '--clear');
  } else {
    const option = b.status.options.find((o) => o.name.toLowerCase() === wanted.toLowerCase());
    if (!option) throw new Error(`unknown status "${wanted}" – one of: ${b.status.options.map((o) => o.name).join(', ')}, none`);
    gh(...edit, '--single-select-option-id', option.id);
    name = option.name;
  }
  console.log(`#${n} → ${name}`);
}

function list(all) {
  const b = board();
  // item-list returns the board's order – inside Backlog that order is the owner's priority
  const items = ghJson('project', 'item-list', b.number, '--owner', OWNER, '-L', '2000').items
    .filter((it) => it.content?.type === 'Issue' && it.content.repository === REPO);
  const rank = new Map(items.map((it, i) => [it.content.number, i]));
  const statusOf = new Map(items.map((it) => [it.content.number, it.status ?? '']));
  const open = JSON.parse(gh('issue', 'list', '--repo', REPO, '-s', 'open', '-L', '2000', '--json', 'number,title,labels,milestone'));
  const columns = new Map(ORDER.map((s) => [s, []]));
  for (const issue of open) {
    const s = statusOf.get(issue.number);
    columns.get(ORDER.includes(s) && !CLOSED.includes(s) ? s : INBOX).push(issue);
  }
  if (all) {
    for (const it of items) {
      if (CLOSED.includes(it.status)) columns.get(it.status).push({ number: it.content.number, title: it.content.title, labels: (it.labels ?? []).map((name) => ({ name })) });
    }
  }
  const line = (i) => {
    const labels = i.labels.map((l) => l.name.replace(/^area:/, '')).filter((l) => !['bug', 'feature', 'chore'].includes(l));
    const kind = i.labels.find((l) => ['bug', 'feature', 'chore'].includes(l.name))?.name ?? '';
    const ms = i.milestone ? `  [${i.milestone.title}]` : '';
    return `  #${String(i.number).padEnd(4)} ${i.title}  · ${[kind, ...labels].join(' ')}${ms}`;
  };
  const shown = all ? ORDER : SHORT;
  const byBoard = (a, c) => (rank.get(a.number) ?? Infinity) - (rank.get(c.number) ?? Infinity) || a.number - c.number;
  for (const s of shown) {
    console.log(`${s} (${columns.get(s).length})`);
    for (const i of columns.get(s).sort(byBoard)) console.log(line(i));
  }
  const rest = ORDER.filter((s) => !shown.includes(s) && !CLOSED.includes(s)).map((s) => `${s} ${columns.get(s).length}`);
  if (rest.length) console.log(`\n${rest.join(' · ')}  (npm run backlog -- all)`);
  const owner = open.filter((i) => i.labels.some((l) => l.name === 'owner' || l.name === 'decide')).length;
  console.log(`Waiting on the owner: ${owner} (gh issue list --assignee @me) · board: ${b.url}`);
}

const [cmd, n, ...status] = process.argv.slice(2);
try {
  if (cmd === 'set' && /^\d+$/.test(n ?? '') && status.length) set(n, status.join(' '));
  else if (!cmd || cmd === 'all') list(cmd === 'all');
  else throw new Error('usage: npm run backlog [-- all | -- set <issue number> <status>]');
} catch (e) {
  console.error(`backlog: ${e.message}`);
  process.exit(1);
}

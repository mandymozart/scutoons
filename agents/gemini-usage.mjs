/**
 * Record Gemini API token usage to the agents tool (POST /api/usage, remote DB).
 * No local ledger: costs are logged to the remote agents/costs database, not to
 * this repo's agents/costs.json. Works with Interactions API (usage: total_*_tokens)
 * and generateContent (usageMetadata).
 *
 * One entry per session; the API uses replace_sessions: true, so a second call
 * for the same session replaces its earlier rows.
 *
 * Example:
 *   import { recordGeminiUsage } from './gemini-usage.mjs';
 *   const response = await client.interactions.create({ ... });
 *   await recordGeminiUsage({
 *     session: 'my-session-id',
 *     model: 'gemini-4-hydrogen',
 *     usage: response.usage,
 *     issues: [242],
 *     title: 'Prototyping audio pipeline'
 *   });
 */
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PYTHON_SCRIPT = resolve(__dirname, '../../agents/costs/gemini_usage.py');

export async function recordGeminiUsage({
  session,
  model,
  usage = {},
  issues = [],
  role = 'worker',
  title = null,
  tier = 'std',
  date = null,
  note = null,
  project = 'scutoons',
  insecure = false,
}) {
  const cleanIssues = (issues || [])
    .map(Number)
    .filter((n) => Number.isInteger(n) && n > 0);

  const args = [
    PYTHON_SCRIPT,
    '--project', project,
    '--session', session,
    '--model', model,
    '--tier', tier,
    '--role', role,
  ];

  if (cleanIssues.length > 0) {
    args.push('--issues', cleanIssues.join(','));
  } else {
    args.push('--no-issue');
  }

  if (title) args.push('--title', title);
  if (note) args.push('--note', note);
  if (date) args.push('--date', date);
  if (insecure) args.push('--insecure');

  const rawInput = usage.total_input_tokens ?? usage.promptTokenCount ?? usage.prompt_token_count ?? 0;
  const cached = usage.total_cached_tokens ?? usage.cachedContentTokenCount ?? usage.cached_content_token_count ?? 0;
  const rawOutput = usage.total_output_tokens ?? usage.candidatesTokenCount ?? usage.candidates_token_count ?? 0;
  const thinking = usage.total_thought_tokens ?? usage.thoughtTokenCount ?? usage.thought_token_count ?? 0;

  args.push('--input', String(rawInput));
  args.push('--cached', String(cached));
  args.push('--output', String(rawOutput));
  args.push('--thinking', String(thinking));

  return new Promise((res, rej) => {
    const proc = spawn('python', args, { stdio: 'inherit' });
    proc.on('close', (code) => {
      if (code === 0) res();
      else rej(new Error(`gemini_usage.py exited with code ${code}`));
    });
  });
}

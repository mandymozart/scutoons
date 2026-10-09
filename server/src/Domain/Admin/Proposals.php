<?php
declare(strict_types=1);

namespace Scutoons\Domain\Admin;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use Throwable;

/**
 * Admin proposals (agents/MCP.md "Draft → link", owner 2026-10-09): an admin's own AI proposes changes – catalog
 * fields, calendar entries, an asset file swapped in place – as a stored proposal (the change list the MCP server's
 * propose_changes checked; shared/api AdminChange). The admin opens its link (/proposals/<id> in the admin app), reads
 * before → after and presses Apply: the admin app makes the changes through the normal admin API (catalog history +
 * undo, the calendar as a draft) and marks the proposal applied – or Dismiss. Nothing changes here but the proposal's
 * status. Every admin sees every proposal (open / applied / dismissed).
 */
final class Proposals
{
    public const STATUSES = ['open', 'applied', 'dismissed'];
    public const TITLE_MAX = 200;
    /** the change list's largest JSON */
    public const MAX_BYTES = 262144;
    private const JSON_FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE | JSON_PRESERVE_ZERO_FRACTION;
    private const SELECT = 'SELECT p.*, a.email AS author_email, d.email AS decider_email FROM admin_proposals p LEFT JOIN users a ON a.id = p.created_by LEFT JOIN users d ON d.id = p.decided_by';

    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    /** Before migration 2026-10-09-mcp-drafts the endpoints answer 503 */
    private function requireAvailable(): void
    {
        try {
            $this->db->value('SELECT COUNT(*) FROM admin_proposals WHERE 1 = 0');
        } catch (Throwable) {
            throw new ApiError(503, 'not_available', 'Proposals are not set up yet (migration 2026-10-09-mcp-drafts).');
        }
    }

    /**
     * `{ title, changes: [ … ], calendar_base?, source? }` → the stored proposal
     * @param array<string, mixed> $data
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function create(array $data, array $user): array
    {
        $this->requireAvailable();
        $title = trim((string) preg_replace('/\s+/', ' ', (string) ($data['title'] ?? '')));
        if ($title === '') throw new ApiError(400, 'title_required', 'A proposal needs a title.', ['field' => 'title']);
        if (mb_strlen($title) > self::TITLE_MAX) throw new ApiError(400, 'title_too_long', 'At most ' . self::TITLE_MAX . ' characters.', ['field' => 'title']);
        $changes = $data['changes'] ?? null;
        if (!is_array($changes) || $changes === [] || !array_is_list($changes)) throw new ApiError(400, 'changes_required', 'A proposal needs a list of changes.', ['field' => 'changes']);
        foreach ($changes as $i => $change) {
            if (!is_array($change) || !in_array($change['type'] ?? null, ['catalog', 'calendar', 'asset'], true)) {
                throw new ApiError(400, 'invalid_change', "Change $i: type must be catalog, calendar or asset.", ['field' => "changes.$i"]);
            }
        }
        $json = (string) json_encode($changes, self::JSON_FLAGS);
        if (strlen($json) > self::MAX_BYTES) throw new ApiError(413, 'too_large', 'The change list is too large – split it into several proposals.');
        $base = isset($data['calendar_base']) && is_int($data['calendar_base']) ? $data['calendar_base'] : null;
        $source = ($data['source'] ?? '') === 'agent' ? 'agent' : 'person';
        $id = Ids::uuid7();
        $this->db->run(
            'INSERT INTO admin_proposals (id, title, changes, calendar_base, source, status, created_by, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)',
            [$id, $title, $json, $base, $source, 'open', (string) $user['id'], $this->clock->utc()]
        );
        return $this->get($id);
    }

    /**
     * Newest first, without the change lists (`count` = how many changes)
     * @return list<array<string, mixed>>
     */
    public function list(?string $status = null): array
    {
        $this->requireAvailable();
        if ($status !== null && !in_array($status, self::STATUSES, true)) throw new ApiError(400, 'invalid_status', 'status: open, applied or dismissed.');
        $rows = $status !== null
            ? $this->db->all(self::SELECT . ' WHERE p.status = ? ORDER BY p.created_at DESC LIMIT 300', [$status])
            : $this->db->all(self::SELECT . ' ORDER BY p.created_at DESC LIMIT 300');
        return array_map(function (array $r): array {
            $item = $this->proposal($r);
            $item['count'] = count($item['changes']);
            unset($item['changes']);
            return $item;
        }, $rows);
    }

    /** @return array<string, mixed> */
    public function get(string $id): array
    {
        $this->requireAvailable();
        $row = $this->db->one(self::SELECT . ' WHERE p.id = ?', [$id]) ?? throw new ApiError(404, 'not_found', 'No such proposal.');
        return $this->proposal($row);
    }

    /**
     * Applied (the admin app made the changes) or dismissed – once, and only while open
     * @param array<string, mixed> $user
     * @return array<string, mixed>
     */
    public function decide(string $id, string $status, array $user): array
    {
        $this->requireAvailable();
        if (!in_array($status, ['applied', 'dismissed'], true)) throw new ApiError(400, 'invalid_status', 'status: applied or dismissed.');
        $current = $this->get($id);
        if ($current['status'] !== 'open') throw new ApiError(409, 'decided', "The proposal is {$current['status']} already.");
        $this->db->run('UPDATE admin_proposals SET status = ?, decided_by = ?, decided_at = ? WHERE id = ? AND status = ?', [$status, (string) $user['id'], $this->clock->utc(), $id, 'open']);
        return $this->get($id);
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private function proposal(array $r): array
    {
        $changes = json_decode((string) $r['changes'], true);
        return [
            'id' => (string) $r['id'],
            'title' => (string) $r['title'],
            'changes' => is_array($changes) ? $changes : [],
            'calendar_base' => $r['calendar_base'] !== null ? (int) $r['calendar_base'] : null,
            'source' => (string) $r['source'],
            'status' => (string) $r['status'],
            'created_by' => (string) $r['created_by'],
            'author' => (string) ($r['author_email'] ?? ''),
            'created_at' => (string) $r['created_at'],
            'decided_by' => (string) ($r['decider_email'] ?? ''),
            'decided_at' => $r['decided_at'] !== null ? (string) $r['decided_at'] : null,
        ];
    }
}

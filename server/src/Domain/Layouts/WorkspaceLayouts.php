<?php
declare(strict_types=1);

namespace Scutoons\Domain\Layouts;

use PDOException;
use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Time;

/**
 * A member's saved creator workspace layouts (#827, migration 2026-11-24-workspace-layouts): the state Reset goes back to
 * and a new device starts from; a row with a name is an own workspace made from another one. A craft's default layout per
 * workspace (`craft_layouts`, configuration) is what a member of that craft starts from before saving.
 *
 * The layout is the creator's floating panels' own JSON. The server checks only its outer shape – an object with no keys
 * but `places`, `tabs`, `stage`, each an object – and its size; the creator's readers check the inside when they load it,
 * as they do for what the browser kept. Rights: a member reads and changes only their own rows.
 */
final class WorkspaceLayouts
{
    public const KEYS = ['places', 'tabs', 'stage'];
    /** the encoded layout's limit in bytes */
    public const MAX_BYTES = 32768;
    /** rows per member (saved workspaces and own ones) */
    public const MAX_ROWS = 60;
    public const MAX_NAME = 80;
    private const ID = '/^[a-z][a-z0-9_-]{0,47}$/';

    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    /**
     * GET /api/me/layouts: the member's saved layouts, oldest first, and their crafts' defaults (per workspace the first
     * craft in the crafts' order that has one). `available` false: the migration has not run – nothing saved yet.
     * @return array{available: bool, layouts: list<array<string, mixed>>, defaults: list<array{workspace: string, craft: string, layout: mixed}>}
     */
    public function of(string $userId): array
    {
        try {
            $rows = $this->db->all('SELECT workspace, kind, name, layout, saved_at FROM member_layouts WHERE user_id = ? ORDER BY saved_at, workspace', [$userId]);
        } catch (PDOException $error) {
            if (self::missing($error)) return ['available' => false, 'layouts' => [], 'defaults' => []];
            throw $error;
        }
        return ['available' => true, 'layouts' => array_map(self::json(...), $rows), 'defaults' => $this->defaults($userId)];
    }

    /**
     * PUT /api/me/layouts/{workspace} { layout, name?, kind? }: the workspace's saved state. A name (with `kind`, the
     * workspace it is made from) makes or renames an own workspace; without one an own workspace keeps its name.
     * @param array<string, mixed> $body
     * @return array<string, mixed>
     */
    public function save(string $userId, string $workspace, array $body): array
    {
        self::checkId($workspace, 'bad_workspace');
        $layout = self::layoutJson($body['layout'] ?? null);
        $name = array_key_exists('name', $body) ? self::name($body['name']) : null;
        $kind = $body['kind'] ?? null;
        if ($kind !== null) {
            if (!is_string($kind)) throw new ApiError(400, 'bad_kind', 'kind must be a workspace id.');
            self::checkId($kind, 'bad_kind');
            if ($kind === $workspace) throw new ApiError(400, 'bad_kind', 'An own workspace is made from another workspace.');
        }
        return $this->db->transaction(function () use ($userId, $workspace, $layout, $name, $kind) {
            $was = $this->row($userId, $workspace, true);
            if ($was === null) {
                $count = (int) $this->db->value('SELECT COUNT(*) FROM member_layouts WHERE user_id = ?', [$userId]);
                if ($count >= self::MAX_ROWS) throw new ApiError(409, 'too_many_layouts', 'Too many saved workspaces – delete one first.', ['max' => self::MAX_ROWS]);
                if ($name !== null && $kind === null) throw new ApiError(400, 'bad_kind', 'A new own workspace needs the workspace it is made from (kind).');
                if ($name === null && $kind !== null) throw new ApiError(400, 'bad_name', 'An own workspace needs a name.');
                $this->db->run(
                    'INSERT INTO member_layouts (user_id, workspace, kind, name, layout, saved_at) VALUES (?, ?, ?, ?, ?, ?)',
                    [$userId, $workspace, $kind, $name, $layout, $this->clock->utc()]
                );
            } else {
                if ($name !== null && $was['name'] === null) throw new ApiError(400, 'bad_name', 'This workspace is not an own one – it keeps its name.');
                $this->db->run(
                    'UPDATE member_layouts SET layout = ?, name = COALESCE(?, name), saved_at = ? WHERE user_id = ? AND workspace = ?',
                    [$layout, $name, $this->clock->utc(), $userId, $workspace]
                );
            }
            return self::json((array) $this->row($userId, $workspace));
        });
    }

    /** DELETE /api/me/layouts/{workspace}: a creator workspace back to its default, an own workspace gone. False: nothing was saved */
    public function delete(string $userId, string $workspace): bool
    {
        self::checkId($workspace, 'bad_workspace');
        try {
            return $this->db->run('DELETE FROM member_layouts WHERE user_id = ? AND workspace = ?', [$userId, $workspace])->rowCount() > 0;
        } catch (PDOException $error) {
            if (self::missing($error)) throw self::notAvailable();
            throw $error;
        }
    }

    /** @return array<string, mixed>|null */
    private function row(string $userId, string $workspace, bool $lock = false): ?array
    {
        try {
            $rows = $this->db->all('SELECT workspace, kind, name, layout, saved_at FROM member_layouts WHERE user_id = ? AND workspace = ?' . ($lock ? ' FOR UPDATE' : ''), [$userId, $workspace]);
        } catch (PDOException $error) {
            if (self::missing($error)) throw self::notAvailable();
            throw $error;
        }
        return $rows[0] ?? null;
    }

    /** @return list<array{workspace: string, craft: string, layout: mixed}> */
    private function defaults(string $userId): array
    {
        try {
            $rows = $this->db->all(
                'SELECT l.workspace, l.craft_id, l.layout FROM craft_layouts l JOIN member_crafts m ON m.craft_id = l.craft_id AND m.user_id = ? JOIN crafts c ON c.id = l.craft_id ORDER BY c.position, c.id, l.workspace',
                [$userId]
            );
        } catch (PDOException $error) {
            if (self::missing($error)) return [];
            throw $error;
        }
        $out = [];
        foreach ($rows as $r) {
            $workspace = (string) $r['workspace'];
            if (isset($out[$workspace])) continue;
            $out[$workspace] = ['workspace' => $workspace, 'craft' => (string) $r['craft_id'], 'layout' => json_decode((string) $r['layout'], true)];
        }
        return array_values($out);
    }

    /** The layout checked and encoded: an object of `places`, `tabs`, `stage` (each an object, any may be missing), at most MAX_BYTES */
    private static function layoutJson(mixed $layout): string
    {
        if (!self::isObject($layout)) throw new ApiError(400, 'bad_layout', 'layout must be an object { places, tabs, stage }.');
        /** @var array<string, mixed> $layout */
        foreach ($layout as $key => $value) {
            if (!in_array($key, self::KEYS, true)) throw new ApiError(400, 'bad_layout', 'layout has only places, tabs and stage.', ['key' => (string) $key]);
            if (!self::isObject($value)) throw new ApiError(400, 'bad_layout', "layout.$key must be an object.", ['key' => (string) $key]);
        }
        $json = json_encode((object) $layout, JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR);
        if (strlen($json) > self::MAX_BYTES) throw new ApiError(413, 'layout_too_large', 'The layout is too large.', ['max' => self::MAX_BYTES]);
        return $json;
    }

    /** a JSON object as PHP decodes it: an array with string keys, or an empty one ({}) */
    private static function isObject(mixed $value): bool
    {
        return is_array($value) && ($value === [] || !array_is_list($value));
    }

    private static function name(mixed $name): string
    {
        $name = is_string($name) ? trim(preg_replace('/\s+/u', ' ', $name) ?? '') : '';
        if ($name === '' || mb_strlen($name) > self::MAX_NAME) throw new ApiError(400, 'bad_name', 'A name of 1 to ' . self::MAX_NAME . ' characters.');
        return $name;
    }

    private static function checkId(string $id, string $code): void
    {
        if (preg_match(self::ID, $id) !== 1) throw new ApiError(400, $code, 'A workspace id: lowercase letters, digits, - and _, at most 48.');
    }

    /**
     * @param array<string, mixed> $r
     * @return array<string, mixed>
     */
    private static function json(array $r): array
    {
        return [
            'workspace' => (string) $r['workspace'],
            'kind' => $r['kind'] === null ? null : (string) $r['kind'],
            'name' => $r['name'] === null ? null : (string) $r['name'],
            'layout' => json_decode((string) $r['layout'], true),
            'saved_at' => Time::iso($r['saved_at'] ?? null),
        ];
    }

    private static function notAvailable(): ApiError
    {
        return new ApiError(503, 'not_available', 'Saved layouts are not set up yet.');
    }

    /** MariaDB 42S02 (table) / 42S22 (column): the migration has not run yet */
    private static function missing(PDOException $error): bool
    {
        return in_array($error->getCode(), ['42S02', '42S22'], true) || preg_match('/Base table or view not found|Unknown column/i', $error->getMessage()) === 1;
    }
}

<?php
declare(strict_types=1);

namespace Scutoons\Domain\Ownership;

use Scutoons\Domain\Access\Permissions;
use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\Admin\CodeAdmin;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Certificates\CertificateVerifier;
use Scutoons\Domain\Codes\CodeRules;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Mail\MailFailed;
use Scutoons\Domain\Mail\Mailer;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Domain\RateLimit\RateLimiter;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Storage\FileStore;
use Scutoons\Support\Ids;
use Scutoons\Support\Time;
use PDOException;
use Throwable;

/**
 * Ownership claims (agents/ITEMS.md "Ownership, claims, gifting"; owner 2026-10-02): a buyer who did not register their
 * piece in time finds it in someone else's account → "This is mine" → a claim with a statement and OPTIONAL evidence
 * (any receipt, photo, the signed certificate – no Bandcamp receipt required). Admins look at it and either move the
 * code to the claimant (OwnershipTransfers, kind `claim`) or reject it. Mails: claimant (received / rejected), admins
 * (new claim), previous + new owner (moved).
 *
 * Reader: file, add evidence files, list mine, withdraw. One open claim per claimant + code; CLAIM_MAX_PER_DAY claims per
 * reader. The claimant never learns who owns the code; the owner is only shown to admins (claim page).
 */
final class OwnershipClaims
{
    public const STATUSES = ['open', 'transferred', 'rejected', 'withdrawn'];
    public const SORTABLE = ['created_at', 'decided_at', 'status', 'code'];
    public const FILTERS = ['status', 'code', 'user'];
    /** accepted evidence types → stored extension (checked by the file's first bytes, not the client's word) */
    public const FILE_TYPES = ['image/jpeg' => 'jpg', 'image/png' => 'png', 'image/webp' => 'webp', 'application/pdf' => 'pdf'];
    private const STATEMENT_MIN = 10;
    private const STATEMENT_MAX = 4000;

    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Users $users,
        private readonly CodeService $codes,
        private readonly CodeAdmin $codeAdmin,
        private readonly Permissions $permissions,
        private readonly OwnershipTransfers $transfers,
        private readonly CertificateVerifier $certificates,
        private readonly RateLimiter $limits,
        private readonly FileStore $files,
        private readonly Mailer $mailer,
        private readonly Settings $settings,
    ) {
    }

    // ── Reader ────────────────────────────────────────────────────────────────────────────────────────────

    /**
     * POST /claims/ { code, statement } → the new claim. 400 invalid_code / statement_required · 404 not_found ·
     * 422 not_claimable · 409 not_registered (nobody owns it: just add it) / already_yours / claim_open { claim } ·
     * 429 rate_limited.
     * @param array<string, mixed> $user
     * @param array<string, mixed> $body
     * @return array<string, mixed>
     */
    public function file(array $user, array $body): array
    {
        $this->transfers->requireTables();
        $userId = (string) $user['id'];
        $code = CodeRules::normalize($body['code'] ?? null) ?? throw new ApiError(400, 'invalid_code', 'Code missing or malformed.');
        $statement = is_string($body['statement'] ?? null) ? trim($body['statement']) : '';
        if (mb_strlen($statement) < self::STATEMENT_MIN) {
            throw new ApiError(400, 'statement_required', 'Tell us in a few words why it is yours (where and when you got it).', ['min' => self::STATEMENT_MIN]);
        }
        $statement = mb_substr($statement, 0, self::STATEMENT_MAX);
        $row = $this->codes->find($code) ?? throw new ApiError(404, 'not_found', 'This code does not exist.');
        $code = (string) $row['code'];
        if (!$this->permissions->claimable((string) $row['type'])) throw new ApiError(422, 'not_claimable', 'Nobody can own this code.');
        if ($row['claimed_by'] === $userId) throw new ApiError(409, 'already_yours', 'This code is already in your account.');
        if (!CodeRules::isClaimed($row)) throw new ApiError(409, 'not_registered', 'Nobody has registered this code yet – add it to your account.');
        $open = $this->db->one("SELECT * FROM ownership_claims WHERE code = ? AND claimant_id = ? AND status = 'open'", [$code, $userId]);
        if ($open) throw new ApiError(409, 'claim_open', 'You already have an open claim for this code.', ['claim' => $this->readerJson($open)]);
        $this->limits->guard("ownclaim:$userId", $this->settings->int('CLAIM_MAX_PER_DAY'), 86400, 'Too many claims today – try again tomorrow.');

        $id = Ids::uuid();
        $this->db->transaction(function () use ($id, $code, $userId, $row, $statement) {
            $this->limits->hit("ownclaim:$userId");
            $this->db->run('INSERT INTO ownership_claims (id, code, claimant_id, owner_id, statement, status, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)',
                [$id, $code, $userId, $row['claimed_by'], $statement, 'open', $this->clock->utc()]);
        });

        $locale = MailTemplates::locale($user['locale'] ?? null);
        try {
            $this->mailer->send(MailTemplates::OWNERSHIP_CLAIM_RECEIVED, (string) $user['email'], $locale,
                ['code' => $code, 'url' => rtrim($this->settings->get('APP_URL'), '/') . '/account']);
        } catch (MailFailed) {
            // logged; the claim stands
        }
        $this->mailer->sendToAdmins(MailTemplates::OWNERSHIP_CLAIM_ADMIN, [
            'code' => $code,
            'claimant' => (string) $user['email'],
            'statement' => mb_strlen($statement) > 300 ? mb_substr($statement, 0, 300) . '…' : $statement,
            'url' => rtrim($this->settings->get('ADMIN_URL'), '/') . '/claims/' . $id,
        ]);
        return $this->readerJson($this->row($id));
    }

    /**
     * POST /claims/{id}/files?name=<file name> – the raw file as the body (Content-Type image/jpeg|png|webp,
     * application/pdf). Only the claimant, only while open. 404 not_found · 409 claim_closed / too_many_files ·
     * 400 empty_file · 413 file_too_large · 415 unsupported_type.
     * @param array<string, mixed> $user
     * @return array<string, mixed> the claim (reader view) with the new file
     */
    public function addFile(array $user, string $claimId, string $bytes, string $name): array
    {
        $claim = $this->db->one('SELECT * FROM ownership_claims WHERE id = ? AND claimant_id = ?', [$claimId, (string) $user['id']])
            ?? throw new ApiError(404, 'not_found', 'Claim not found.');
        if ($claim['status'] !== 'open') throw new ApiError(409, 'claim_closed', 'This claim is closed.');
        $max = $this->settings->int('CLAIM_FILES_MAX');
        if ((int) $this->db->value('SELECT COUNT(*) FROM ownership_claim_files WHERE claim_id = ?', [$claimId]) >= $max) {
            throw new ApiError(409, 'too_many_files', "At most $max files per claim.", ['max' => $max]);
        }
        if ($bytes === '') throw new ApiError(400, 'empty_file', 'The file is empty.');
        $maxBytes = $this->settings->int('CLAIM_FILE_MAX_BYTES');
        if (strlen($bytes) > $maxBytes) throw new ApiError(413, 'file_too_large', 'The file is too big.', ['max_bytes' => $maxBytes]);
        $type = self::sniff($bytes) ?? throw new ApiError(415, 'unsupported_type', 'Photos (JPEG, PNG, WebP) or PDF only.', ['types' => array_keys(self::FILE_TYPES)]);

        $fileId = Ids::uuid();
        $filename = $claimId . '-' . substr(str_replace('-', '', $fileId), 0, 12) . '.' . self::FILE_TYPES[$type];
        $this->files->writeClaim($filename, $bytes);
        $clean = trim((string) preg_replace('/[^\p{L}\p{N} ._()-]+/u', '_', $name));
        $this->db->run('INSERT INTO ownership_claim_files (id, claim_id, filename, name, content_type, size, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)',
            [$fileId, $claimId, $filename, mb_substr($clean !== '' ? $clean : 'file.' . self::FILE_TYPES[$type], 0, 200), $type, strlen($bytes), $this->clock->utc()]);
        return $this->readerJson($this->row($claimId));
    }

    /** The real type of an evidence file from its first bytes, or null */
    public static function sniff(string $bytes): ?string
    {
        return match (true) {
            str_starts_with($bytes, "\xFF\xD8\xFF") => 'image/jpeg',
            str_starts_with($bytes, "\x89PNG\r\n\x1A\n") => 'image/png',
            strlen($bytes) >= 12 && substr($bytes, 0, 4) === 'RIFF' && substr($bytes, 8, 4) === 'WEBP' => 'image/webp',
            str_starts_with($bytes, '%PDF-') => 'application/pdf',
            default => null,
        };
    }

    /**
     * GET /claims/ – my claims, newest first
     * @return list<array<string, mixed>>
     */
    public function mine(string $userId): array
    {
        try {
            $rows = $this->db->all('SELECT * FROM ownership_claims WHERE claimant_id = ? ORDER BY created_at DESC, id DESC LIMIT 100', [$userId]);
        } catch (PDOException $error) {
            if (!OwnershipTransfers::missingTable($error)) throw $error;
            return []; // before the migration (the account page still loads)
        }
        return array_map(fn(array $r) => $this->readerJson($r), $rows);
    }

    /**
     * POST /claims/{id}/withdraw – the claimant takes back an open claim. 404 · 409 claim_closed.
     * @return array<string, mixed>
     */
    public function withdraw(string $userId, string $claimId): array
    {
        $changed = $this->db->run("UPDATE ownership_claims SET status = 'withdrawn', decided_at = ? WHERE id = ? AND claimant_id = ? AND status = 'open'",
            [$this->clock->utc(), $claimId, $userId])->rowCount();
        $row = $this->db->one('SELECT * FROM ownership_claims WHERE id = ? AND claimant_id = ?', [$claimId, $userId]) ?? throw new ApiError(404, 'not_found', 'Claim not found.');
        if ($changed === 0) throw new ApiError(409, 'claim_closed', 'This claim is closed.', ['claim' => $this->readerJson($row)]);
        return $this->readerJson($row);
    }

    /**
     * The claimant's view: no owner, no admin, no internal note
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    public function readerJson(array $row): array
    {
        return [
            'id' => $row['id'],
            'code' => $row['code'],
            'status' => $row['status'],
            'statement' => $row['statement'],
            'files' => array_map(fn(array $f) => ['id' => $f['id'], 'name' => $f['name'], 'type' => $f['content_type'], 'size' => (int) $f['size']], $this->fileRows((string) $row['id'])),
            'created_at' => Time::iso($row['created_at']),
            'decided_at' => Time::iso($row['decided_at']),
        ];
    }

    // ── Admin ─────────────────────────────────────────────────────────────────────────────────────────────

    /** Open claims (the admin nav badge) */
    public function openCount(): int
    {
        try {
            return (int) $this->db->value("SELECT COUNT(*) FROM ownership_claims WHERE status = 'open'");
        } catch (PDOException $error) {
            if (!OwnershipTransfers::missingTable($error)) throw $error;
            return 0;
        }
    }

    /**
     * GET /admin/claims – filters status, code, user (claimant or owner at filing), q (part of the code or the
     * claimant's email); sort created_at (default -created_at), decided_at, status, code.
     * @return array{items: list<mixed>, pagination: array{total: int, page: int, pageSize: int}}
     */
    public function list(ListQuery $query): array
    {
        $conditions = [];
        $params = [];
        if (($status = $query->filter('status')) !== null) {
            if (!in_array($status, self::STATUSES, true)) throw new ApiError(400, 'invalid_status', 'Unknown status filter.', ['statuses' => self::STATUSES]);
            $conditions[] = 'c.status = ?';
            $params[] = $status;
        }
        if (($code = $query->filter('code')) !== null) {
            $conditions[] = 'c.code = ?';
            $params[] = $code;
        }
        if (($user = $query->filter('user')) !== null) {
            $conditions[] = '(c.claimant_id = ? OR c.owner_id = ?)';
            array_push($params, $user, $user);
        }
        if ($query->q !== '') {
            // the admin search box ignores case on every engine (a filter, not a lookup: codes match exactly everywhere else)
            $conditions[] = "(LOWER(c.code) LIKE ? ESCAPE '!' OR u.email LIKE ? ESCAPE '!')";
            array_push($params, Database::likeContains(strtolower($query->q)), Database::likeContains(strtolower($query->q)));
        }
        try {
            $this->db->value('SELECT 1 FROM ownership_claims LIMIT 1');
        } catch (PDOException $error) {
            if (!OwnershipTransfers::missingTable($error)) throw $error;
            return $query->result([], 0); // before the migration (the admin nav badge / page still load)
        }
        $where = $conditions ? 'WHERE ' . implode(' AND ', $conditions) : '';
        $from = 'FROM ownership_claims c LEFT JOIN users u ON u.id = c.claimant_id LEFT JOIN users o ON o.id = c.owner_id LEFT JOIN qr q ON q.code = c.code';
        $total = (int) $this->db->value("SELECT COUNT(*) $from $where", $params);
        $rows = $this->db->all("SELECT c.*, u.email AS claimant_email, o.email AS owner_email, q.edition_id, q.edition_number, q.claimed_by AS current_owner_id,
                (SELECT COUNT(*) FROM ownership_claim_files f WHERE f.claim_id = c.id) AS files
            $from $where {$query->orderBy('c.' . $query->sortField, 'c.id')} LIMIT {$query->pageSize} OFFSET {$query->offset()}", $params);
        $items = array_map(fn(array $r) => [
            'id' => $r['id'],
            'code' => $r['code'],
            'status' => $r['status'],
            'claimant' => ['id' => $r['claimant_id'], 'email' => $r['claimant_email']],
            'owner' => $r['owner_id'] !== null ? ['id' => $r['owner_id'], 'email' => $r['owner_email']] : null,
            'owner_changed' => $r['current_owner_id'] !== $r['owner_id'],
            'edition_id' => $r['edition_id'],
            'edition_number' => $r['edition_number'] !== null ? (int) $r['edition_number'] : null,
            'files' => (int) $r['files'],
            'statement' => mb_strlen((string) $r['statement']) > 140 ? mb_substr((string) $r['statement'], 0, 140) . '…' : $r['statement'],
            'created_at' => Time::iso($r['created_at']),
            'decided_at' => Time::iso($r['decided_at']),
        ], $rows);
        return $query->result($items, $total);
    }

    /**
     * GET /admin/claims/{id} → { claim, code (CodeAdmin::detail), current_owner, certificate (CertificateVerifier, or
     * { error }), history (transfers of the code), other_claims (the code's other claims) }
     * @return array<string, mixed>
     */
    public function detail(string $claimId): array
    {
        $row = $this->row($claimId);
        $code = (string) $row['code'];
        $qr = $this->codes->find($code);
        try {
            $certificate = $this->certificates->byCode($code);
            unset($certificate['status']);
        } catch (Throwable $error) {
            $certificate = ['valid' => false, 'reason' => $error instanceof ApiError ? $error->errorCode : 'error', 'error' => $error->getMessage()];
        }
        $others = $this->db->all('SELECT * FROM ownership_claims WHERE code = ? AND id <> ? ORDER BY created_at DESC LIMIT 50', [$code, $claimId]);
        return [
            'claim' => $this->adminJson($row),
            'code' => $qr ? $this->codeAdmin->detail($code) : null,
            'current_owner' => $this->transfers->accountJson($qr['claimed_by'] ?? null),
            'legacy_registration' => $qr !== null && $qr['claimed_by'] === null && (string) $qr['email'] !== '',
            'certificate' => $certificate,
            'history' => $this->transfers->history($code),
            'other_claims' => array_map(fn(array $r) => $this->adminJson($r), $others),
        ];
    }

    /**
     * POST /admin/claims/{id}/transfer { note } – moves the code to the claimant; mails both owners.
     * 400 note_required · 404 · 409 claim_closed / already_owner.
     * @param array<string, mixed> $admin
     * @param array<string, mixed> $body
     * @return array<string, mixed> detail()
     */
    public function transfer(array $admin, string $claimId, array $body): array
    {
        $note = OwnershipTransfers::note($body['note'] ?? null) ?? throw new ApiError(400, 'note_required', 'Say why (a note is required).');
        $itemId = is_string($body['item_id'] ?? null) && $body['item_id'] !== '' ? $body['item_id'] : null;
        $variant = is_string($body['variant'] ?? null) && $body['variant'] !== '' ? $body['variant'] : null;
        $moved = $this->db->transaction(function () use ($admin, $claimId, $note, $itemId, $variant) {
            $claim = $this->db->one('SELECT * FROM ownership_claims WHERE id = ? FOR UPDATE', [$claimId]) ?? throw new ApiError(404, 'not_found', 'Claim not found.');
            if ($claim['status'] !== 'open') throw new ApiError(409, 'claim_closed', 'This claim is already decided.');
            $claimant = $this->users->byId((string) $claim['claimant_id']) ?? throw new ApiError(404, 'unknown_user', 'The claimant\'s account is gone.');
            $moved = $this->transfers->moveCode((string) $claim['code'], $claimant, 'claim', (string) $admin['id'], $note, $claimId, $itemId, $variant);
            $this->decide($claimId, 'transferred', (string) $admin['id'], $note);
            return $moved;
        });
        $this->transfers->notifyMoved($moved);
        return $this->detail($claimId);
    }

    /**
     * POST /admin/claims/{id}/reject { note } – the code stays; the claimant gets a mail (the note stays internal).
     * 400 note_required · 404 · 409 claim_closed.
     * @param array<string, mixed> $admin
     * @param array<string, mixed> $body
     * @return array<string, mixed> detail()
     */
    public function reject(array $admin, string $claimId, array $body): array
    {
        $note = OwnershipTransfers::note($body['note'] ?? null) ?? throw new ApiError(400, 'note_required', 'Say why (a note is required).');
        $claim = $this->row($claimId);
        if ($claim['status'] !== 'open') throw new ApiError(409, 'claim_closed', 'This claim is already decided.');
        $this->decide($claimId, 'rejected', (string) $admin['id'], $note);
        $claimant = $this->users->byId((string) $claim['claimant_id']);
        if ($claimant) {
            try {
                $this->mailer->send(MailTemplates::OWNERSHIP_CLAIM_REJECTED, (string) $claimant['email'], MailTemplates::locale($claimant['locale'] ?? null),
                    ['code' => (string) $claim['code'], 'url' => rtrim($this->settings->get('APP_URL'), '/') . '/account']);
            } catch (MailFailed) {
                // logged
            }
        }
        return $this->detail($claimId);
    }

    /**
     * An evidence file for the admin: [bytes, content type, file name]. 404 not_found.
     * @return array{0: string, 1: string, 2: string}
     */
    public function adminFile(string $claimId, string $fileId): array
    {
        $file = $this->db->one('SELECT * FROM ownership_claim_files WHERE id = ? AND claim_id = ?', [$fileId, $claimId]) ?? throw new ApiError(404, 'not_found', 'File not found.');
        $bytes = $this->files->readClaim((string) $file['filename']) ?? throw new ApiError(404, 'not_found', 'The file is missing in the storage.');
        return [$bytes, (string) $file['content_type'], (string) $file['name']];
    }

    /**
     * @param array<string, mixed> $row
     * @return array<string, mixed>
     */
    private function adminJson(array $row): array
    {
        return [
            'id' => $row['id'],
            'code' => $row['code'],
            'status' => $row['status'],
            'statement' => $row['statement'],
            'claimant' => $this->transfers->accountJson((string) $row['claimant_id']),
            'owner' => $this->transfers->accountJson($row['owner_id'] !== null ? (string) $row['owner_id'] : null),
            'admin' => $this->transfers->accountJson($row['admin_id'] !== null ? (string) $row['admin_id'] : null),
            'admin_note' => $row['admin_note'],
            'files' => array_map(fn(array $f) => ['id' => $f['id'], 'name' => $f['name'], 'type' => $f['content_type'], 'size' => (int) $f['size'],
                'created_at' => Time::iso($f['created_at'])], $this->fileRows((string) $row['id'])),
            'created_at' => Time::iso($row['created_at']),
            'decided_at' => Time::iso($row['decided_at']),
        ];
    }

    private function decide(string $claimId, string $status, string $adminId, string $note): void
    {
        $this->db->run("UPDATE ownership_claims SET status = ?, admin_id = ?, admin_note = ?, decided_at = ? WHERE id = ? AND status = 'open'",
            [$status, $adminId, $note, $this->clock->utc(), $claimId]);
    }

    /** @return array<string, mixed> */
    private function row(string $claimId): array
    {
        return $this->db->one('SELECT * FROM ownership_claims WHERE id = ?', [$claimId]) ?? throw new ApiError(404, 'not_found', 'Claim not found.');
    }

    /** @return list<array<string, mixed>> */
    private function fileRows(string $claimId): array
    {
        return $this->db->all('SELECT * FROM ownership_claim_files WHERE claim_id = ? ORDER BY created_at, id', [$claimId]);
    }
}

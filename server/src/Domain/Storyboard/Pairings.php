<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Live\LiveSessions;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Throwable;

/**
 * The handheld camera's pairings (#126, #469; agents/research/HANDHELD-CAMERA.md, STUDIO-IN-CREATOR.md §2.6): how the
 * phone gets the right to join ONE relay session – the live road (QUIC first, the WebSocket last) between the member's
 * PC (the creator's Handheld panel, the session's conductor) and their phone (the creator's barebone page /h/<code>, no
 * sign-in). The PC asks for a one-time code (signed-in content editor) and gets with it a fresh relay session and its own
 * conductor token; the phone claims the code once and gets the session and a `handheld` token for it (the relay checks
 * both with the secret it shares with this API, deck/relay/src/token.rs). The code is good for one claim within MINUTES;
 * the tokens last TOKEN_HOURS (a session's length: every rejoin, every fall to the WebSocket says hello again). The code
 * (128 bits) is the phone's only right: it opens this one claim, nothing else of the API.
 */
final class Pairings
{
    public const MINUTES = 10;
    /** a handheld session's tokens last this long */
    public const TOKEN_HOURS = 12;
    /** open pairings a member may hold at once (a QR shown on a few PCs) */
    public const OPEN_MAX = 10;
    public const CODE = '#^[A-Za-z0-9_-]{22}$#';
    /** the relay session: `H` + 11 characters of LiveSessions' alphabet (no one types it: it is in the tokens) */
    public const SESSION = '#^H[A-Z2-9]{11}$#';
    private const SHOT = '#^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$#';
    private const ALPHABET = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';

    public function __construct(private readonly Database $db, private readonly Clock $clock, private readonly LiveSessions $live)
    {
    }

    /** Before migration 2026-11-10-storyboard-pairings-relay the endpoints answer 503 */
    private function requireAvailable(): void
    {
        try {
            $this->db->value('SELECT COUNT(session) FROM storyboard_pairings WHERE 1 = 0');
        } catch (Throwable) {
            throw new ApiError(503, 'not_available', 'Handheld pairing is not set up yet (migration 2026-11-10-storyboard-pairings-relay).');
        }
    }

    /**
     * A new one-time code for the member's PC and its relay session → { code, toon, chapter, shot, expires_at, session,
     * token (the PC's: conductor), dev (the relay's dev mode) }
     * @param array<string, mixed> $user
     * @param array<mixed> $body { toon, chapter, shot? }
     * @return array<string, mixed>
     */
    public function create(array $user, array $body): array
    {
        $this->requireAvailable();
        [$toon, $chapter] = WorkStorage::check($body['toon'] ?? null, $body['chapter'] ?? null);
        $shot = $body['shot'] ?? null;
        if ($shot !== null && (!is_string($shot) || !preg_match(self::SHOT, $shot))) throw new ApiError(422, 'invalid_pairing', 'shot is the shot id.', ['field' => 'shot']);
        $this->sweep();
        $open = (int) $this->db->value('SELECT COUNT(*) FROM storyboard_pairings WHERE created_by = ? AND claimed_at IS NULL', [(string) $user['id']]);
        if ($open >= self::OPEN_MAX) throw new ApiError(429, 'rate_limited', 'Too many open pairings – wait a few minutes.', ['retry_after' => self::MINUTES * 60]);
        $code = rtrim(strtr(base64_encode(random_bytes(16)), '+/', '-_'), '=');
        $session = 'H';
        for ($i = 0; $i < 11; $i++) $session .= self::ALPHABET[random_int(0, strlen(self::ALPHABET) - 1)];
        $expires = $this->clock->utc(self::MINUTES * 60);
        $this->db->run('INSERT INTO storyboard_pairings (code, toon, chapter, shot, session, created_by, expires_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)',
            [$code, $toon, $chapter, $shot, $session, (string) $user['id'], $expires, $this->clock->utc()]);
        return ['code' => $code, 'toon' => $toon, 'chapter' => $chapter, 'shot' => $shot, 'expires_at' => $expires, 'session' => $session,
            'token' => $this->live->token($session, 'conductor', self::TOKEN_HOURS * 3600), 'dev' => $this->live->dev()];
    }

    /**
     * The phone's claim, once (no session – the code is the right): { shot, session, token (handheld) }. A second claim is
     * 409 claimed; a wrong or expired code a plain 404.
     * @return array<string, mixed>
     */
    public function claim(string $code): array
    {
        $row = $this->open($code);
        $claimed = $row['claimed_at'] === null
            && $this->db->run('UPDATE storyboard_pairings SET claimed_at = ? WHERE code = ? AND claimed_at IS NULL', [$this->clock->utc(), $code])->rowCount() === 1;
        // two phones at once: only the one whose update took the row
        if (!$claimed) throw new ApiError(409, 'claimed', 'This code is used already – show a new QR code on the PC.');
        return ['shot' => $row['shot'], 'session' => $row['session'], 'token' => $this->live->token((string) $row['session'], 'handheld', self::TOKEN_HOURS * 3600)];
    }

    /**
     * The PC gives up (Disconnect, a new code): its own pairing goes
     * @param array<string, mixed> $user
     */
    public function delete(string $code, array $user): void
    {
        $this->requireAvailable();
        if (!preg_match(self::CODE, $code)) throw new ApiError(404, 'not_found', 'No such pairing.');
        $this->db->run('DELETE FROM storyboard_pairings WHERE code = ? AND created_by = ?', [$code, (string) $user['id']]);
    }

    /** @return array<string, mixed> the row while it is open – a wrong or expired code is a plain 404 */
    private function open(string $code): array
    {
        $this->requireAvailable();
        if (!preg_match(self::CODE, $code)) throw new ApiError(404, 'not_found', 'No such pairing.');
        $row = $this->db->one('SELECT code, shot, session, claimed_at, expires_at FROM storyboard_pairings WHERE code = ?', [$code]);
        if ($row === null) throw new ApiError(404, 'not_found', 'No such pairing.');
        if ((string) $row['expires_at'] <= $this->clock->utc()) {
            $this->db->run('DELETE FROM storyboard_pairings WHERE code = ?', [$code]);
            throw new ApiError(404, 'not_found', 'No such pairing.');
        }
        return $row;
    }

    /** the expired rows go (a claimed code too: its tokens carry the session from then on) */
    private function sweep(): void
    {
        $this->db->run('DELETE FROM storyboard_pairings WHERE expires_at <= ?', [$this->clock->utc()]);
    }
}

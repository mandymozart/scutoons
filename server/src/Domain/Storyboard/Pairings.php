<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Database\Database;
use Throwable;

/**
 * The handheld camera's pairings (#126, agents/research/STUDIO-IN-CREATOR.md §2.6): the signaling of ONE WebRTC data
 * channel between the member's PC (the creator's Handheld panel) and their phone (the creator's barebone page /h/<code>,
 * no sign-in). The PC asks for a one-time code (signed-in content editor), puts its offer, the phone reads the offer
 * with the code and puts its answer, the PC reads the answer – and with that read the row is gone. A row lives
 * MINUTES at most; every new pairing sweeps the expired ones. The code (128 bits) is the phone's only right: this row's
 * offer and answer, each written once – nothing else of the API is readable with it.
 */
final class Pairings
{
    public const MINUTES = 10;
    /** open pairings a member may hold at once (a QR shown on a few PCs) */
    public const OPEN_MAX = 10;
    public const CODE = '#^[A-Za-z0-9_-]{22}$#';
    private const SHOT = '#^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$#';
    /** an SDP with its gathered candidates is a few KB; this is plenty */
    private const SDP_MAX = 32768;

    public function __construct(private readonly Database $db, private readonly Clock $clock)
    {
    }

    /** Before migration 2026-10-21-storyboard-pairings the endpoints answer 503 */
    private function requireAvailable(): void
    {
        try {
            $this->db->value('SELECT COUNT(*) FROM storyboard_pairings WHERE 1 = 0');
        } catch (Throwable) {
            throw new ApiError(503, 'not_available', 'Handheld pairing is not set up yet (migration 2026-10-21-storyboard-pairings).');
        }
    }

    /**
     * A new one-time code for the member's PC → { code, toon, chapter, shot, expires_at }
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
        $open = (int) $this->db->value('SELECT COUNT(*) FROM storyboard_pairings WHERE created_by = ?', [(string) $user['id']]);
        if ($open >= self::OPEN_MAX) throw new ApiError(429, 'rate_limited', 'Too many open pairings – wait a few minutes.', ['retry_after' => self::MINUTES * 60]);
        $code = rtrim(strtr(base64_encode(random_bytes(16)), '+/', '-_'), '=');
        $expires = $this->clock->utc(self::MINUTES * 60);
        $this->db->run('INSERT INTO storyboard_pairings (code, toon, chapter, shot, created_by, expires_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)',
            [$code, $toon, $chapter, $shot, (string) $user['id'], $expires, $this->clock->utc()]);
        return ['code' => $code, 'toon' => $toon, 'chapter' => $chapter, 'shot' => $shot, 'expires_at' => $expires];
    }

    /** The PC's offer, once (409 offer_exists) */
    public function offer(string $code, mixed $body): void
    {
        $row = $this->open($code);
        if ($row['offer'] !== null) throw new ApiError(409, 'offer_exists', 'This pairing has its offer already.');
        $this->db->run('UPDATE storyboard_pairings SET offer = ? WHERE code = ? AND offer IS NULL', [self::sdp($body), $code]);
    }

    /** The phone's answer, once, after the offer (409 no_offer / answer_exists) */
    public function answer(string $code, mixed $body): void
    {
        $row = $this->open($code);
        if ($row['offer'] === null) throw new ApiError(409, 'no_offer', 'The PC has not put its offer yet.');
        if ($row['answer'] !== null) throw new ApiError(409, 'answer_exists', 'This pairing is answered already.');
        $this->db->run('UPDATE storyboard_pairings SET answer = ? WHERE code = ? AND answer IS NULL', [self::sdp($body), $code]);
    }

    /**
     * The pairing as the code shows it: { code, toon, chapter, shot, offer, answered, expires_at } – the phone. The member
     * who asked for it (signed in) also gets the `answer`; reading it ends the pairing (the row is deleted).
     * @param array<string, mixed>|null $user
     * @return array<string, mixed>
     */
    public function read(string $code, ?array $user): array
    {
        $row = $this->open($code);
        $view = ['code' => $code, 'toon' => $row['toon'], 'chapter' => (int) $row['chapter'], 'shot' => $row['shot'], 'offer' => $row['offer'],
            'answered' => $row['answer'] !== null, 'expires_at' => $row['expires_at']];
        if ($user === null || (string) $user['id'] !== (string) $row['created_by']) return $view;
        if ($row['answer'] !== null) $this->db->run('DELETE FROM storyboard_pairings WHERE code = ?', [$code]);
        return $view + ['answer' => $row['answer']];
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

    /** @return array<string, mixed> the row while it is open – a wrong, used or expired code is a plain 404 */
    private function open(string $code): array
    {
        $this->requireAvailable();
        if (!preg_match(self::CODE, $code)) throw new ApiError(404, 'not_found', 'No such pairing.');
        $row = $this->db->one('SELECT code, toon, chapter, shot, created_by, offer, answer, expires_at FROM storyboard_pairings WHERE code = ?', [$code]);
        if ($row === null) throw new ApiError(404, 'not_found', 'No such pairing.');
        if ((string) $row['expires_at'] <= $this->clock->utc()) {
            $this->db->run('DELETE FROM storyboard_pairings WHERE code = ?', [$code]);
            throw new ApiError(404, 'not_found', 'No such pairing.');
        }
        return $row;
    }

    /** the expired rows go */
    private function sweep(): void
    {
        $this->db->run('DELETE FROM storyboard_pairings WHERE expires_at <= ?', [$this->clock->utc()]);
    }

    /** { sdp } – a session description (text, `v=0` …) */
    private static function sdp(mixed $body): string
    {
        $sdp = is_array($body) ? ($body['sdp'] ?? null) : null;
        if (!is_string($sdp) || !str_starts_with($sdp, 'v=0') || strlen($sdp) > self::SDP_MAX) {
            throw new ApiError(422, 'invalid_pairing', 'sdp is a WebRTC session description (v=0 …, at most 32 KB).', ['field' => 'sdp']);
        }
        return $sdp;
    }
}

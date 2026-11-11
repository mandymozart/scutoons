<?php
declare(strict_types=1);

namespace Scutoons\Domain\Live;

use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;

/**
 * A live session (LIVE.md B.6): a code, and tokens the relay checks with the secret it shares with this API
 * (`LIVE_TOKEN_SECRET`; deck/relay/src/token.rs is the other half: `base64url(payload JSON) "." base64url(HMAC-SHA256)`, tested
 * against one vector on both sides). Unset secret: the relay's dev mode, any token passes, so `dev` is one.
 */
final class LiveSessions
{
    public const HOURS = 24;
    /** #478: a proof lasts an hour (a deck token's life); a client asks again at half of it */
    public const PROOF_SECONDS = 3600;
    /** #478: the account roles a proof may state, in this order (developer is no fact a role asks; guest needs no proof) */
    public const FACTS = [Roles::MEMBER, Roles::CREATOR, Roles::ADMIN, Roles::LIVE];
    private const ALPHABET = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';

    public function __construct(private readonly Settings $settings, private readonly Clock $clock)
    {
    }

    /** @return array{code: string, token: string, tokens: array{bridge: string, seat: string, uplink: string}, dev: bool} */
    public function open(?string $show): array
    {
        $code = '';
        for ($i = 0; $i < 6; $i++) $code .= self::ALPHABET[random_int(0, strlen(self::ALPHABET) - 1)];
        $token = fn(string $role): string => $this->token($code, $role, self::HOURS * 3600, $show);
        return ['code' => $code, 'token' => $token('conductor'), 'tokens' => ['bridge' => $token('bridge'), 'seat' => $token('seat'), 'uplink' => $token('uplink')], 'dev' => $this->dev()];
    }

    /** The relay's token for `$role` in session `$code` for `$seconds` from now (#469: the handheld camera's two as well) */
    public function token(string $code, string $role, int $seconds, ?string $show = null): string
    {
        $secret = $this->settings->get('LIVE_TOKEN_SECRET');
        if ($secret === '') return 'dev';
        return self::sign($secret, ['session' => $code, 'role' => $role, 'exp' => $this->clock->time() + $seconds] + ($show === null ? [] : ['show' => $show]));
    }

    /**
     * #478: a client's proof for session `$code` – the facts a role's filter may ask that only this API knows: the account
     * roles among member, creator, admin, live (a reader's, or the member's a paired deck belongs to, #481), and whether it
     * is an approved deck. Nothing about who it is (no id, no name,
     * no mail). Signed like the tokens, `{ session, facts: { account, approved }, exp }` (deck/relay/src/token.rs `proof`;
     * the shared vector: deck/protocol/fixtures/live-proof.json). No secret: signed with the empty key – the relay in dev
     * mode takes the facts as stated, a relay with a secret refuses them (that client is a guest).
     * @param list<string> $account
     * @return array{proof: string, expires_in: int, dev: bool}
     */
    public function proof(string $code, array $account, bool $approved): array
    {
        $claims = ['session' => $code, 'facts' => ['account' => $account, 'approved' => $approved], 'exp' => $this->clock->time() + self::PROOF_SECONDS];
        return ['proof' => self::sign($this->settings->get('LIVE_TOKEN_SECRET'), $claims), 'expires_in' => self::PROOF_SECONDS, 'dev' => $this->dev()];
    }

    /** no secret: the relay's dev mode */
    public function dev(): bool
    {
        return $this->settings->get('LIVE_TOKEN_SECRET') === '';
    }

    /** @param array<string, mixed> $claims in the order the relay writes them */
    public static function sign(string $secret, array $claims): string
    {
        $b64 = fn(string $bytes): string => rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
        $payload = $b64((string) json_encode($claims, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE));
        return $payload . '.' . $b64(hash_hmac('sha256', $payload, $secret, true));
    }
}

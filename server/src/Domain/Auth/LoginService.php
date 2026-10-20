<?php
declare(strict_types=1);

namespace Scutoons\Domain\Auth;

use Scutoons\Domain\Access\Permissions;
use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Campaigns\Giveaways;
use Scutoons\Domain\Campaigns\Invites;
use Scutoons\Domain\Codes\CodeRules;
use Scutoons\Domain\Codes\CodeService;
use Scutoons\Domain\Events\EventLog;
use Scutoons\Domain\Mail\MailFailed;
use Scutoons\Domain\Mail\Mailer;
use Scutoons\Domain\Mail\MailTemplates;
use Scutoons\Domain\Newsletter\NewsletterSignin;
use Scutoons\Domain\RequestContext;
use Scutoons\Domain\Secrets;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Support\Ids;
use PDOException;

/**
 * Email login: request → mail with a magic link and a 6-digit one-time code → verify (link token or email + code) →
 * session token (Bearer). The first confirmation creates the user (with the `member` role and the starter items).
 * A claimable code sent with the request is claimed in the same step. Tokens and codes are stored as hashes only.
 *
 * Signup rule (ITEMS.md "Roles, signup rule, promo codes"): a NEW account needs a signup code – a code whose type has
 * `"signup": true` (membership = a product, promo = a PR code). An address without an account and without such a code
 * still gets `{ status: "sent" }` (no user enumeration), but the mail only explains how to get an account.
 *
 * "This is mine" (agents/ITEMS.md "Ownership, claims, gifting"): `dispute: true` with a membership code that is
 * registered to someone else lets a new account be created anyway – nothing is claimed, the reader then files an
 * ownership claim. Only the "already registered" refusal is lifted (expired / revoked / no-signup codes still refuse).
 */
final class LoginService
{
    public function __construct(
        private readonly Database $db,
        private readonly Clock $clock,
        private readonly Settings $settings,
        private readonly Secrets $secrets,
        private readonly RequestContext $context,
        private readonly Users $users,
        private readonly Sessions $sessions,
        private readonly CodeService $codes,
        private readonly Permissions $permissions,
        private readonly EventLog $events,
        private readonly Mailer $mailer,
        private readonly Invites $invites,
        private readonly Giveaways $giveaways,
    ) {
    }

    /**
     * POST /auth/request { email, code?, item_id?, variant?, locale?, marketing_opt_in?, dispute? } → { status: "sent", expires_at }
     * (same answer for new and known addresses, with or without a usable signup code)
     * @param array<string, mixed> $body
     * @return array<string, mixed>
     */
    public function request(array $body): array
    {
        $email = self::email($body['email'] ?? null);
        $locale = MailTemplates::locale($body['locale'] ?? null);
        $claimCode = CodeRules::normalize($body['code'] ?? null);
        $itemId = is_string($body['item_id'] ?? null) ? substr($body['item_id'], 0, 64) : null;
        $variant = is_string($body['variant'] ?? null) ? substr($body['variant'], 0, 64) : null;
        $dispute = $claimCode !== null && Users::flag($body['dispute'] ?? false);
        $ipHash = $this->secrets->hmac($this->context->clientIp());

        // Housekeeping: requests are only needed while they can be confirmed and for the hourly limits
        $this->db->run('DELETE FROM login_requests WHERE created_at < ?', [$this->clock->utc(-86400)]);
        $hourAgo = $this->clock->utc(-3600);
        // the newsletter's sign-in links (NewsletterSignin) don't count
        if ((int) $this->db->value('SELECT COUNT(*) FROM login_requests WHERE email = ? AND created_at > ? AND ip_hash <> ?', [$email, $hourAgo, NewsletterSignin::IP_HASH]) >= $this->settings->int('LOGIN_MAX_PER_EMAIL_HOUR')
            || (int) $this->db->value('SELECT COUNT(*) FROM login_requests WHERE ip_hash = ? AND created_at > ?', [$ipHash, $hourAgo]) >= $this->settings->int('LOGIN_MAX_PER_IP_HOUR')) {
            throw new ApiError(429, 'rate_limited', 'Too many emails requested – try again later.');
        }

        $optIn = Users::flag($body['marketing_opt_in'] ?? false);
        $id = Ids::uuid7($this->clock->ms()); // time-ordered: verify by email takes the newest request, also within one second
        $token = Ids::token();
        $otp = sprintf('%06d', random_int(0, 999999));
        $ttl = $this->settings->int('LOGIN_TTL_MINUTES') * 60;
        $hasAccount = $this->users->byEmail($email) !== null;
        // no code, but the old system registered a code to this address: that code signs it up (and is claimed with it)
        if (!$hasAccount && $claimCode === null) $claimCode = $this->codes->legacySignupCode($email);
        $refusal =$hasAccount ? null : $this->codes->signupRefusal($claimCode, $email);
        $canSignIn = $hasAccount || $refusal === null || ($dispute && $refusal === 'already_claimed');
        // Without an account and without a signup code the request is only recorded for the hourly limits: its
        // hashes belong to values nobody knows (no link or code is mailed), so it can never be confirmed.
        $values = [$id, $email, hash('sha256', $canSignIn ? $token : Ids::token()), $canSignIn ? $this->otpHash($id, $otp) : str_repeat('0', 64),
            $locale, $claimCode, $itemId, $variant, $optIn ? 1 : 0, $ipHash, $this->clock->utc(), $this->clock->utc($ttl)];
        if ($dispute) {
            // the column of 2026-10-02-ownership-signup – only written for a dispute, so every other sign-in works before it ran
            try {
                $this->db->run(
                    'INSERT INTO login_requests (id, email, token_hash, code_hash, locale, claim_code, claim_item_id, claim_variant, marketing_opt_in, ip_hash, attempts, created_at, expires_at, dispute)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?, 1)', $values);
            } catch (PDOException $error) {
                error_log('[qr-api] dispute login request: ' . $error->getMessage());
                throw new ApiError(503, 'not_available', 'Ownership claims are not available yet – try again later.');
            }
        } else {
            $this->db->run(
                'INSERT INTO login_requests (id, email, token_hash, code_hash, locale, claim_code, claim_item_id, claim_variant, marketing_opt_in, ip_hash, attempts, created_at, expires_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)', $values);
        }

        if ($canSignIn) {
            // Only claimable codes (products) go into the inventory; a promo code is just the signup code
            $row = $claimCode !== null ? $this->codes->find($claimCode) : null;
            $claimable = $row !== null && $this->permissions->claimable((string) $row['type']);
            $template = MailTemplates::LOGIN_CODE;
            $vars = ['link' => $this->link($token, $claimCode), 'otp' => $otp, 'minutes' => $this->settings->int('LOGIN_TTL_MINUTES'),
                'code' => $claimable && !$dispute ? $claimCode : null, 'signup' => !$hasAccount];
        } else {
            $template = MailTemplates::SIGNUP_WITHOUT_CODE;
            $vars = ['url' => $this->viewerUrl(), 'code' => $claimCode];
        }
        try {
            $this->mailer->send($template, $email, $locale, $vars);
        } catch (MailFailed) {
            $this->db->run('DELETE FROM login_requests WHERE id = ?', [$id]);
            throw new ApiError(502, 'mail_failed', 'The email could not be sent.');
        }

        return ['status' => 'sent', 'expires_at' => gmdate('c', $this->clock->time() + $ttl)];
    }

    /** The viewer the request came from (allowed Origin), else APP_URL – for the link in the no-account mail */
    private function viewerUrl(): string
    {
        $origin = $this->allowedOrigin();
        return $origin ?? rtrim($this->settings->get('APP_URL'), '/');
    }

    /**
     * Where the magic link points: the calling viewer (allowed Origin) → `<origin>/?login=<token>`;
     * otherwise (the QR landing page) → `QR_URL/?login=<token>[&c=<code>]`.
     */
    private function link(string $token, ?string $claimCode): string
    {
        $origin = $this->allowedOrigin();
        if ($origin !== null) return $origin . '/?login=' . rawurlencode($token);
        return rtrim($this->settings->get('QR_URL'), '/') . '/?login=' . rawurlencode($token) . ($claimCode !== null ? '&c=' . rawurlencode($claimCode) : '');
    }

    private function allowedOrigin(): ?string
    {
        $origin = $this->context->origin();
        if ($origin === null) return null;
        foreach ($this->settings->allowedOrigins() as $allowed) {
            if (strcasecmp(rtrim($allowed, '/'), $origin) === 0) return $origin;
        }
        return null;
    }

    /**
     * POST /auth/verify { email, otp } | { token } (+ optional item_id, variant, marketing_opt_in)
     * → { status: "success", session_token, user, created, claimed?: [code], claim_error? }
     * @param array<string, mixed> $body
     * @return array<string, mixed>
     */
    public function verify(array $body): array
    {
        $token = $body['token'] ?? null;
        $byLink = is_string($token) && $token !== '';
        if ($byLink) {
            $request = $this->db->one('SELECT * FROM login_requests WHERE token_hash = ?', [hash('sha256', $token)]);
            if (!$request) throw new ApiError(400, 'invalid_token', 'This link is not valid.');
        } else {
            $email = self::email($body['email'] ?? null);
            $otp = is_string($body['otp'] ?? null) || is_int($body['otp'] ?? null) ? (string) preg_replace('/\D/', '', (string) $body['otp']) : '';
            if ($otp === '') throw new ApiError(400, 'invalid_otp', 'Code missing.');
            // The newest request of this address is the one whose code was just mailed (a newsletter's link has no code)
            $request = $this->db->one('SELECT * FROM login_requests WHERE email = ? AND ip_hash <> ? ORDER BY created_at DESC, id DESC LIMIT 1', [$email, NewsletterSignin::IP_HASH]);
            if (!$request) throw new ApiError(400, 'invalid_otp', 'This code is not valid.');
        }
        if ($request['used_at'] !== null) throw new ApiError(410, 'already_used', 'This link or code was used already.');
        if ($request['expires_at'] < $this->clock->utc()) throw new ApiError(410, 'expired', 'This link or code has expired.');
        if (!$byLink) {
            if ((int) $request['attempts'] >= $this->settings->int('LOGIN_MAX_ATTEMPTS')) {
                throw new ApiError(429, 'too_many_attempts', 'Too many wrong codes – request a new email.');
            }
            if (!hash_equals((string) $request['code_hash'], $this->otpHash((string) $request['id'], $otp ?? ''))) {
                $this->db->run('UPDATE login_requests SET attempts = attempts + 1 WHERE id = ?', [$request['id']]);
                throw new ApiError(400, 'invalid_otp', 'This code is not valid.');
            }
        }
        return $this->confirm($request, $body);
    }

    /**
     * Confirms a checked login request (OTP or link already verified): signs in, or creates the account.
     * A NEW account with a claimable code (a product) is created and claims that code in ONE transaction – if the
     * claim fails (409 already_claimed, 422 choose_item / invalid_item, …) nothing is created and the request stays
     * unused, so the client can confirm again (e.g. with item_id/variant). Existing accounts claim afterwards, and a
     * failed claim is only reported as claim_error. Public for concurrent-signup tests.
     * @param array<string, mixed> $request the login_requests row
     * @param array<string, mixed> $body    item_id?, variant?, marketing_opt_in?
     * @return array<string, mixed> the /auth/verify/ response
     */
    public function confirm(array $request, array $body): array
    {
        $optIn = array_key_exists('marketing_opt_in', $body) ? Users::flag($body['marketing_opt_in']) : (bool) (int) ($request['marketing_opt_in'] ?? 0);
        $code = $request['claim_code'] !== null ? (string) $request['claim_code'] : null;
        $itemId = is_string($body['item_id'] ?? null) ? $body['item_id'] : ($request['claim_item_id'] ?? null);
        $variant = array_key_exists('variant', $body) ? (is_string($body['variant']) ? $body['variant'] : null) : ($request['claim_variant'] ?? null);
        $claimRow = $code !== null ? $this->codes->find($code) : null;
        // Only claimable codes (products) are claimed; promo codes are just signup codes
        $claimable = $code !== null && ($claimRow === null || $this->permissions->claimable((string) $claimRow['type']));
        // "This is mine": the account is for filing an ownership claim – the code is not claimed (it is someone else's)
        $dispute = (int) ($request['dispute'] ?? 0) === 1;
        if ($dispute) $claimable = false;

        try {
            [$user, $created, $session, $claimedNew] = $this->db->transaction(function () use ($request, $optIn, $code, $itemId, $variant, $claimable, $dispute) {
                // Only one confirmation per request, also when link and code race
                $used = $this->db->run('UPDATE login_requests SET used_at = ? WHERE id = ? AND used_at IS NULL', [$this->clock->utc(), $request['id']]);
                if ($used->rowCount() !== 1) throw new ApiError(410, 'already_used', 'This link or code was used already.');
                $user = $this->users->byEmail((string) $request['email']);
                if ($user) return [$user, false, $this->sessions->create((string) $user['id']), null];

                // Signup rule: only with a code of a type with "signup": true (checked again – it may have changed)
                $refusal = $this->codes->signupRefusal($code, (string) $request['email']);
                if ($refusal === 'already_claimed' && $dispute) $refusal = null;
                if ($refusal === 'already_claimed') {
                    throw new ApiError(409, 'already_claimed', 'This code is already in someone else\'s account, so it cannot create a new one. You can still read with it.');
                }
                if ($refusal !== null) {
                    throw new ApiError(403, 'signup_code_required', 'An account needs a code from a SCUTOONS product or a promo code.');
                }
                $user = $this->users->create((string) $request['email'], (string) $request['locale'], $code, $optIn);
                // Account + claim of its product code are atomic: a failed claim rolls the new account back
                $claimedNew = $claimable ? $this->codes->claimInTransaction($user, (string) $code, $itemId, $variant) : null;
                // gift invites waiting for this address arrive with the account (agents/research/GIFT-INVITES.md)
                $this->invites->deliverTo($user, $code);
                // a giveaway's promo code gets its campaign's reading rule (agents/research/CAMPAIGN-GIFTS.md §3)
                $this->giveaways->joined($user, $code);
                return [$user, true, $this->sessions->create((string) $user['id']), $claimedNew];
            });
        } catch (ApiError $error) {
            if ($code !== null && in_array($error->errorCode, ['already_claimed', 'email_required', 'revoked', 'expired'], true)) {
                $this->events->log('claim_refused', $code);
            }
            // a newsletter's sign-up link whose QR code offers a choice: the viewer opens the code's claim sheet
            if ($code !== null && $error->errorCode === 'choose_item' && $request['ip_hash'] === NewsletterSignin::IP_HASH) {
                throw new ApiError($error->status, $error->errorCode, $error->getMessage(), $error->extra + ['code' => $code]);
            }
            throw $error;
        }

        $response = ['status' => 'success', 'session_token' => $session, 'user' => null, 'created' => $created] + ($dispute ? ['dispute' => true] : []);
        if ($created) {
            $this->events->log('signup', (string) $code, (string) $user['id']);
            $this->mailer->sendToAdmins(MailTemplates::SIGNUP_ADMIN, [
                'email' => (string) $user['email'],
                'code' => (string) $code,
                'type' => $claimRow !== null ? (string) $claimRow['type'] : '–',
                'dispute' => $dispute,
                'url' => rtrim($this->settings->get('ADMIN_URL'), '/') . '/users/' . rawurlencode((string) $user['id']),
            ]);
            if ($claimable) {
                if ($claimedNew) $this->events->log('claim', (string) $code, (string) $user['id']);
                $response['claimed'] = [$code];
            }
        } else {
            if ($optIn && !(int) $user['marketing_opt_in']) {
                // An existing account that ticked "Send me news" while signing in opts in (never opts out here)
                $this->users->setMarketingOptIn((string) $user['id'], true);
                $user = $this->users->byId((string) $user['id']) ?? $user;
            }
            // Existing account: the code sent with the login request is claimed in the same step, failures are reported
            if ($claimable) {
                try {
                    $this->codes->claim($user, (string) $code, $itemId, $variant);
                    $response['claimed'] = [$code];
                } catch (ApiError $error) {
                    $response['claimed'] = [];
                    $response['claim_error'] = ['code' => $code, 'error' => $error->errorCode, 'message' => $error->getMessage()] + $error->extra;
                }
            }
        }
        $response['user'] = $this->users->toJson($user);

        // Statistics: one `login` event per code the account owns
        foreach ($this->db->all('SELECT code FROM qr WHERE claimed_by = ?', [$user['id']]) as $owned) {
            $this->events->log('login', (string) $owned['code'], (string) $user['id']);
        }
        return $response;
    }

    public static function email(mixed $value): string
    {
        return Users::normalizeEmail($value) ?? throw new ApiError(400, 'invalid_email', 'This email address is not valid.');
    }

    private function otpHash(string $requestId, string $otp): string
    {
        return $this->secrets->hmac("$requestId:$otp");
    }
}

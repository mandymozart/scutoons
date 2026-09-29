<?php
declare(strict_types=1);

/**
 * Email login (port of osct server/api/src/Auth.php): request → mail with a magic link and a 6-digit one-time
 * code → verify (link token or email + code) → session token (Bearer). The first confirmation creates the user
 * (with the `member` role and the starter items). A claimable code sent with the request is claimed in the same step.
 * Tokens and codes are stored as hashes only.
 *
 * Signup rule (ITEMS.md "Roles, signup rule, promo codes"): a NEW account needs a signup code – a code whose type has
 * `"signup": true` in api/permissions.json (membership = a product, promo = a PR code). An address without an account
 * and without such a code still gets `{ status: "sent" }` (no user enumeration), but the mail only explains how to
 * get an account (no link, no one-time code). Existing accounts sign in with or without a code.
 */
final class Auth
{
    /**
     * POST /auth/request { email, code?, item_id?, variant?, locale?, marketing_opt_in? } → { status: "sent" }
     * (same answer for new and known addresses, with or without a usable signup code)
     */
    public static function request(array $body): never
    {
        $email = self::email($body['email'] ?? null);
        $locale = LoginMail::locale($body['locale'] ?? null);
        $claimCode = Codes::normalize($body['code'] ?? null);
        $itemId = is_string($body['item_id'] ?? null) ? substr($body['item_id'], 0, 64) : null;
        $variant = is_string($body['variant'] ?? null) ? substr($body['variant'], 0, 64) : null;
        $secret = self::secret();
        $ipHash = hash_hmac('sha256', Http::clientIp(), $secret);

        // Housekeeping: requests are only needed while they can be confirmed and for the hourly limits
        Db::run('DELETE FROM login_requests WHERE created_at < ?', [Db::now(-86400)]);
        $hourAgo = Db::now(-3600);
        if ((int) Db::value('SELECT COUNT(*) FROM login_requests WHERE email = ? AND created_at > ?', [$email, $hourAgo]) >= Config::int('LOGIN_MAX_PER_EMAIL_HOUR')
            || (int) Db::value('SELECT COUNT(*) FROM login_requests WHERE ip_hash = ? AND created_at > ?', [$ipHash, $hourAgo]) >= Config::int('LOGIN_MAX_PER_IP_HOUR')) {
            throw new ApiError(429, 'rate_limited', 'Too many emails requested – try again later.');
        }

        $optIn = Users::flag($body['marketing_opt_in'] ?? false);
        $id = Db::uuid7(); // time-ordered: verify by email takes the newest request, also within one second
        $token = self::randomToken();
        $otp = sprintf('%06d', random_int(0, 999999));
        $ttl = Config::int('LOGIN_TTL_MINUTES') * 60;
        $hasAccount = Users::byEmail($email) !== null;
        $canSignIn = $hasAccount || Codes::signupRefusal($claimCode, $email) === null;
        // Without an account and without a signup code the request is only recorded for the hourly limits: its
        // hashes belong to values nobody knows (no link or code is mailed), so it can never be confirmed.
        Db::run(
            'INSERT INTO login_requests (id, email, token_hash, code_hash, locale, claim_code, claim_item_id, claim_variant, marketing_opt_in, ip_hash, attempts, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)',
            [$id, $email, hash('sha256', $canSignIn ? $token : self::randomToken()), $canSignIn ? self::otpHash($id, $otp) : str_repeat('0', 64),
                $locale, $claimCode, $itemId, $variant, $optIn ? 1 : 0, $ipHash, Db::now(), Db::now($ttl)]
        );

        if ($canSignIn) {
            // Only claimable codes (products) go into the inventory; a promo code is just the signup code
            $row = $claimCode !== null ? Codes::find($claimCode) : null;
            $claimable = $row !== null && Permissions::claimable((string) $row['type']);
            $mail = LoginMail::compose($locale, self::link($token, $claimCode), $otp, $claimable ? $claimCode : null, !$hasAccount);
        } else {
            $mail = LoginMail::composeNoAccount($locale, self::viewerUrl(), $claimCode);
        }
        try {
            Mailer::send($email, $mail['subject'], $mail['text'], $mail['html']);
        } catch (Throwable $error) {
            Db::run('DELETE FROM login_requests WHERE id = ?', [$id]);
            error_log('[qr-api] login mail failed: ' . $error->getMessage());
            throw new ApiError(502, 'mail_failed', 'The email could not be sent.');
        }

        Http::json(200, ['status' => 'sent', 'expires_at' => gmdate('c', time() + $ttl)]);
    }

    /** The viewer the request came from (allowed Origin), else APP_URL – for the link in the no-account mail */
    private static function viewerUrl(): string
    {
        $origin = Http::origin();
        return $origin !== null && Http::isAllowedOrigin($origin) ? $origin : rtrim(Config::get('APP_URL'), '/');
    }

    /**
     * Where the magic link points: the calling viewer (allowed Origin) → `<origin>/?login=<token>`;
     * otherwise (the QR landing page) → `QR_URL/?login=<token>[&c=<code>]`.
     */
    private static function link(string $token, ?string $claimCode): string
    {
        $origin = Http::origin();
        if ($origin !== null && Http::isAllowedOrigin($origin)) {
            return $origin . '/?login=' . rawurlencode($token);
        }
        return rtrim(Config::get('QR_URL'), '/') . '/?login=' . rawurlencode($token) . ($claimCode !== null ? '&c=' . rawurlencode($claimCode) : '');
    }

    /**
     * POST /auth/verify { email, otp } | { token } (+ optional item_id, variant for the code of the request,
     * marketing_opt_in – overrides the value sent with the request; stored for a new account, `true` also opts an
     * existing account in)
     * → { status: "success", session_token, user, created, claimed?: [code], claim_error? }
     * A new account needs the request's code to be a signup code (checked again here), else 403 signup_code_required;
     * a product code someone else owns → 409 already_claimed. A new account and the claim of its product code are one
     * transaction (claim fails → no account, e.g. 422 choose_item: confirm again with item_id/variant).
     * `claimed` / `claim_error` only for claimable codes (a promo code is never owned).
     */
    public static function verify(array $body): never
    {
        $token = $body['token'] ?? null;
        $byLink = is_string($token) && $token !== '';
        if ($byLink) {
            $request = Db::one('SELECT * FROM login_requests WHERE token_hash = ?', [hash('sha256', $token)]);
            if (!$request) throw new ApiError(400, 'invalid_token', 'This link is not valid.');
        } else {
            $email = self::email($body['email'] ?? null);
            $otp = is_string($body['otp'] ?? null) || is_int($body['otp'] ?? null) ? preg_replace('/\D/', '', (string) $body['otp']) : '';
            if ($otp === '') throw new ApiError(400, 'invalid_otp', 'Code missing.');
            // The newest request of this address is the one whose code was just mailed
            $request = Db::one('SELECT * FROM login_requests WHERE email = ? ORDER BY created_at DESC, id DESC LIMIT 1', [$email]);
            if (!$request) throw new ApiError(400, 'invalid_otp', 'This code is not valid.');
        }
        if ($request['used_at'] !== null) throw new ApiError(410, 'already_used', 'This link or code was used already.');
        if ($request['expires_at'] < Db::now()) throw new ApiError(410, 'expired', 'This link or code has expired.');
        if (!$byLink) {
            if ((int) $request['attempts'] >= Config::int('LOGIN_MAX_ATTEMPTS')) {
                throw new ApiError(429, 'too_many_attempts', 'Too many wrong codes – request a new email.');
            }
            if (!hash_equals($request['code_hash'], self::otpHash($request['id'], $otp))) {
                Db::run('UPDATE login_requests SET attempts = attempts + 1 WHERE id = ?', [$request['id']]);
                throw new ApiError(400, 'invalid_otp', 'This code is not valid.');
            }
        }

        Http::json(200, self::confirm($request, $body));
    }

    /**
     * Confirms a checked login request (OTP or link already verified): signs in, or creates the account.
     * A NEW account with a claimable code (a product) is created and claims that code in ONE transaction – if the
     * claim fails (409 already_claimed, 422 choose_item / invalid_item, …) nothing is created and the request stays
     * unused, so the client can confirm again (e.g. with item_id/variant). Existing accounts claim afterwards, and a
     * failed claim is only reported as claim_error. Public for tests/claim-race.php (signup mode).
     * @param array<string, mixed> $request the login_requests row
     * @param array<string, mixed> $body    item_id?, variant?, marketing_opt_in?
     * @return array<string, mixed> the /auth/verify/ response
     */
    public static function confirm(array $request, array $body): array
    {
        $optIn = array_key_exists('marketing_opt_in', $body) ? Users::flag($body['marketing_opt_in']) : (bool) (int) ($request['marketing_opt_in'] ?? 0);
        $code = $request['claim_code'];
        $itemId = is_string($body['item_id'] ?? null) ? $body['item_id'] : $request['claim_item_id'];
        $variant = array_key_exists('variant', $body) ? (is_string($body['variant']) ? $body['variant'] : null) : $request['claim_variant'];
        $claimRow = $code !== null ? Codes::find($code) : null;
        // Only claimable codes (products) are claimed; promo codes are just signup codes
        $claimable = $code !== null && ($claimRow === null || Permissions::claimable((string) $claimRow['type']));

        try {
            [$user, $created, $session, $claimedNew] = Db::transaction(function () use ($request, $optIn, $code, $itemId, $variant, $claimable) {
                // Only one confirmation per request, also when link and code race
                $used = Db::run('UPDATE login_requests SET used_at = ? WHERE id = ? AND used_at IS NULL', [Db::now(), $request['id']]);
                if ($used->rowCount() !== 1) throw new ApiError(410, 'already_used', 'This link or code was used already.');
                $user = Users::byEmail($request['email']);
                if ($user) return [$user, false, self::createSession($user['id']), null];

                // Signup rule: only with a code of a type with "signup": true (checked again – it may have changed)
                $refusal = Codes::signupRefusal($code, $request['email']);
                if ($refusal === 'already_claimed') {
                    throw new ApiError(409, 'already_claimed', 'This code is already in someone else\'s account, so it cannot create a new one. You can still read with it.');
                }
                if ($refusal !== null) {
                    throw new ApiError(403, 'signup_code_required', 'An account needs a code from a SCUTOONS product or a promo code.');
                }
                $user = Users::create($request['email'], $request['locale'], $code, $optIn);
                // Account + claim of its product code are atomic: a failed claim rolls the new account back
                $claimedNew = $claimable ? Codes::claimInTransaction($user, (string) $code, $itemId, $variant) : null;
                return [$user, true, self::createSession($user['id']), $claimedNew];
            });
        } catch (ApiError $error) {
            if ($code !== null && in_array($error->errorCode, ['already_claimed', 'email_required', 'revoked', 'expired'], true)) {
                Events::log('claim_refused', (string) $code);
            }
            throw $error;
        }

        $response = ['status' => 'success', 'session_token' => $session, 'user' => null, 'created' => $created];
        if ($created) {
            Events::log('signup', (string) $code, $user['id']);
            if ($claimable) {
                if ($claimedNew) Events::log('claim', (string) $code, $user['id']);
                $response['claimed'] = [$code];
            }
        } else {
            if ($optIn && !(int) $user['marketing_opt_in']) {
                // An existing account that ticked "Send me news" while signing in opts in (never opts out here)
                Users::setMarketingOptIn($user['id'], true);
                $user = Users::byId($user['id']);
            }
            // Existing account: the code sent with the login request is claimed in the same step, failures are reported
            if ($claimable) {
                try {
                    Codes::claim($user, (string) $code, $itemId, $variant);
                    $response['claimed'] = [$code];
                } catch (ApiError $error) {
                    $response['claimed'] = [];
                    $response['claim_error'] = ['code' => $code, 'error' => $error->errorCode, 'message' => $error->getMessage()] + $error->extra;
                }
            }
        }
        $response['user'] = Users::toJson($user);

        // Statistics: one `login` event per code the account owns
        foreach (Db::all('SELECT code FROM qr WHERE claimed_by = ?', [$user['id']]) as $owned) {
            Events::log('login', $owned['code'], $user['id']);
        }
        return $response;
    }

    /** POST /auth/logout → { status: "success" } (this device only) */
    public static function logout(): never
    {
        $token = Http::bearerToken();
        if ($token !== null) Db::run('DELETE FROM sessions WHERE token_hash = ?', [hash('sha256', $token)]);
        Http::json(200, ['status' => 'success']);
    }

    /** A new session; returns the token (only its hash is stored) */
    public static function createSession(string $userId): string
    {
        $session = self::randomToken();
        Db::run('INSERT INTO sessions (id, user_id, token_hash, created_at, last_used_at) VALUES (?, ?, ?, ?, ?)',
            [Db::uuid(), $userId, hash('sha256', $session), Db::now(), Db::now()]);
        return $session;
    }

    /**
     * The signed-in user (Bearer token), else 401. Sessions end after SESSION_TTL_DAYS without use.
     * @return array<string, mixed>
     */
    public static function user(): array
    {
        $user = self::optionalUser();
        if ($user === null) throw new ApiError(401, 'unauthorized', 'Not signed in.');
        return $user;
    }

    /** @return array<string, mixed>|null the signed-in user, or null (no/invalid token) */
    public static function optionalUser(): ?array
    {
        static $cache = [];
        $token = Http::bearerToken();
        if ($token === null) return null;
        if (array_key_exists($token, $cache)) return $cache[$token];
        $session = Db::one('SELECT * FROM sessions WHERE token_hash = ?', [hash('sha256', $token)]);
        if (!$session) return $cache[$token] = null;
        if ($session['last_used_at'] < Db::now(-86400 * Config::int('SESSION_TTL_DAYS'))) {
            Db::run('DELETE FROM sessions WHERE id = ?', [$session['id']]);
            return $cache[$token] = null;
        }
        if ($session['last_used_at'] < Db::now(-3600)) {
            Db::run('UPDATE sessions SET last_used_at = ? WHERE id = ?', [Db::now(), $session['id']]);
        }
        return $cache[$token] = Users::byId($session['user_id']);
    }

    public static function email(mixed $value): string
    {
        $email = is_string($value) ? strtolower(trim($value)) : '';
        if ($email === '' || strlen($email) > 254 || !filter_var($email, FILTER_VALIDATE_EMAIL)) {
            throw new ApiError(400, 'invalid_email', 'This email address is not valid.');
        }
        return $email;
    }

    private static function otpHash(string $requestId, string $otp): string
    {
        return hash_hmac('sha256', "$requestId:$otp", self::secret());
    }

    public static function secret(): string
    {
        $secret = Config::get('AUTH_SECRET');
        if (strlen($secret) < 32) throw new RuntimeException('AUTH_SECRET is missing or shorter than 32 characters');
        return $secret;
    }

    private static function randomToken(): string
    {
        return rtrim(strtr(base64_encode(random_bytes(32)), '+/', '-_'), '=');
    }
}

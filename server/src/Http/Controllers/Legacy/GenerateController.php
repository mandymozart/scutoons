<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Legacy;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\Batches;
use Scutoons\Domain\Admin\CodeAdmin;
use Scutoons\Domain\Admin\UserAdmin;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Codes\CodeRules;
use Scutoons\Http\Download;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\GenerateToken;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Signing\KeyStore;
use Scutoons\Infrastructure\Storage\FileStore;
use Throwable;

/**
 * LEGACY – the PHP endpoints of the generate/ admin SPA (static password token, X-Generate-Token), kept unchanged in
 * URL and response shape until the admin app (/api/admin/*, role admin) replaces generate/. Same domain services as
 * the admin API. Every endpoint except auth.php and get-signature.php sits behind the GenerateToken middleware.
 * Answers are the old ones: `{ success, … }` / `{ success: false, message|error }`, mostly with status 200.
 */
final class GenerateController
{
    private const FLAGS = 0;

    public function __construct(
        private readonly Settings $settings,
        private readonly CodeAdmin $codes,
        private readonly Batches $batches,
        private readonly UserAdmin $users,
        private readonly KeyStore $keys,
        private readonly FileStore $files,
    ) {
    }

    /** POST auth.php { action: login, password } → { token } · { action: validate, token } → { valid } */
    public function auth(Request $request, Response $response): Response
    {
        $body = json_decode(JsonBody::raw($request), true);
        $body = is_array($body) ? $body : [];
        $password = $this->settings->get('GENERATE_PASSWORD');
        $action = $body['action'] ?? '';
        if ($action === 'login') {
            $given = is_string($body['password'] ?? null) ? $body['password'] : '';
            if ($password !== '' && hash_equals($password, $given)) {
                return $this->json($response, 200, ['token' => GenerateToken::expected($password)]);
            }
            return $this->json($response, 401, ['error' => 'Invalid password']);
        }
        if ($action === 'validate') {
            $token = is_string($body['token'] ?? null) ? $body['token'] : '';
            return $this->json($response, 200, ['valid' => $password !== '' && hash_equals(GenerateToken::expected($password), $token)]);
        }
        return $this->json($response, 400, ['error' => 'Invalid action']);
    }

    /** POST generate.php (form: code, redirectUrl, type, email, expiresAt, edition_id) → the stored code */
    public function generate(Request $request, Response $response): Response
    {
        if ($request->getMethod() !== 'POST') return $this->json($response, 405, ['success' => false, 'message' => 'Method not allowed']);
        $form = (array) $request->getParsedBody();
        $redirectUrl = (string) ($form['redirectUrl'] ?? '');
        $code = (string) ($form['code'] ?? '');
        if ($redirectUrl === '' || $code === '') return $this->json($response, 200, ['success' => false, 'message' => 'Missing required fields']);
        if (!filter_var($redirectUrl, FILTER_VALIDATE_URL)) return $this->json($response, 200, ['success' => false, 'message' => 'Invalid URL']);
        $email = (string) ($form['email'] ?? '');
        if ($email !== '' && !filter_var($email, FILTER_VALIDATE_EMAIL)) return $this->json($response, 200, ['success' => false, 'message' => 'Invalid email']);
        try {
            $row = $this->codes->create([
                'code' => $code,
                'redirect_to' => $redirectUrl,
                'type' => (string) ($form['type'] ?? 'public'),
                'email' => $email,
                'expires_at' => (string) ($form['expiresAt'] ?? ''),
                'edition_id' => (string) ($form['edition_id'] ?? ''),
            ]);
        } catch (ApiError $error) {
            return $this->json($response, 200, ['success' => false, 'message' => $error->errorCode === 'code_exists' ? 'Code already exists, please try again' : $error->getMessage()]);
        } catch (Throwable $error) {
            error_log('[qr] generate: ' . $error->getMessage());
            return $this->json($response, 200, ['success' => false, 'message' => 'Database error']);
        }
        return $this->json($response, 200, ['success' => true, 'code' => $row['code'], 'redirect_to' => $row['redirect_to'], 'type' => $row['type'],
            'email' => $row['email'], 'expires_at' => $row['expires_at']]);
    }

    /** GET get-batches.php → { success, batches } */
    public function batches(Request $request, Response $response): Response
    {
        return $this->json($response, 200, ['success' => true, 'batches' => $this->batches->summaries()]);
    }

    /** GET get-batch.php?file= → the manifest as stored */
    public function batch(Request $request, Response $response): Response
    {
        $filename = basename((string) ($request->getQueryParams()['file'] ?? ''));
        if (!Batches::validFilename($filename)) return $this->json($response, 400, ['success' => false, 'message' => 'Invalid filename']);
        $raw = $this->batches->raw($filename);
        if ($raw === null) return $this->json($response, 404, ['success' => false, 'message' => 'Manifest not found']);
        return Json::raw($response, 200, $raw, 'application/json');
    }

    /** GET download-batch.php?file=&token= → ZIP of the manifest + the files of its finished codes */
    public function download(Request $request, Response $response): Response
    {
        $filename = basename((string) ($request->getQueryParams()['file'] ?? ''));
        if (!Batches::validFilename($filename)) return Json::raw($response, 400, 'Invalid filename', 'text/html; charset=UTF-8');
        if ($this->batches->raw($filename) === null) return Json::raw($response, 404, 'Manifest not found', 'text/html; charset=UTF-8');
        try {
            $zipPath = $this->batches->zip($filename);
        } catch (Throwable $error) {
            error_log('[qr] download-batch: ' . $error->getMessage());
            return Json::raw($response, 500, 'Could not create ZIP archive', 'text/html; charset=UTF-8');
        }
        return Download::file($response, $zipPath, 'application/zip', basename($zipPath));
    }

    /** POST files.php (form: doc, filename, type) → { success } */
    public function files(Request $request, Response $response): Response
    {
        $form = (array) $request->getParsedBody();
        $type = (string) ($form['type'] ?? 'certificates');
        if (!in_array($type, FileStore::TYPES, true)) return $this->json($response, 200, ['success' => false, 'error' => 'Invalid file type']);
        $doc = (string) ($form['doc'] ?? '');
        $filename = (string) ($form['filename'] ?? '');
        if ($doc === '' || $filename === '') return $this->json($response, 200, ['success' => false, 'error' => 'Missing doc or filename']);
        try {
            $this->batches->writeFile($type, $filename, $doc);
        } catch (ApiError $error) {
            $failed = $this->json($response, 200, ['success' => false, 'error' => $error->getMessage()]);
            return str_starts_with($error->getMessage(), 'Invalid JSON') ? $failed->withHeader('Content-Type', 'application/json; charset=utf-8') : $failed;
        }
        return $this->json($response, 200, ['success' => true]);
    }

    /** POST save-signature.php { code, signature } → { success, message, code } */
    public function saveSignature(Request $request, Response $response): Response
    {
        if ($request->getMethod() !== 'POST') return $this->json($response, 405, ['success' => false, 'message' => 'Method not allowed']);
        $data = json_decode(JsonBody::raw($request), true);
        if (!is_array($data) || !isset($data['code']) || !isset($data['signature'])) {
            return $this->json($response, 200, ['success' => false, 'message' => 'Missing required fields: code and signature']);
        }
        try {
            $this->codes->saveSignature((string) $data['code'], $data['signature']);
        } catch (ApiError $error) {
            return $this->json($response, 200, ['success' => false, 'message' => $error->getMessage()]);
        }
        return $this->json($response, 200, ['success' => true, 'message' => 'Signature saved to database', 'code' => $data['code']]);
    }

    /**
     * PUBLIC (no token – used by the verify page /verify.html): GET get-signature.php?code= →
     * { success, signature, fingerprint, qr_data, redirect_to } · 404 when unknown or unsigned
     */
    public function signature(Request $request, Response $response): Response
    {
        if ($request->getMethod() !== 'GET') return $this->json($response, 405, ['success' => false, 'message' => 'Method not allowed']);
        $code = $request->getQueryParams()['code'] ?? '';
        if (!is_string($code) || $code === '') return $this->json($response, 200, ['success' => false, 'message' => 'Missing code parameter']);
        try {
            return $this->json($response, 200, ['success' => true] + $this->codes->signature($code));
        } catch (ApiError $error) {
            return $this->json($response, $error->status === 500 ? 200 : $error->status, ['success' => false, 'message' => $error->getMessage()]);
        }
    }

    /**
     * The signature file of a code (verify page fallback: generate/files/signatures/SIGNATURE-<code>.json)
     * @param array{name: string} $args
     */
    public function signatureFile(Request $request, Response $response, array $args): Response
    {
        $content = $this->files->read('signatures', $args['name']);
        if ($content === null) return Json::raw($response, 404, 'Not found', 'text/plain; charset=UTF-8');
        return Json::raw($response, 200, $content, 'application/json');
    }

    /** GET keys.php → the full keypair (client-side signing) */
    public function keys(Request $request, Response $response): Response
    {
        $keys = $this->keys->keypair();
        if (!$this->keys->exists()) {
            return $this->json($response, 200, ['success' => false, 'error' => 'Keys not initialized. See docs/KEY-MANAGEMENT.md.', 'needsGeneration' => true], true);
        }
        if ($keys === null || !isset($keys['privateKey'], $keys['publicKey'])) {
            error_log('[qr] generate/keys.php: invalid keypair.json');
            return $this->json($response, 500, ['success' => false, 'error' => 'Invalid key file'], true);
        }
        return $this->json($response, 200, ['success' => true, 'privateKey' => $keys['privateKey'], 'publicKey' => $keys['publicKey'], 'generated' => $keys['generated'] ?? null], true);
    }

    /** GET stats.php[?days=60][&format=csv] → { success, days, codes, daily } or the CSV */
    public function stats(Request $request, Response $response): Response
    {
        $query = $request->getQueryParams();
        $days = max(1, min(366, (int) ($query['days'] ?? 60)));
        try {
            $codes = $this->codes->stats();
            if (($query['format'] ?? '') === 'csv') {
                return Download::csv($response, $codes, 'scutoons-code-stats-' . gmdate('Y-m-d') . '.csv');
            }
            return $this->json($response, 200, ['success' => true, 'days' => $days, 'codes' => $codes, 'daily' => $this->codes->daily($days)], true);
        } catch (Throwable $error) {
            error_log('[qr] stats: ' . $error->getMessage());
            return $this->json($response, 500, ['success' => false, 'message' => 'Could not load statistics']);
        }
    }

    /** GET code-events.php?code= → { success, code, events } */
    public function codeEvents(Request $request, Response $response): Response
    {
        $code = CodeRules::normalize($request->getQueryParams()['code'] ?? null);
        if ($code === null) return $this->json($response, 400, ['success' => false, 'message' => 'Missing code'], true);
        try {
            $detail = $this->codes->detail($code);
        } catch (ApiError) {
            return $this->json($response, 404, ['success' => false, 'message' => 'Code not found'], true);
        }
        return $this->json($response, 200, ['success' => true, 'code' => $detail['code'], 'events' => $this->codes->timeline($code)], true);
    }

    /** POST revoke.php { code, action: revoke|unrevoke, reason? } → { success, code, revoked_at, revoked_reason } */
    public function revoke(Request $request, Response $response): Response
    {
        if ($request->getMethod() !== 'POST') return $this->json($response, 405, ['success' => false, 'message' => 'Method not allowed'], true);
        $body = json_decode(JsonBody::raw($request), true) ?: [];
        $code = CodeRules::normalize($body['code'] ?? null);
        $action = $body['action'] ?? '';
        $reason = is_string($body['reason'] ?? null) ? $body['reason'] : '';
        if ($code === null || !in_array($action, ['revoke', 'unrevoke'], true)) {
            return $this->json($response, 400, ['success' => false, 'message' => 'code and action (revoke|unrevoke) required'], true);
        }
        try {
            return $this->json($response, 200, ['success' => true] + $this->codes->setRevoked($code, $action === 'revoke', $reason), true);
        } catch (ApiError) {
            return $this->json($response, 404, ['success' => false, 'message' => 'Code not found'], true);
        }
    }

    /** GET users.php?q=&limit= → { success, roles, total, users } */
    public function users(Request $request, Response $response): Response
    {
        $query = $request->getQueryParams();
        $q = is_string($query['q'] ?? null) ? strtolower(trim(mb_substr($query['q'], 0, 254))) : '';
        return $this->json($response, 200, ['success' => true] + $this->users->search($q, (int) ($query['limit'] ?? 50)), true, true);
    }

    /** POST user-roles.php { user_id | email, role, action: grant|revoke } → { success, changed, user } */
    public function userRoles(Request $request, Response $response): Response
    {
        if ($request->getMethod() !== 'POST') return $this->json($response, 405, ['success' => false, 'message' => 'Method not allowed'], true);
        $body = json_decode(JsonBody::raw($request), true) ?: [];
        $role = is_string($body['role'] ?? null) ? $body['role'] : '';
        $action = $body['action'] ?? '';
        if (!in_array($action, ['grant', 'revoke'], true) || $role === '') {
            return $this->json($response, 400, ['success' => false, 'message' => 'role and action (grant|revoke) required'], true);
        }
        $userId = is_string($body['user_id'] ?? null) ? $body['user_id'] : null;
        if ($userId === null && is_string($body['email'] ?? null)) {
            $userId = $this->users->idByEmail(strtolower(trim($body['email'])));
        }
        try {
            $result = $this->users->setRole((string) $userId, $role, $action === 'grant', 'admin');
        } catch (ApiError $error) {
            return $this->json($response, $error->status, ['success' => false, 'message' => $error->getMessage()], true);
        }
        return $this->json($response, 200, ['success' => true] + $result, true, true);
    }

    /** @param array<string, mixed> $data */
    private function json(Response $response, int $status, array $data, bool $noStore = false, bool $unescaped = false): Response
    {
        $response->getBody()->write((string) json_encode($data, $unescaped ? JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE : self::FLAGS));
        $response = $response->withStatus($status)->withHeader('Content-Type', 'application/json');
        return $noStore ? $response->withHeader('Cache-Control', 'no-store') : $response;
    }
}

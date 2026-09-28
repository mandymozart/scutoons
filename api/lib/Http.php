<?php
declare(strict_types=1);

/** An error answered as `{ "status": "error", "error": "<code>", "message": "…" }` (+ extra fields) */
final class ApiError extends Exception
{
    /** @param array<string, mixed> $extra */
    public function __construct(
        public readonly int $status,
        public readonly string $errorCode,
        string $message = '',
        public readonly array $extra = []
    ) {
        parent::__construct($message !== '' ? $message : $errorCode);
    }
}

/** Request/response helpers: JSON in and out, CORS, Bearer token */
final class Http
{
    /** @return array<string, mixed> */
    public static function body(): array
    {
        static $body = null;
        if ($body !== null) return $body;
        $raw = (string) file_get_contents('php://input');
        if (trim($raw) === '') return $body = [];
        $data = json_decode($raw, true);
        if (!is_array($data)) throw new ApiError(400, 'invalid_json', 'The request body is not a JSON object.');
        return $body = $data;
    }

    public static function json(int $status, mixed $data): never
    {
        http_response_code($status);
        header('Content-Type: application/json; charset=utf-8');
        header('Cache-Control: no-store');
        echo json_encode($data, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
        exit;
    }

    public static function error(ApiError $error): never
    {
        self::json($error->status, ['status' => 'error', 'error' => $error->errorCode, 'message' => $error->getMessage()] + $error->extra);
    }

    /** `Authorization: Bearer <token>` (Apache may hide it – see .htaccess) */
    public static function bearerToken(): ?string
    {
        $header = $_SERVER['HTTP_AUTHORIZATION'] ?? $_SERVER['REDIRECT_HTTP_AUTHORIZATION'] ?? '';
        if ($header === '' && function_exists('getallheaders')) {
            foreach (getallheaders() as $name => $value) {
                if (strcasecmp((string) $name, 'Authorization') === 0) $header = (string) $value;
            }
        }
        return preg_match('/^Bearer\s+([A-Za-z0-9_-]{20,128})$/', trim($header), $m) ? $m[1] : null;
    }

    public static function origin(): ?string
    {
        $origin = $_SERVER['HTTP_ORIGIN'] ?? '';
        return $origin !== '' ? rtrim($origin, '/') : null;
    }

    public static function isAllowedOrigin(string $origin): bool
    {
        foreach (Config::list('ALLOWED_ORIGINS') as $allowed) {
            if (strcasecmp(rtrim($allowed, '/'), $origin) === 0) return true;
        }
        return false;
    }

    /** CORS headers for allowed origins; a preflight (OPTIONS) request ends here */
    public static function cors(): void
    {
        $origin = self::origin();
        if ($origin !== null && self::isAllowedOrigin($origin)) {
            header('Access-Control-Allow-Origin: ' . $origin);
            header('Vary: Origin');
            header('Access-Control-Allow-Methods: GET, POST, PUT, OPTIONS');
            header('Access-Control-Allow-Headers: Content-Type, Authorization');
            header('Access-Control-Max-Age: 86400');
        }
        if (($_SERVER['REQUEST_METHOD'] ?? 'GET') === 'OPTIONS') {
            http_response_code(204);
            exit;
        }
    }

    public static function clientIp(): string
    {
        return (string) ($_SERVER['REMOTE_ADDR'] ?? '');
    }

    /** Runs an endpoint: ApiError → its JSON error, anything else → logged, generic 500 */
    public static function run(callable $handler): never
    {
        try {
            self::cors();
            $handler();
            throw new ApiError(500, 'server_error', 'No response.');
        } catch (ApiError $error) {
            self::error($error);
        } catch (Throwable $error) {
            error_log('[qr-api] ' . $error::class . ': ' . $error->getMessage() . ' in ' . $error->getFile() . ':' . $error->getLine());
            self::error(new ApiError(500, 'server_error', 'Something went wrong on the server.'));
        }
    }
}

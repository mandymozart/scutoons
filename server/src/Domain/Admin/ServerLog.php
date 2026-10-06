<?php
declare(strict_types=1);

namespace Scutoons\Domain\Admin;

/**
 * The newest entries of a PHP error log (BACKLOG 10.11, owner 2026-10-02: the dashboard's 500 was only found by reading
 * /web/api/logs/error.log over FTP). Reads at most the last MAX_BYTES of the file, groups an entry with its following
 * lines (a stack trace), newest first. Messages only – what could be personal or secret is masked (e-mail addresses,
 * tokens / keys / hashes, passwords in a DSN or a query) and the server's own folder is cut from file paths.
 */
final class ServerLog
{
    public const MAX_BYTES = 512 * 1024;
    public const MAX_ENTRIES = 500;
    /** `[07-Oct-2026 00:12:34 Europe/Berlin] PHP Warning:  …` (PHP's error_log line) */
    private const HEAD = '/^\[(\d{2})-([A-Za-z]{3})-(\d{4}) (\d{2}):(\d{2}):(\d{2})(?: ([^\]]+))?\] ?(.*)$/';
    private const MONTHS = ['Jan' => 1, 'Feb' => 2, 'Mar' => 3, 'Apr' => 4, 'May' => 5, 'Jun' => 6, 'Jul' => 7, 'Aug' => 8, 'Sep' => 9, 'Oct' => 10, 'Nov' => 11, 'Dec' => 12];

    /**
     * @return array{exists: bool, size: int, truncated: bool, days: list<string>, entries: list<array{at: ?string, day: ?string, level: string, message: string, detail: string}>}
     */
    public static function read(string $path, ?string $day = null, string $q = '', int $limit = 200, string $root = ''): array
    {
        if ($path === '' || !is_file($path) || !is_readable($path)) {
            return ['exists' => false, 'size' => 0, 'truncated' => false, 'days' => [], 'entries' => []];
        }
        $size = (int) filesize($path);
        $from = max(0, $size - self::MAX_BYTES);
        $text = (string) file_get_contents($path, false, null, $from);
        // a cut first line belongs to an older entry
        if ($from > 0) $text = (string) substr($text, (int) (strpos($text, "\n") === false ? strlen($text) : strpos($text, "\n") + 1));
        $entries = self::parse($text, $root);
        $days = array_values(array_unique(array_filter(array_map(fn(array $e) => $e['day'], $entries))));
        rsort($days);
        $needle = mb_strtolower(trim($q));
        $out = [];
        for ($i = count($entries) - 1; $i >= 0 && count($out) < min(max(1, $limit), self::MAX_ENTRIES); $i--) {
            $e = $entries[$i];
            if ($day !== null && $day !== '' && $e['day'] !== $day) continue;
            if ($needle !== '' && !str_contains(mb_strtolower($e['message'] . "\n" . $e['detail']), $needle)) continue;
            $out[] = $e;
        }
        return ['exists' => true, 'size' => $size, 'truncated' => $from > 0, 'days' => $days, 'entries' => $out];
    }

    /** @return list<array{at: ?string, day: ?string, level: string, message: string, detail: string}> oldest first */
    public static function parse(string $text, string $root = ''): array
    {
        $entries = [];
        foreach (preg_split('/\r?\n/', $text) ?: [] as $line) {
            if ($line === '') continue;
            if (preg_match(self::HEAD, $line, $m) === 1 && isset(self::MONTHS[$m[2]])) {
                $at = sprintf('%04d-%02d-%02dT%02d:%02d:%02d', (int) $m[3], self::MONTHS[$m[2]], (int) $m[1], (int) $m[4], (int) $m[5], (int) $m[6]);
                [$level, $message] = self::level($m[8]);
                $entries[] = ['at' => $at, 'day' => substr($at, 0, 10), 'level' => $level, 'message' => self::clean($message, $root), 'detail' => ''];
            } elseif ($entries !== []) {
                $last = count($entries) - 1;
                $entries[$last]['detail'] .= ($entries[$last]['detail'] === '' ? '' : "\n") . self::clean($line, $root);
            } else {
                // a line before any dated entry (a log without PHP's prefix)
                [$level, $message] = self::level($line);
                $entries[] = ['at' => null, 'day' => null, 'level' => $level, 'message' => self::clean($message, $root), 'detail' => ''];
            }
        }
        foreach ($entries as &$e) {
            if (strlen($e['detail']) > 4000) $e['detail'] = substr($e['detail'], 0, 4000) . "\n…";
        }
        return $entries;
    }

    /** @return array{0: string, 1: string} `PHP Fatal error:  Uncaught …` → [fatal, Uncaught …] */
    private static function level(string $message): array
    {
        if (preg_match('/^PHP (Fatal error|Parse error|Warning|Notice|Deprecated|Strict Standards|Recoverable fatal error):\s*(.*)$/s', $message, $m) === 1) {
            $level = match ($m[1]) {
                'Fatal error', 'Parse error', 'Recoverable fatal error' => 'fatal',
                'Warning' => 'warning',
                default => 'notice',
            };
            return [$level, $m[2]];
        }
        return ['error', $message];
    }

    /** masks what could be personal or secret; the server's folder is cut from paths */
    public static function clean(string $text, string $root = ''): string
    {
        if ($root !== '') $text = str_replace([rtrim($root, '/') . '/', str_replace('/', '\\', rtrim($root, '/')) . '\\'], '', $text);
        $text = (string) preg_replace('/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/', '[email]', $text);
        // a header's whole value ("Authorization: Bearer …"), else the value after the name
        $text = (string) preg_replace('/((?:authorization|cookie|x-service-key)\s*:\s*).*/i', '$1[hidden]', $text);
        $text = (string) preg_replace('/((?:password|passwd|pwd|secret|token|api[_-]?key)\s*[=:]\s*)("[^"]*"|\'[^\']*\'|\S+)/i', '$1[hidden]', $text);
        $text = (string) preg_replace('#(://[^:/\s]+:)[^@\s]+@#', '$1[hidden]@', $text);
        // a long run of letters AND digits (a token, key, hash) – not a long class name
        $text = (string) preg_replace('/\b(?:Bearer\s+)?(?=[A-Za-z_\-]*\d)(?=[0-9_\-]*[A-Za-z])[A-Za-z0-9_\-]{32,}\b/', '[token]', $text);
        return $text;
    }
}

<?php
declare(strict_types=1);

namespace Qr;

use RuntimeException;
use Throwable;

/**
 * Plain PHP templates in templates/ (the landing page). A template gets its variables as locals plus `$e` (HTML
 * escape) and `$view` (this renderer, for partials). Same renderer the server used for these templates.
 */
final class View
{
    public function __construct(private readonly string $directory)
    {
    }

    /** @param array<string, mixed> $vars */
    public function render(string $name, array $vars = []): string
    {
        $file = $this->directory . '/' . $name . '.php';
        if (!is_file($file)) throw new RuntimeException("Template not found: $name");
        $e = static fn(mixed $value): string => htmlspecialchars(is_scalar($value) ? (string) $value : '', ENT_QUOTES, 'UTF-8');
        $view = $this;
        ob_start();
        try {
            (static function (string $__file, array $__vars, callable $e, View $view): void {
                extract($__vars, EXTR_SKIP);
                require $__file;
            })($file, $vars, $e, $view);
        } catch (Throwable $error) {
            ob_end_clean();
            throw $error;
        }
        return (string) ob_get_clean();
    }

    /** @return array<mixed> a PHP file in templates/ that returns data (the texts) */
    public function data(string $name): array
    {
        $file = $this->directory . '/' . $name . '.php';
        $data = is_file($file) ? require $file : null;
        if (!is_array($data)) throw new RuntimeException("Template data not found: $name");
        return $data;
    }
}

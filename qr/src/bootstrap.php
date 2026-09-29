<?php
declare(strict_types=1);

/**
 * The qr app has no dependencies (plain PHP, no Composer packages): classes `Qr\Foo` live in src/Foo.php.
 */
spl_autoload_register(static function (string $class): void {
    if (!str_starts_with($class, 'Qr\\')) return;
    $file = __DIR__ . '/' . str_replace('\\', '/', substr($class, 3)) . '.php';
    if (is_file($file)) require $file;
});

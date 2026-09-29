<?php
declare(strict_types=1);

/** Reader accounts (email only) */
final class Users
{
    /** @return array<string, mixed>|null */
    public static function byEmail(string $email): ?array
    {
        return Db::one('SELECT * FROM users WHERE email = ?', [$email]);
    }

    /** @return array<string, mixed>|null */
    public static function byId(string $id): ?array
    {
        return Db::one('SELECT * FROM users WHERE id = ?', [$id]);
    }

    /**
     * Creates the user (after a confirmed login) and grants the starter items. Call inside a transaction.
     * @return array<string, mixed>
     */
    public static function create(string $email, string $locale): array
    {
        $id = Db::uuid();
        $now = Db::now();
        Db::run('INSERT INTO users (id, email, locale, created_at, confirmed_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)',
            [$id, $email, $locale, $now, $now, $now]);
        foreach (Catalog::starters() as $item) {
            Inventory::add($id, $item['id'], null, 1, 'starter');
        }
        return self::byId($id);
    }

    /** @param array<string, mixed> $row */
    public static function toJson(array $row): array
    {
        return [
            'id' => $row['id'],
            'email' => $row['email'],
            'created_at' => Db::iso($row['created_at']),
        ];
    }
}

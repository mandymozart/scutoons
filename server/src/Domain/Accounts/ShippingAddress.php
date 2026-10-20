<?php
declare(strict_types=1);

namespace Scutoons\Domain\Accounts;

use Scutoons\Domain\ApiError;
use Scutoons\Infrastructure\Database\Database;

/**
 * The account's shipping address (users.ship_*, migration 2026-10-20-campaign-giveaways; owner 2026-10-20: "die muss
 * vom reader im account nach getragen werden"): entered on the account page (Stammdaten), used by the physical gifts
 * of giveaway campaigns (Campaigns\Giveaways) – a claimed one waits for it. Only the account itself and the admin's
 * list of gifts to send see it.
 */
final class ShippingAddress
{
    public function __construct(private readonly Database $db)
    {
    }

    /** @return array{name: string, address: string, country: string}|null */
    public function of(string $userId): ?array
    {
        $row = $this->db->one('SELECT ship_name, ship_address, ship_country FROM users WHERE id = ?', [$userId]);
        if ($row === null || ($row['ship_name'] ?? null) === null || ($row['ship_address'] ?? null) === null || ($row['ship_country'] ?? null) === null) return null;
        return ['name' => (string) $row['ship_name'], 'address' => (string) $row['ship_address'], 'country' => (string) $row['ship_country']];
    }

    /**
     * The address when it is in one of $countries (none listed = anywhere)
     * @param list<string> $countries
     * @return array{name: string, address: string, country: string}|null
     */
    public function forCountries(string $userId, array $countries): ?array
    {
        $address = $this->of($userId);
        if ($address === null || ($countries !== [] && !in_array($address['country'], $countries, true))) return null;
        return $address;
    }

    /**
     * PUT /api/me/shipping { name, address, country } – all three, or all empty to remove it.
     * 400 invalid_address / invalid_country.
     * @param array<string, mixed> $body
     * @return array{name: string, address: string, country: string}|null
     */
    public function set(string $userId, array $body): ?array
    {
        $name = is_string($body['name'] ?? null) ? trim($body['name']) : '';
        $address = is_string($body['address'] ?? null) ? trim(str_replace("\r", '', $body['address'])) : '';
        $country = strtoupper(is_string($body['country'] ?? null) ? trim($body['country']) : '');
        if ($name === '' && $address === '' && $country === '') {
            $this->db->run('UPDATE users SET ship_name = NULL, ship_address = NULL, ship_country = NULL WHERE id = ?', [$userId]);
            return null;
        }
        if ($name === '' || mb_strlen($name) > 200 || $address === '' || mb_strlen($address) > 1000) throw new ApiError(400, 'invalid_address', 'Name and address, please.');
        if (!preg_match('/^[A-Z]{2}$/', $country)) throw new ApiError(400, 'invalid_country', 'country: a two-letter country code (DE, AT, …).');
        $this->db->run('UPDATE users SET ship_name = ?, ship_address = ?, ship_country = ? WHERE id = ?', [$name, $address, $country, $userId]);
        return $this->of($userId);
    }
}

<?php
declare(strict_types=1);

namespace Scutoons\Tests\Domain;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Embassy\Embassy;
use Scutoons\Domain\Embassy\Stockists;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Tests\TestCase;

/** Embassy: access, organisations, cadence after mails and calls, the Today list, duplicates and the import */
final class EmbassyTest extends TestCase
{
    /** @return array<string, mixed> */
    private function member(string $role, string $email = ''): array
    {
        $user = $this->user($email);
        if ($role !== '') $this->get(Roles::class)->grant((string) $user['id'], $role);
        return $user;
    }

    private function embassy(): Embassy
    {
        return $this->get(Embassy::class);
    }

    private function error(callable $fn): ApiError
    {
        try {
            $fn();
        } catch (ApiError $e) {
            return $e;
        }
        self::fail('Expected an ApiError');
    }

    public function testAccessByRole(): void
    {
        self::assertSame(401, $this->error(fn() => $this->embassy()->access(null))->status);
        self::assertSame(403, $this->error(fn() => $this->embassy()->access($this->member('')))->status);
        self::assertSame('liaison', $this->embassy()->access($this->member('liaison')));
        self::assertSame('ambassador', $this->embassy()->access($this->member('ambassador')));
        self::assertSame('ambassador', $this->embassy()->access($this->member(Roles::ADMIN)));
    }

    public function testCreateNormalizesAndRejectsDuplicates(): void
    {
        $amb = $this->member('ambassador', 'amb@example.com');
        $org = $this->embassy()->create([
            'name' => ' Institut für die Zukunft ', 'type' => 'venue', 'city' => 'Leipzig', 'country' => 'de', 'website' => 'ifz.me',
            'tags' => 'techno, club', 'people' => [['name' => 'Kathi', 'email' => 'booking@ifz.me']], 'leads' => [['track' => 'live']],
        ], $amb);
        self::assertSame(['Institut für die Zukunft', 'DE', 'https://ifz.me', ['techno', 'club']], [$org['org']['name'], $org['org']['country'], $org['org']['website'], $org['org']['tags']]);
        self::assertCount(1, $org['people']);
        self::assertSame(['live', 'research', (string) $amb['id']], [$org['leads'][0]['track'], $org['leads'][0]['stage'], $org['leads'][0]['owner_id']]);

        $dup = $this->error(fn() => $this->embassy()->create(['name' => 'institut fur die zukunft', 'city' => 'leipzig'], $amb));
        self::assertSame([409, 'duplicate', $org['org']['id']], [$dup->status, $dup->errorCode, $dup->extra['id']]);
        // other city = other organisation
        self::assertNotSame($org['org']['id'], $this->embassy()->create(['name' => 'Institut für die Zukunft', 'city' => 'Wien'], $amb)['org']['id']);
        self::assertSame('invalid_type', $this->error(fn() => $this->embassy()->create(['name' => 'X', 'type' => 'castle'], $amb))->errorCode);
        self::assertSame('invalid_country', $this->error(fn() => $this->embassy()->create(['name' => 'X', 'country' => 'Austria'], $amb))->errorCode);
    }

    public function testCadenceMailThenCalls(): void
    {
        $liaison = $this->member('liaison');
        $id = $this->embassy()->create(['name' => 'Dachstock', 'city' => 'Bern', 'country' => 'CH', 'phone' => '+41 1'], $liaison)['org']['id'];

        // 2026-09-21 (a Monday) is the frozen "today"; + 6 days = Sunday → Monday
        $r = $this->embassy()->log($id, ['kind' => 'mail_out', 'track' => 'live', 'note' => 'EPK sent'], $liaison);
        self::assertSame(['contacted', '2026-09-28'], [$r['lead']['stage'] ?? null, $r['lead']['next_at'] ?? null]);
        self::assertNotNull($r['lead']['last_contact_at'] ?? null);

        $r = $this->embassy()->log($id, ['kind' => 'call', 'track' => 'live', 'outcome' => 'voicemail'], $liaison);
        self::assertSame(['contacted', '2026-09-24'], [$r['lead']['stage'] ?? null, $r['lead']['next_at'] ?? null]);

        self::assertSame('next_at_required', $this->error(fn() => $this->embassy()->log($id, ['kind' => 'call', 'track' => 'live', 'outcome' => 'call_back'], $liaison))->errorCode);
        $r = $this->embassy()->log($id, ['kind' => 'call', 'track' => 'live', 'outcome' => 'call_back', 'next_at' => '2026-10-02'], $liaison);
        self::assertSame('2026-10-02', $r['lead']['next_at'] ?? null);

        $r = $this->embassy()->log($id, ['kind' => 'call', 'track' => 'live', 'outcome' => 'reached'], $liaison);
        self::assertSame(['in_conversation', '2026-10-01'], [$r['lead']['stage'] ?? null, $r['lead']['next_at'] ?? null]);

        $r = $this->embassy()->log($id, ['kind' => 'note', 'track' => 'live', 'note' => 'likes the record'], $liaison);
        self::assertSame(['in_conversation', '2026-10-01'], [$r['lead']['stage'] ?? null, $r['lead']['next_at'] ?? null]);

        $r = $this->embassy()->log($id, ['kind' => 'call', 'track' => 'live', 'outcome' => 'declined'], $liaison);
        self::assertSame(['no', null], [$r['lead']['stage'] ?? null, $r['lead']['next_at'] ?? null]);

        self::assertCount(6, $this->embassy()->get($id)['activities']);
        self::assertSame('invalid_outcome', $this->error(fn() => $this->embassy()->log($id, ['kind' => 'call'], $liaison))->errorCode);
    }

    public function testTodayListsDueLeadsOfTheOwner(): void
    {
        $a = $this->member('liaison', 'a@example.com');
        $b = $this->member('liaison', 'b@example.com');
        $one = $this->embassy()->create(['name' => 'One', 'leads' => [['track' => 'live', 'next_at' => '2026-09-20']]], $a)['org']['id'];
        $this->embassy()->create(['name' => 'Two', 'leads' => [['track' => 'promo', 'next_at' => '2026-09-21']]], $a);
        $this->embassy()->create(['name' => 'Later', 'leads' => [['track' => 'live', 'next_at' => '2026-09-30']]], $a);
        $this->embassy()->create(['name' => 'Theirs', 'leads' => [['track' => 'live', 'next_at' => '2026-09-01']]], $b);
        $this->embassy()->create(['name' => 'Blocked', 'do_not_contact' => true, 'leads' => [['track' => 'live', 'next_at' => '2026-09-01']]], $a);
        $this->embassy()->create(['name' => 'Fresh', 'leads' => [['track' => 'vinyl']]], $a);

        $today = $this->embassy()->today('me', $a);
        self::assertSame('2026-09-21', $today['today']);
        self::assertSame(['One', 'Two'], array_map(fn(array $d) => $d['org']['name'], $today['due']));
        self::assertSame([1, 0], array_map(fn(array $d) => $d['overdue_days'], $today['due']));
        self::assertSame(1, $today['unscheduled']);
        self::assertCount(3, $this->embassy()->today('all', $a)['due']);

        // a liaison can't work somebody else's lead, an ambassador can and may hand it over
        self::assertSame('not_your_lead', $this->error(fn() => $this->embassy()->log($one, ['kind' => 'call', 'track' => 'live', 'outcome' => 'reached'], $b))->errorCode);
        self::assertSame(403, $this->error(fn() => $this->embassy()->setLead($one, 'live', ['owner_id' => $b['id']], $a))->status);
        $amb = $this->member('ambassador');
        self::assertSame($b['id'], $this->embassy()->setLead($one, 'live', ['owner_id' => $b['id']], $amb)['owner_id']);
        self::assertSame(['Theirs', 'One'], array_map(fn(array $d) => $d['org']['name'], $this->embassy()->today('me', $b)['due']));
    }

    public function testListFilters(): void
    {
        $amb = $this->member('ambassador');
        $this->embassy()->create(['name' => 'Cashmere Radio', 'type' => 'radio', 'country' => 'DE', 'leads' => [['track' => 'promo']]], $amb);
        $this->embassy()->create(['name' => 'Kiosk Radio', 'type' => 'radio', 'country' => 'BE', 'tier' => 'a', 'leads' => [['track' => 'promo', 'stage' => 'won']]], $amb);
        $this->embassy()->create(['name' => 'Powerline Agency', 'type' => 'agency', 'country' => 'DE', 'territories' => ['de', 'at', 'ch'], 'leads' => [['track' => 'booking']]], $amb);

        self::assertSame(3, $this->embassy()->list([], $amb)['total']);
        self::assertSame(['Cashmere Radio', 'Kiosk Radio'], array_column($this->embassy()->list(['type' => 'radio'], $amb)['items'], 'name'));
        self::assertSame(['Kiosk Radio'], array_column($this->embassy()->list(['stage' => 'won'], $amb)['items'], 'name'));
        self::assertSame(['Kiosk Radio'], array_column($this->embassy()->list(['tier' => 'A'], $amb)['items'], 'name'));
        self::assertSame(['Cashmere Radio', 'Powerline Agency'], array_column($this->embassy()->list(['country' => 'de'], $amb)['items'], 'name'));
        self::assertSame(['Powerline Agency'], array_column($this->embassy()->list(['track' => 'booking'], $amb)['items'], 'name'));
        self::assertSame(['DE', 'AT', 'CH'], $this->embassy()->list(['q' => 'power'], $amb)['items'][0]['territories']);
    }

    public function testImportMergesAndNeedsAnAmbassador(): void
    {
        $amb = $this->member('ambassador');
        $data = ['organisations' => [
            ['name' => 'Cashmere Radio', 'type' => 'radio', 'city' => 'Berlin', 'website' => 'https://cashmereradio.com', 'leads' => [['track' => 'promo']]],
            ['name' => 'Cashmere Radio', 'type' => 'radio', 'city' => 'Berlin', 'notes' => 'community radio in Wedding', 'tags' => ['experimental']],
            ['name' => 'Powerline Agency', 'type' => 'agency', 'people' => [['name' => 'Wieland', 'email' => 'wk@powerline-agency.com'], ['name' => 'Mathias', 'email' => 'ms@powerline-agency.com']]],
            ['name' => '', 'type' => 'venue'],
        ]];
        self::assertSame(403, $this->error(fn() => $this->embassy()->import($data, $this->member('liaison')))->status);

        $dry = $this->embassy()->import($data + ['dry_run' => true], $amb);
        self::assertSame([true, 2, 1, 2, 1, 1], [$dry['dry_run'], $dry['created'], $dry['merged'], $dry['people_added'], $dry['leads_added'], count($dry['errors'])]);
        self::assertSame(0, $this->embassy()->list([], $amb)['total']);

        $report = $this->embassy()->import($data, $amb);
        self::assertSame([2, 1, 2, 1], [$report['created'], $report['merged'], $report['people_added'], $report['leads_added']]);
        $cashmere = $this->embassy()->list(['q' => 'cashmere'], $amb)['items'][0];
        self::assertSame(['community radio in Wedding', ['experimental'], null], [$cashmere['notes'], $cashmere['tags'], $cashmere['leads'][0]['owner_id']]);

        // working an unowned lead takes it
        $liaison = $this->member('liaison');
        self::assertNull($this->embassy()->log($cashmere['id'], ['kind' => 'note', 'track' => 'promo', 'note' => 'x'], $liaison)['lead']['owner_id'] ?? null);
        self::assertSame($liaison['id'], $this->embassy()->log($cashmere['id'], ['kind' => 'mail_out', 'track' => 'promo'], $liaison)['lead']['owner_id'] ?? null);

        // again: nothing new
        $again = $this->embassy()->import($data, $amb);
        self::assertSame([0, 3, 0, 0], [$again['created'], $again['merged'], $again['people_added'], $again['leads_added']]);
    }

    public function testDeleteNeedsAnAmbassador(): void
    {
        $liaison = $this->member('liaison');
        $id = $this->embassy()->create(['name' => 'Gone', 'people' => [['name' => 'P']]], $liaison)['org']['id'];
        $this->embassy()->log($id, ['kind' => 'note', 'note' => 'x'], $liaison);
        self::assertSame(403, $this->error(fn() => $this->embassy()->delete($id, $liaison))->status);
        $this->embassy()->delete($id, $this->member('ambassador'));
        self::assertSame(404, $this->error(fn() => $this->embassy()->get($id))->status);
    }

    public function testStoreFieldsAndTheStockistList(): void
    {
        $liaison = $this->member('liaison');
        $store = $this->embassy()->create([
            'name' => 'Testladen', 'type' => 'store', 'city' => 'Wien', 'country' => 'at', 'website' => 'testladen.example',
            'email' => 'secret@testladen.example', 'phone' => '+43 1 000', 'notes' => 'internal', 'address' => ' Teststraße 1, 1010 Wien ',
            'opening' => 'Mo–Fr 10–18', 'listed' => true, 'independent' => true, 'carries' => ['awakening_vinyl', 'dago_hoodie'],
            'people' => [['name' => 'Owner', 'email' => 'owner@testladen.example']],
        ], $liaison)['org'];
        self::assertSame([true, true, 'Teststraße 1, 1010 Wien', 'Mo–Fr 10–18', ['awakening_vinyl', 'dago_hoodie']],
            [$store['listed'], $store['independent'], $store['address'], $store['opening'], $store['carries']]);
        self::assertSame('invalid_product', $this->error(fn() => $this->embassy()->update($store['id'], ['carries' => ['no_such_product']], $liaison))->errorCode);

        // a chain (not independent), a record shop in Germany, an unlisted store, a listed venue, a store carrying nothing
        $chain = $this->embassy()->create(['name' => 'Kettenmarkt', 'type' => 'store', 'city' => 'Graz', 'country' => 'AT', 'listed' => true, 'carries' => ['awakening_vinyl']], $liaison)['org'];
        $records = $this->embassy()->create(['name' => 'Plattenkiste', 'type' => 'record_shop', 'city' => 'Berlin', 'country' => 'DE', 'listed' => true, 'independent' => true, 'carries' => ['awakening_vinyl'], 'website' => 'mailto:x@y.z'], $liaison)['org'];
        $this->embassy()->create(['name' => 'Hidden', 'type' => 'store', 'city' => 'Linz', 'country' => 'AT', 'listed' => false, 'carries' => ['awakening_vinyl']], $liaison);
        $this->embassy()->create(['name' => 'Club', 'type' => 'venue', 'city' => 'Linz', 'country' => 'AT', 'listed' => true, 'carries' => ['awakening_vinyl']], $liaison);
        $this->embassy()->create(['name' => 'Empty', 'type' => 'store', 'city' => 'Linz', 'country' => 'AT', 'listed' => true], $liaison);

        $stockists = $this->get(Stockists::class);
        $all = $stockists->list();
        // independent first (country, city, name), then the others
        self::assertSame(['Testladen', 'Plattenkiste', 'Kettenmarkt'], array_column($all['stores'], 'name'));
        self::assertSame(['awakening_vinyl' => 3, 'dago_hoodie' => 1], $all['products']);
        self::assertSame(['AT', 'DE'], $all['countries']);
        // public fields only
        self::assertSame(['id', 'name', 'type', 'independent', 'city', 'country', 'address', 'website', 'opening', 'carries'], array_keys($all['stores'][0]));
        self::assertSame('https://testladen.example', $all['stores'][0]['website']);
        self::assertSame('', $all['stores'][1]['website'], 'only http(s) links are public');

        self::assertSame([$store['id']], array_column($stockists->list(['product' => 'dago_hoodie'])['stores'], 'id'));
        self::assertSame([$records['id']], array_column($stockists->list(['country' => 'de'])['stores'], 'id'));
        self::assertSame(['Testladen', 'Kettenmarkt'], array_column($stockists->list(['country' => 'AT', 'product' => 'awakening_vinyl'])['stores'], 'name'));
        self::assertSame([], $stockists->list(['product' => 'unknown_product'])['stores']);
        self::assertSame('invalid_country', $this->error(fn() => $stockists->list(['country' => 'Austria']))->errorCode);

        // unlisting removes a store; the import merges carries
        $this->embassy()->update($chain['id'], ['listed' => false], $liaison);
        self::assertSame(['Testladen', 'Plattenkiste'], array_column($stockists->list()['stores'], 'name'));
        $this->embassy()->import(['organisations' => [['name' => 'Testladen', 'city' => 'Wien', 'carries' => ['scorpion_amulet']]]], $this->member('ambassador'));
        self::assertSame(['awakening_vinyl', 'dago_hoodie', 'scorpion_amulet'], $this->embassy()->get($store['id'])['org']['carries']);
    }

    public function testNameKey(): void
    {
        self::assertSame('institutfurdiezukunft', Embassy::nameKey(' Institut für die Zukunft '));
        self::assertSame('trezorsa', Embassy::nameKey('@trezor_sa'));
        self::assertSame('partae', Embassy::nameKey('The Partae'));
    }
}

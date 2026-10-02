<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\CodeAdmin;
use Scutoons\Domain\Admin\ListQuery;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Codes\CodeRules;
use Scutoons\Domain\Ownership\OwnershipTransfers;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Download;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\Config\Settings;

/**
 * /api/admin/codes – role admin (docs/API.md "Admin API"):
 *   GET  /codes                       list + statistics (page, page_size, sort, q, edition_id, type, status); ?format=csv
 *   POST /codes                       create one code
 *   GET  /codes/{code}                detail (no email, no owner id)
 *   GET  /codes/{code}/events         timeline (?limit=, newest first)
 *   POST /codes/{code}/revoke|unrevoke
 *   GET|PUT /codes/{code}/signature   read / store the certificate signature
 *   GET  /codes/{code}/history        ownership history (transfers: claim, gift, admin – with accounts)
 *   POST /codes/{code}/transfer       { to: email | user id, note, item_id?, variant? } – move it to another account
 */
final class CodesController
{
    public function __construct(private readonly CodeAdmin $codes, private readonly OwnershipTransfers $transfers, private readonly Settings $settings)
    {
    }

    public function list(Request $request, Response $response): Response
    {
        $params = $request->getQueryParams();
        $query = ListQuery::from($params, CodeAdmin::SORTABLE, 'code', CodeAdmin::FILTERS, 50, $this->settings->int('ADMIN_MAX_PAGE_SIZE'));
        if (($params['format'] ?? '') === 'csv') {
            return Download::csv($response, $this->codes->export($query), 'scutoons-codes-' . gmdate('Y-m-d') . '.csv');
        }
        return Json::response($response, 200, ['status' => 'success'] + $this->codes->list($query));
    }

    public function create(Request $request, Response $response): Response
    {
        return Json::response($response, 201, ['status' => 'success', 'code' => $this->codes->create(JsonBody::of($request))]);
    }

    /** @param array{code: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->codes->detail(self::code($args)));
    }

    /** @param array{code: string} $args */
    public function events(Request $request, Response $response, array $args): Response
    {
        $code = self::code($args);
        $this->codes->detail($code); // 404 for unknown codes
        $limit = (int) ($request->getQueryParams()['limit'] ?? 1000);
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->codes->timeline($code, $limit)]);
    }

    /** @param array{code: string} $args */
    public function revoke(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        $reason = is_string($body['reason'] ?? null) ? $body['reason'] : '';
        return Json::response($response, 200, ['status' => 'success'] + $this->codes->setRevoked(self::code($args), true, $reason));
    }

    /** @param array{code: string} $args */
    public function unrevoke(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->codes->setRevoked(self::code($args), false));
    }

    /** @param array{code: string} $args */
    public function signature(Request $request, Response $response, array $args): Response
    {
        return Json::response($response, 200, ['status' => 'success'] + $this->codes->signature(self::code($args)));
    }

    /** @param array{code: string} $args */
    public function saveSignature(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        $code = self::code($args);
        $this->codes->saveSignature($code, $body['signature'] ?? null);
        return Json::response($response, 200, ['status' => 'success', 'code' => $code]);
    }

    /** @param array{code: string} $args */
    public function history(Request $request, Response $response, array $args): Response
    {
        $code = self::code($args);
        $this->codes->detail($code); // 404 for unknown codes
        return Json::response($response, 200, ['status' => 'success', 'items' => $this->transfers->history($code)]);
    }

    /** @param array{code: string} $args */
    public function transfer(Request $request, Response $response, array $args): Response
    {
        $body = JsonBody::of($request);
        $admin = CurrentSession::of($request)->requireUser();
        $code = self::code($args);
        $this->codes->detail($code); // 404 for unknown codes
        $moved = $this->transfers->adminTransfer($admin, $code, $body);
        return Json::response($response, 200, ['status' => 'success', 'code' => $code, 'from' => $moved['from_account'], 'to' => $moved['to_account'],
            'entries' => count($moved['entries']), 'history' => $this->transfers->history($code)]);
    }

    /** @param array{code: string} $args */
    private static function code(array $args): string
    {
        return CodeRules::normalize($args['code']) ?? throw new ApiError(400, 'invalid_code', 'Code missing or malformed.');
    }
}

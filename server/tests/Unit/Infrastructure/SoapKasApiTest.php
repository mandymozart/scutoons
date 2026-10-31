<?php
declare(strict_types=1);

namespace Scutoons\Tests\Infrastructure;

use ArrayObject;
use PHPUnit\Framework\TestCase;
use Scutoons\Infrastructure\Kas\KasFault;
use Scutoons\Infrastructure\Kas\SoapKasApi;

/**
 * The KAS API's wire format (#201, Infrastructure\Kas\SoapKasApi) against recorded answers – no call leaves the test:
 * the session from KasAuth, the action with KasRequestParams, SOAP-encoded maps and arrays, faults, the flood delay.
 */
final class SoapKasApiTest extends TestCase
{
    private const AUTH_ANSWER = '<?xml version="1.0" encoding="UTF-8"?><SOAP-ENV:Envelope xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:ns1="urn:xmethodsKasApiAuthentication" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><SOAP-ENV:Body><ns1:KasAuthResponse><return xsi:type="xsd:string">session-token-1</return></ns1:KasAuthResponse></SOAP-ENV:Body></SOAP-ENV:Envelope>';

    private static function apiAnswer(string $returnInfo, string $flood = '0.5'): string
    {
        return '<?xml version="1.0" encoding="UTF-8"?><SOAP-ENV:Envelope xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:ns1="urn:xmethodsKasApi" xmlns:ns2="http://xml.apache.org/xml-soap" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/"><SOAP-ENV:Body><ns1:KasApiResponse>'
            . '<return xsi:type="ns2:Map"><item><key xsi:type="xsd:string">Request</key><value xsi:type="ns2:Map"><item><key xsi:type="xsd:string">KasRequestTime</key><value xsi:type="xsd:int">1790000000</value></item></value></item>'
            . '<item><key xsi:type="xsd:string">Response</key><value xsi:type="ns2:Map">'
            . '<item><key xsi:type="xsd:string">KasFloodDelay</key><value xsi:type="xsd:float">' . $flood . '</value></item>'
            . '<item><key xsi:type="xsd:string">ReturnInfo</key>' . $returnInfo . '</item>'
            . '<item><key xsi:type="xsd:string">ReturnString</key><value xsi:type="xsd:string">TRUE</value></item>'
            . '</value></item></return></ns1:KasApiResponse></SOAP-ENV:Body></SOAP-ENV:Envelope>';
    }

    private const RECORDS = '<value SOAP-ENC:arrayType="ns2:Map[2]" xsi:type="SOAP-ENC:Array">'
        . '<item xsi:type="ns2:Map"><item><key xsi:type="xsd:string">record_name</key><value xsi:type="xsd:string">x1.lan</value></item><item><key xsi:type="xsd:string">record_type</key><value xsi:type="xsd:string">A</value></item><item><key xsi:type="xsd:string">record_data</key><value xsi:type="xsd:string">192.168.1.116</value></item><item><key xsi:type="xsd:string">record_id</key><value xsi:type="xsd:string">42</value></item></item>'
        . '<item xsi:type="ns2:Map"><item><key xsi:type="xsd:string">record_name</key><value xsi:type="xsd:string">www</value></item><item><key xsi:type="xsd:string">record_type</key><value xsi:type="xsd:string">CNAME</value></item></item>'
        . '</value>';

    private const FAULT = '<?xml version="1.0" encoding="UTF-8"?><SOAP-ENV:Envelope xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/"><SOAP-ENV:Body><SOAP-ENV:Fault><faultcode>SOAP-ENV:Server</faultcode><faultstring>record_id_not_found</faultstring></SOAP-ENV:Fault></SOAP-ENV:Body></SOAP-ENV:Envelope>';

    /** @param list<array{int, string}> $answers */
    private static function http(array $answers): RecordedHttp
    {
        return new RecordedHttp($answers);
    }

    /**
     * the JSON inside a request's <Params>
     * @return array<string, mixed>
     */
    private static function params(string $body): array
    {
        self::assertSame(1, preg_match('#<Params xsi:type="xsd:string">(.*)</Params>#s', $body, $m));
        $params = json_decode(html_entity_decode($m[1] ?? '', ENT_XML1 | ENT_QUOTES, 'UTF-8'), true);
        self::assertIsArray($params);
        return $params;
    }

    public function testSignsInOnceThenCallsTheActionWithItsParams(): void
    {
        $http = self::http([[200, self::AUTH_ANSWER], [200, self::apiAnswer(self::RECORDS, '0')], [200, self::apiAnswer('<value xsi:type="xsd:string">TRUE</value>', '0')]]);
        $kas = new SoapKasApi('w0123456', 'pa"ss<word>', $http);

        $records = $kas->call('get_dns_settings', ['zone_host' => 'scutoons.com.']);
        self::assertSame([
            ['record_name' => 'x1.lan', 'record_type' => 'A', 'record_data' => '192.168.1.116', 'record_id' => '42'],
            ['record_name' => 'www', 'record_type' => 'CNAME'],
        ], $records);
        self::assertSame('TRUE', $kas->call('update_dns_settings', ['record_id' => '42', 'record_data' => '192.168.1.117']));

        self::assertCount(3, $http->requests, 'one KasAuth, two KasApi');
        [$auth, $get, $update] = $http->requests;
        self::assertSame(SoapKasApi::AUTH_URL, $auth['url']);
        self::assertSame('"urn:xmethodsKasApiAuthentication#KasAuth"', $auth['headers']['SOAPAction']);
        self::assertSame(['kas_login' => 'w0123456', 'kas_auth_type' => 'plain', 'kas_auth_data' => 'pa"ss<word>', 'session_lifetime' => 600, 'session_update_lifetime' => 'Y'], self::params($auth['body']));
        self::assertStringNotContainsString('pa"ss<word>', $auth['body'], 'the password is XML-escaped');

        self::assertSame(SoapKasApi::API_URL, $get['url']);
        self::assertSame('"urn:xmethodsKasApi#KasApi"', $get['headers']['SOAPAction']);
        self::assertSame(['kas_login' => 'w0123456', 'kas_auth_type' => 'session', 'kas_auth_data' => 'session-token-1', 'kas_action' => 'get_dns_settings',
            'KasRequestParams' => ['zone_host' => 'scutoons.com.']], self::params($get['body']));
        self::assertSame('update_dns_settings', self::params($update['body'])['kas_action']);
        self::assertStringNotContainsString('pa"ss', $get['body'], 'the password goes only to KasAuth');
    }

    public function testWaitsOutTheFloodDelayBeforeTheNextCall(): void
    {
        $http = self::http([[200, self::AUTH_ANSWER], [200, self::apiAnswer(self::RECORDS, '2')], [200, self::apiAnswer(self::RECORDS, '0')]]);
        /** @var ArrayObject<int, float> $waits */
        $waits = new ArrayObject();
        $kas = new SoapKasApi('w0', 'pw', $http, static function (float $s) use ($waits): void { $waits->append($s); });
        $kas->call('get_dns_settings', ['zone_host' => 'scutoons.com.']);
        self::assertCount(0, $waits, 'nothing to wait for before the first call');
        $kas->call('get_dns_settings', ['zone_host' => 'scutoons.com.']);
        self::assertCount(1, $waits);
        self::assertGreaterThan(1.5, $waits[0]);
        self::assertLessThanOrEqual(2.0, $waits[0]);
    }

    public function testAFaultIsAKasFault(): void
    {
        $kas = new SoapKasApi('w0', 'pw', self::http([[200, self::AUTH_ANSWER], [500, self::FAULT]]));
        $this->expectException(KasFault::class);
        $this->expectExceptionMessage('record_id_not_found');
        $kas->call('update_dns_settings', ['record_id' => '9']);
    }

    public function testNoCredentialsNoCall(): void
    {
        $http = self::http([]);
        $kas = new SoapKasApi('', '', $http);
        try {
            $kas->call('get_dns_settings');
            self::fail('a KasFault');
        } catch (KasFault $e) {
            self::assertStringContainsString('KAS_LOGIN', $e->getMessage());
        }
        self::assertSame([], $http->requests);
    }

    public function testParsesScalarsAndRejectsNonXml(): void
    {
        $answer = SoapKasApi::parse(self::apiAnswer('<value xsi:type="xsd:string">101</value>'));
        self::assertIsArray($answer);
        self::assertSame(['KasFloodDelay' => '0.5', 'ReturnInfo' => '101', 'ReturnString' => 'TRUE'], $answer['Response'], 'a new record id');
        self::assertNull(SoapKasApi::parse(''));
        $this->expectException(KasFault::class);
        SoapKasApi::parse('<html>Bad gateway');
    }
}

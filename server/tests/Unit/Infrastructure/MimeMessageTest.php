<?php
declare(strict_types=1);

namespace Scutoons\Tests\Infrastructure;

use PHPUnit\Framework\TestCase;
use Scutoons\Infrastructure\Mail\MimeMessage;

/** The MIME reader of the Embassy inbox sync: headers, transfer encodings, charsets, multipart, HTML-only mails */
final class MimeMessageTest extends TestCase
{
    private static function mail(string $headers, string $body): string
    {
        return str_replace("\n", "\r\n", trim($headers)) . "\r\n\r\n" . str_replace("\n", "\r\n", $body);
    }

    public function testHeadersAreUnfoldedAndEncodedWordsDecoded(): void
    {
        $mail = MimeMessage::parse(self::mail(<<<'H'
            From: =?ISO-8859-1?Q?J=FCrgen_M=FCller?= <Juergen.Mueller@Example.AT>
            Subject: =?UTF-8?Q?Gr=C3=BC=C3=9Fe_aus?=
             =?UTF-8?B?IFdpZW4=?= und mehr
            Message-ID: <abc.123@mail.example.at>
            In-Reply-To: <0192-ours@scutoons.com>
            References: <first@scutoons.com>
              <0192-ours@scutoons.com>
            X-Empty:
            H, 'Hallo'));
        self::assertSame('Grüße aus Wien und mehr', $mail->header('Subject'));
        self::assertSame('Jürgen Müller <Juergen.Mueller@Example.AT>', $mail->header('from'));
        self::assertSame('juergen.mueller@example.at', $mail->address('From'));
        self::assertSame(['abc.123@mail.example.at'], $mail->ids('Message-ID'));
        self::assertSame(['first@scutoons.com', '0192-ours@scutoons.com'], $mail->ids('References'));
        self::assertSame(['0192-ours@scutoons.com'], $mail->ids('In-Reply-To'));
        self::assertSame('', $mail->header('X-Missing'));
        self::assertTrue($mail->has('X-Empty'));
        self::assertSame('bare@example.com', MimeMessage::parse(self::mail('From: bare@Example.com', ''))->address('From'));
        self::assertSame('', MimeMessage::parse(self::mail('From: Nobody', ''))->address('From'));
    }

    public function testQuotedPrintableAndCharsets(): void
    {
        $latin1 = MimeMessage::parse(self::mail(<<<'H'
            Content-Type: text/plain; charset="iso-8859-1"
            Content-Transfer-Encoding: quoted-printable
            H, "Liebe Gr=FC=DFe, ein sehr langer Satz, der weich umbrochen=\nwird \x80 =80\n"));
        self::assertSame("Liebe Grüße, ein sehr langer Satz, der weich umbrochenwird € €", $latin1->text());

        $utf8 = MimeMessage::parse(self::mail("Content-Type: text/plain; charset=utf-8\nContent-Transfer-Encoding: base64",
            chunk_split(base64_encode("Danke – wir melden uns!\n\n\n\nBis bald  \n"), 20)));
        self::assertSame("Danke – wir melden uns!\n\nBis bald", $utf8->text());

        $unknown = MimeMessage::parse(self::mail('Content-Type: text/plain; charset=x-made-up', "caf\xE9 ok"));
        self::assertSame("caf? ok", $unknown->text()); // unknown charset: kept, invalid bytes replaced
        self::assertSame('plain 7bit', MimeMessage::parse("Subject: no type\r\n\r\nplain 7bit")->text());
    }

    public function testMultipartAlternativePrefersThePlainPart(): void
    {
        $mail = MimeMessage::parse(self::mail(<<<'H'
            Content-Type: multipart/alternative; boundary="b1"
            H, <<<'B'
            This is a multi-part message.
            --b1
            Content-Type: text/html; charset=utf-8

            <p>HTML version</p>
            --b1
            Content-Type: text/plain; charset=utf-8
            Content-Transfer-Encoding: quoted-printable

            Plain version =E2=9C=93
            --b1--
            B));
        self::assertSame('Plain version ✓', $mail->text());
    }

    public function testNestedMultipartSkipsAttachments(): void
    {
        $mail = MimeMessage::parse(self::mail('Content-Type: multipart/mixed; boundary=outer', <<<'B'
            --outer
            Content-Type: text/plain; name="notes.txt"
            Content-Disposition: attachment; filename="notes.txt"

            attachment text
            --outer
            Content-Type: multipart/alternative; boundary="inner"

            --inner
            Content-Type: text/plain; charset=windows-1252
            Content-Transfer-Encoding: quoted-printable

            Sch=F6n =96 bis Freitag
            --inner
            Content-Type: text/html

            <b>ignored</b>
            --inner--
            --outer
            Content-Type: application/pdf; name="epk.pdf"
            Content-Transfer-Encoding: base64

            JVBERi0=
            --outer--
            B));
        self::assertSame('Schön – bis Freitag', $mail->text());
    }

    public function testHtmlOnlyMailBecomesText(): void
    {
        $html = '<html><head><title>x</title><style>p { color: red }</style></head><body><div>Hi there,</div>'
            . '<p>we&#39;d love to &amp; can<br>book you.</p><ul><li>Friday</li><li>Saturday</li></ul><script>alert(1)</script>'
            . '<p>Cheers&nbsp;Anna</p></body></html>';
        $mail = MimeMessage::parse(self::mail("Content-Type: text/html; charset=UTF-8\nContent-Transfer-Encoding: base64", base64_encode($html)));
        self::assertSame("Hi there,\n\nwe'd love to & can\nbook you.\n\n- Friday\n- Saturday\n\nCheers Anna", $mail->text());
        self::assertSame('', MimeMessage::parse(self::mail('Content-Type: image/png', 'xxx'))->text());
    }
}

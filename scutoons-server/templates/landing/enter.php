<?php
/**
 * No code in the URL: enter one by hand (the form sends ?c=, the parameter printed on every QR code).
 * @var callable $e  @var array<string, string> $t
 */
?>
<div class="panel">
    <h1><?= $e($t['members']) ?></h1>
    <p><?= $e($t['members_hint']) ?></p>
    <form method="GET">
        <label class="field-label" for="c"><?= $e($t['access_code']) ?></label>
        <input type="text" id="c" name="c" required maxlength="8" autocomplete="off" placeholder="<?= $e($t['access_code']) ?>">
        <button type="submit"><?= $e($t['enter']) ?></button>
    </form>
</div>
<div class="faq">
    <h2><?= $e($t['faq_q']) ?></h2>
    <p><?= $e($t['faq_a']) ?> <a href="https://mandymozart.bandcamp.com/merch" target="_blank" rel="noopener"><?= $e($t['shop']) ?></a></p>
</div>

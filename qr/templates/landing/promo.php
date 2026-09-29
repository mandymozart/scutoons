<?php
/**
 * A promo code (signup, nothing to own): no item preview; Continue (device access in the viewer) + Create account.
 * @var callable $e  @var array<string, string> $t  @var string $continueUrl  @var string $accountUrl
 */
?>
<div class="panel" id="promo"><h1><?= $e($t['promo']) ?></h1><p><?= $e($t['promo_hint']) ?></p>
<div class="actions">
<a class="btn" id="continue" href="<?= $e($continueUrl) ?>"><?= $e($t['continue']) ?></a><p class="caption"><?= $e($t['continue_hint']) ?></p>
<a class="btn btn-secondary" id="create-account" href="<?= $e($accountUrl) ?>"><?= $e($t['create_account']) ?></a><p class="caption"><?= $e($t['create_account_hint']) ?></p>
</div></div>

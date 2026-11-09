# The two-deck demo (#442, events between linked decks) on this PC: a local relay, the deck `pc` playing rig:catpope-analog
# (its 16-step sequencer sends notes), and a second deck playing rig:far-listener (rigs/far_listener.toml), whose far world
# node names `/deck:pc/scene:demo/character:catpope/analog:seq.notes` and plays those notes on its own analog synth.
# The first deck is this PC's deck (ASIO on the Fireface, deck.toml: name = "pc", join = "deck"); the second runs windowed on
# WASAPI so both can sound. The console of the first prints  far events: <client> asks for /deck:pc/...  once the second has
# asked; the second's being pulses with the notes it gets.
# Needs: target\asio\debug\deck.exe (cargo build -p deck-native --features modules,asio --target-dir target/asio),
# target\debug\relay.exe (cargo build -p relay), the module wasm in target\hear-modules (see run-catpope-analog.ps1).
# Optional: -Frames N writes a screenshot of each deck after N frames (target\far-demo-*.png) and quits.
# (ASCII only: Windows PowerShell 5.1 reads a script without a BOM as ANSI)
param([int]$Frames = 0)
$deck = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$exe = Join-Path $deck 'target\asio\debug\deck.exe'
$relay = Join-Path $deck 'target\debug\relay.exe'
foreach ($f in @($exe, $relay, (Join-Path $deck 'target\hear-modules\analog_module.wasm'))) {
    if (-not (Test-Path $f)) { Write-Error "missing $f - see the header of this script"; exit 1 }
}
$env:DECK_MODULES = Join-Path $deck 'target\hear-modules'
$env:RUST_LOG = 'info'
$env:RELAY_ADDR = '127.0.0.1:7742'
$url = 'ws://127.0.0.1:7742/live/FAR'
$relayProc = Start-Process -FilePath $relay -PassThru -WindowStyle Minimized
Start-Sleep -Seconds 1
$check = @()
if ($Frames -gt 0) { $check = @('--frames', "$Frames") }
$a = @('--scene', 'rig:catpope-analog', '--windowed', '--relay', $url) + $check
$b = @('--scene', 'rig:far-listener', '--windowed', '--audio-host', 'wasapi', '--relay', $url) + $check
if ($Frames -gt 0) {
    $a += @('--screenshot', (Join-Path $deck 'target\far-demo-pc.png'))
    $b += @('--screenshot', (Join-Path $deck 'target\far-demo-listener.png'))
}
$pc = Start-Process -FilePath $exe -ArgumentList $a -PassThru -RedirectStandardOutput (Join-Path $deck 'target\far-demo-pc.log')
Start-Sleep -Seconds 2
$listener = Start-Process -FilePath $exe -ArgumentList $b -PassThru -RedirectStandardOutput (Join-Path $deck 'target\far-demo-listener.log')
Write-Host "relay $($relayProc.Id) | pc deck $($pc.Id) | listener $($listener.Id) - logs in target\far-demo-*.log; Ctrl+Q in a deck window ends it"
$pc.WaitForExit()
$listener.WaitForExit()
Stop-Process -Id $relayProc.Id -Force -ErrorAction SilentlyContinue

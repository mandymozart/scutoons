#!/usr/bin/env bash
# #438: the relay under real packet loss – Linux `tc netem` on lo: 5 ms each way (RTT 10 ms), loss per packet, real TCP
# and real UDP. Needs root or CAP_NET_ADMIN, in a throw-away container (the qdisc changes lo for everything in that net
# namespace). Results: one JSON line per run (doors_bench) or the bench's own text, plus the kernel's TCP counters.
#
#   bench=/target/release/examples/bench doors=/target/release/examples/doors_bench out=/results secs=30 runs=3 \
#     bash deck/relay/netem/run.sh baseline|doors|mixed|smoke
set -u
bench=${bench:-target/release/examples/bench}
doors=${doors:-target/release/examples/doors_bench}
out=${out:-.}
secs=${secs:-30}
runs=${runs:-3}
mkdir -p "$out"

path() {
  tc qdisc del dev lo root 2>/dev/null
  if [ "$1" = 0 ]; then tc qdisc add dev lo root netem delay 5ms; else tc qdisc add dev lo root netem delay 5ms loss "$1%"; fi
}
trap 'tc qdisc del dev lo root 2>/dev/null' EXIT

counters() { nstat -az TcpRetransSegs TcpExtTCPTimeouts TcpExtTCPLostRetransmit TcpExtTCPLossProbes TcpExtTCPSpuriousRtxHostQueues 2>/dev/null | awk 'NR>1 && $1!~/^#/{printf "%s=%s ", $1, $2}'; echo; }

baseline() {
  for loss in 0 1 5; do
    path "$loss"
    for i in $(seq "$runs"); do
      nstat -n
      "$bench" --decks 20 --listeners 100 --seconds "$secs" >> "$out/baseline.$loss.txt" 2>&1
      { echo "run $i loss $loss: $(counters)"; } >> "$out/baseline.$loss.txt"
    done
  done
}

one() { # one <loss> <resend> <label> <doors args …>
  local loss=$1 resend=$2 label=$3; shift 3
  path "$loss"
  for i in $(seq "$runs"); do
    nstat -n
    "$doors" --seconds "$secs" --resend "$resend" --label "$label loss$loss" "$@" >> "$out/doors.jsonl" 2>> "$out/doors.err"
    echo "$label loss$loss run $i: $(counters)" >> "$out/doors.counters.txt"
  done
}

doors_matrix() {
  for loss in 0 1 5; do
    for via in ws quinn wt renet2; do one "$loss" 40 "$via" --via "$via"; done
  done
}

mixed() {
  for loss in 1 5; do
    # every door at once, one session
    one "$loss" 40 mixed --decks-via renet2 --conductor-via wt --phones-via ws
    # renet2's fixed retransmit timer: its default class (300 ms) and shorter
    for resend in 20 300; do one "$loss" "$resend" "renet2-resend$resend" --via renet2; done
    # quinn's ack delay, so a lost cue on an idle stream waits a shorter probe timeout
    one "$loss" 40 quinn-ack5 --via quinn --ack-delay 5
  done
}

smoke() {
  secs=5 runs=1
  path 1
  for via in ws quinn wt renet2; do
    "$doors" --seconds 5 --label "smoke $via" --via "$via" 2>&1 | cut -c1-900
  done
}

case "${1:-}" in
  baseline) baseline ;;
  doors) doors_matrix ;;
  mixed) mixed ;;
  smoke) smoke ;;
  *) echo "usage: run.sh baseline|doors|mixed|smoke" >&2; exit 2 ;;
esac

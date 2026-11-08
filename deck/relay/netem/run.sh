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

one() { # one <loss> <label> <doors args …>
  local loss=$1 label=$2; shift 2
  path "$loss"
  for i in $(seq "$runs"); do
    nstat -n
    "$doors" --seconds "$secs" --label "$label loss$loss" "$@" >> "$out/doors.jsonl" 2>> "$out/doors.err"
    echo "$label loss$loss run $i: $(counters)" >> "$out/doors.counters.txt"
  done
}

doors_matrix() {
  for loss in 0 1 5; do
    for via in ws wt; do one "$loss" "$via" --via "$via"; done
  done
}

# both doors at once, one session
mixed() {
  for loss in 1 5; do one "$loss" mixed --decks-via wt,ws --conductor-via wt --phones-via wt,ws; done
}

smoke() {
  secs=5 runs=1
  path 1
  for via in ws wt; do
    "$doors" --seconds 5 --label "smoke $via" --via "$via" 2>&1 | cut -c1-900
  done
}

# The relay alone in its own process (RSS, CPU), the clients in another; the egress from iptables counters on the doors.
cost_one() { # cost_one <loss> <label> <decks> <phones> <client args …>
  local loss=$1 label=$2 decks=$3 phones=$4; shift 4
  path "$loss"
  iptables -F OUTPUT
  iptables -A OUTPUT -p tcp --sport 7070 -j ACCEPT
  iptables -A OUTPUT -p udp -m multiport --sports 7071,7072,7073 -j ACCEPT
  for i in $(seq "$runs"); do
    rm -f /tmp/438/ready
    "$doors" --serve --label "$label loss$loss ${decks}x$phones" >> "$out/cost.jsonl" 2>> "$out/cost.err" &
    local server=$!
    while [ ! -e /tmp/438/ready ]; do sleep 0.05; done
    "$doors" --connect --count-egress --seconds "$secs" --decks "$decks" --listeners "$phones" --label "$label loss$loss ${decks}x$phones" "$@" >> "$out/cost.jsonl" 2>> "$out/cost.err"
    wait "$server"
  done
  iptables -F OUTPUT
}

cost() {
  for size in "20 100" "50 500"; do
    read -r decks phones <<< "$size"
    for via in ws wt; do cost_one 1 "alone-$via" "$decks" "$phones" --via "$via"; done
    cost_one 1 mixed "$decks" "$phones" --decks-via wt,ws --phones-via wt,ws --conductor-via wt
  done
}

# what fits one datagram; a shared 2 Mbit path (does the adaptive rate see it?)
limits() {
  one 1 keys60-wt --via wt --keys 60
  tc qdisc del dev lo root 2>/dev/null
  for via in ws wt; do
    tc qdisc del dev lo root 2>/dev/null
    tc qdisc add dev lo root netem delay 5ms rate 2mbit limit 200
    for i in $(seq "$runs"); do "$doors" --seconds "$secs" --via "$via" --label "bottleneck-$via 2mbit" >> "$out/doors.jsonl" 2>> "$out/doors.err"; done
  done
}

# UDP blocked outright: the WebSocket rung is the worst case
udp_blocked() {
  path 1
  iptables -I INPUT -p udp -j DROP
  for i in $(seq "$runs"); do "$doors" --seconds "$secs" --via ws --label "udp-blocked ws loss1" >> "$out/doors.jsonl" 2>> "$out/doors.err"; done
  iptables -D INPUT -p udp -j DROP
}

case "${1:-}" in
  cost) cost ;;
  limits) limits ;;
  udp-blocked) udp_blocked ;;
  baseline) baseline ;;
  doors) doors_matrix ;;
  mixed) mixed ;;
  smoke) smoke ;;
  *) echo "usage: run.sh baseline|doors|mixed|cost|limits|udp-blocked|smoke" >&2; exit 2 ;;
esac

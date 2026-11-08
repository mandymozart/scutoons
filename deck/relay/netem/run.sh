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
    for via in ws wt renet2; do cost_one 1 "alone-$via" "$decks" "$phones" --via "$via"; done
    cost_one 1 mixed "$decks" "$phones" --decks-via renet2,wt,ws --phones-via renet2,wt,ws --conductor-via wt
    # the target layout: native decks on RUDP, browser decks on QUIC, the audience's browsers on QUIC
    cost_one 1 target "$decks" "$phones" --decks-via renet2,wt --phones-via wt --conductor-via wt
  done
}

# what fits one datagram; a shared 2 Mbit path (does the adaptive rate see it?)
limits() {
  for via in quinn wt renet2; do one 1 40 "keys60-$via" --via "$via" --keys 60; done
  tc qdisc del dev lo root 2>/dev/null
  for via in ws wt renet2; do
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

ladder() {
  path 1
  for stack in native browser; do
    for climb in race turn; do
      # nothing blocked
      "$doors" --ladder-join "$stack" --climb "$climb" --joins 40 --label "none" >> "$out/ladder.jsonl" 2>> "$out/ladder.err"
      # the RUDP port filtered, UDP 443 passing (the case for the middle rung)
      [ "$stack" = native ] && { iptables -I INPUT -p udp --dport 7073 -j DROP
        "$doors" --ladder-join "$stack" --climb "$climb" --joins 40 --label "rudp-blocked" >> "$out/ladder.jsonl" 2>> "$out/ladder.err"
        iptables -D INPUT -p udp --dport 7073 -j DROP; }
      # all UDP blocked
      iptables -I INPUT -p udp -j DROP
      "$doors" --ladder-join "$stack" --climb "$climb" --joins 40 --label "udp-blocked" >> "$out/ladder.jsonl" 2>> "$out/ladder.err"
      iptables -D INPUT -p udp -j DROP
    done
  done
  for i in 1 2 3; do "$doors" --ladder-show --seconds 12 --block-at 8 --label "udp-dies-midshow loss1" >> "$out/ladder.jsonl" 2>> "$out/ladder.err"; done
}

case "${1:-}" in
  cost) cost ;;
  limits) limits ;;
  udp-blocked) udp_blocked ;;
  ladder) ladder ;;
  baseline) baseline ;;
  doors) doors_matrix ;;
  mixed) mixed ;;
  smoke) smoke ;;
  *) echo "usage: run.sh baseline|doors|mixed|cost|limits|udp-blocked|ladder|smoke" >&2; exit 2 ;;
esac

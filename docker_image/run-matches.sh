#!/bin/bash
set -u

cd /filler/solution || exit $?
CARGO_TARGET_DIR=/tmp/filler-target cargo test --release || exit $?
CARGO_TARGET_DIR=/tmp/filler-target cargo build --release || exit $?

cd /filler || exit $?
bot=/tmp/filler-target/release/filler
match_failed=0

for opponent in bender h2_d2 wall_e terminator; do
    echo "=== player 1: filler vs $opponent (map01, seed 42) ==="
    ./linux_game_engine -f maps/map01 -s 42 -p1 "$bot" -p2 "linux_robots/$opponent"
    if [ "$?" -ne 0 ]; then match_failed=1; fi

    echo "=== player 2: $opponent vs filler (map01, seed 42) ==="
    ./linux_game_engine -f maps/map01 -s 42 -p1 "linux_robots/$opponent" -p2 "$bot"
    if [ "$?" -ne 0 ]; then match_failed=1; fi
done

exit "$match_failed"

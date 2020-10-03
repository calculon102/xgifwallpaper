#!/bin/sh

# Run from tests-directory

function run_debug {
	set -x
	../../target/release/xgifwallpaper $VERBOSE -b $1 -s $2 $3 &
	{ set +x; } 2> /dev/null

	last_pid=$!
	sleep $SLEEP_TIME
	kill -INT $last_pid
}

BACKGROUND="white"
FILE="sample-1x1.gif"
POSITION="FILL"
SLEEP_TIME="3s"
VERBOSE=""

cargo build --release

run_debug $BACKGROUND $POSITION "sample-1x1.gif"
run_debug $BACKGROUND $POSITION "sample-2x1.gif"
run_debug $BACKGROUND $POSITION "sample-1x2.gif"
run_debug $BACKGROUND $POSITION "sample-1x1-one-frame.gif"

POSITION="MAX"
run_debug $BACKGROUND $POSITION "sample-1x1.gif"
run_debug $BACKGROUND $POSITION "sample-2x1.gif"
run_debug $BACKGROUND $POSITION "sample-1x2.gif"
run_debug $BACKGROUND $POSITION "sample-1x1-one-frame.gif"

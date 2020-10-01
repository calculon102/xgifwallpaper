#!/bin/sh

BACKGROUND="white"
POSITION="FILL"
FILE="sample-1x1.gif"
VERBOSE=""

function run_debug {
	set -x
	../target/debug/xgifwallpaper $VERBOSE -b $1 -p $2 $3 &
	set +x

	last_pid=$!
	sleep 10s
	kill -KILL $last_pid
}

cargo build

run_debug $BACKGROUND $POSITION "sample-1x1.gif"
run_debug $BACKGROUND $POSITION "sample-2x1.gif"
run_debug $BACKGROUND $POSITION "sample-1x2.gif"
run_debug $BACKGROUND $POSITION "sample-1x1-not-animated.gif"

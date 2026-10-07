#!/usr/bin/env bash
# The competition entry point: domain.hddl problem.hddl plan-file.
#
#   parse -> ground -> generate -> compile (this is the search) -> convert to the IPC format
#
# TWINE is the crate, with its library already built into $TWINE/target, and PARSER/GROUNDER the
# pandaPI tools. The image is read-only at run time, so the crate is copied to a scratch
# directory and the problem is generated and compiled there.

set -uo pipefail

if [ $# -ne 3 ]; then
    echo "usage: $0 domain.hddl problem.hddl plan-file" >&2
    exit 2
fi

DOMAIN="$1"
PROBLEM="$2"
PLAN="$3"
TWINE="${TWINE:-$(cd "$(dirname "$0")/.." && pwd)}"
PARSER="${PARSER:-pandaPIparser}"
GROUNDER="${GROUNDER:-pandaPIgrounder}"
# A two-phase portfolio. Deepening from 1 finds the plans of easy problems fastest, so it gets
# a short slice; a problem it has not solved by then is searched again from scratch with the
# default schedule, whose generous first bound lets depth-first search dive straight down a plan.
FIRST_FLAGS="${FIRST_FLAGS:---rounds 12}"
FIRST_SECONDS="${FIRST_SECONDS:-30}"
THEN_FLAGS="${THEN_FLAGS:-}"

work="$(mktemp -d -t twine-XXXXXX)"
trap 'rm -rf "$work"' EXIT

echo "== parse (${SECONDS}s) ==" >&2
"$PARSER" "$DOMAIN" "$PROBLEM" "$work/problem.htn" >&2 || exit 1

echo "== ground (${SECONDS}s) ==" >&2
"$GROUNDER" -q "$work/problem.htn" "$work/problem.sas" >&2 || exit 1

# -a keeps the timestamps, so cargo sees the prebuilt library as fresh and only builds weave.
cp -a "$TWINE/Cargo.toml" "$TWINE/Cargo.lock" "$TWINE/src" "$TWINE/.cargo" "$TWINE/target" "$work/"
export CARGO_HOME="$work/cargo-home"
export CARGO_INCREMENTAL=0

# Generates the problem with the given twine-gen options and compiles it, which is the search
# (.cargo/config.toml selects the trait solver it runs in). With a time limit, the whole process
# group goes when it runs out, rustc included.
weave() {
    local limit="$1"
    shift
    echo "== generate: $* (${SECONDS}s) ==" >&2
    "$work/target/release/twine-gen" "$work/problem.sas" --out "$work/src/problem.rs" "$@" >&2 \
        || return 1
    echo "== search (rustc) (${SECONDS}s) ==" >&2
    if [ -z "$limit" ]; then
        (cd "$work" && cargo build --offline --quiet --bin weave) >&2
        return
    fi
    setsid bash -c 'cd "$1" && exec cargo build --offline --quiet --bin weave' _ "$work" >&2 &
    local pid=$!
    local waited=0
    while kill -0 "$pid" 2>/dev/null; do
        if [ "$waited" -ge "$limit" ]; then
            kill -KILL -- "-$pid" 2>/dev/null
            wait "$pid" 2>/dev/null
            echo "== no plan within ${limit}s (${SECONDS}s) ==" >&2
            return 1
        fi
        sleep 1
        waited=$((waited + 1))
    done
    wait "$pid"
}

# shellcheck disable=SC2086
weave "$FIRST_SECONDS" $FIRST_FLAGS || weave "" $THEN_FLAGS || exit 1

echo "== convert (${SECONDS}s) ==" >&2
"$work/target/debug/weave" --save "$work/plan.raw" > /dev/null || exit 1
"$PARSER" -c "$work/plan.raw" "$PLAN" >&2 || exit 1
echo "== plan written to $PLAN (${SECONDS}s) ==" >&2

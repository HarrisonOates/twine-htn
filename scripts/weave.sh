#!/usr/bin/env bash
# The full pipeline: parse -> ground -> generate -> compile (this is the search) -> verify.
#
#   PARSER=... GROUNDER=... scripts/weave.sh domain.hddl problem.hddl [twine-gen options...]
#
# PARSER and GROUNDER default to pandaPIparser and pandaPIgrounder on PATH. Leaves
# src/problem.rs pointing at the problem it just planned.

set -euo pipefail

if [ $# -lt 2 ]; then
    sed -n '2,7p' "$0" >&2
    exit 2
fi

DOMAIN="$1"
PROBLEM="$2"
shift 2
PARSER="${PARSER:-pandaPIparser}"
GROUNDER="${GROUNDER:-pandaPIgrounder}"

for tool in "$PARSER" "$GROUNDER"; do
    if ! command -v "$tool" >/dev/null && [ ! -x "$tool" ]; then
        echo "missing: $tool — build it from https://github.com/panda-planner-dev" >&2
        exit 1
    fi
done

work="$(mktemp -d -t twine-XXXXXX)"
[ "${KEEP:-0}" = "1" ] || trap 'rm -rf "$work"' EXIT

echo "== parse ==" >&2
"$PARSER" "$DOMAIN" "$PROBLEM" "$work/problem.htn" >&2

echo "== ground ==" >&2
"$GROUNDER" "$work/problem.htn" "$work/problem.sas" >&2

echo "== generate ==" >&2
cargo weave-gen "$work/problem.sas" "$@" >&2

echo "== search (rustc) ==" >&2
cargo weave --save "$work/plan.raw"

echo "== convert ==" >&2
"$PARSER" -c "$work/plan.raw" "$work/plan.converted" >&2

echo "== verify ==" >&2
if "$PARSER" -v "$DOMAIN" "$PROBLEM" "$work/plan.converted" >&2; then
    echo "== PLAN VERIFIED ==" >&2
    [ "${KEEP:-0}" = "1" ] && echo "work dir: $work" >&2
    exit 0
else
    echo "== PLAN REJECTED BY VERIFIER ==" >&2
    echo "work dir kept: $work" >&2
    trap - EXIT
    exit 1
fi

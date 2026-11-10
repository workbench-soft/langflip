#!/bin/sh
# Ties Release to the Checks run of the same commit, so no job waits on a runner for the other.
#   release-gate.sh checks    outputs checks=success, failure, pending or none, once
#   release-gate.sh build     on workflow_run of Checks: outputs run=<id> when the build of the commit
#                             finished and nothing in its own run publishes it
#   release-gate.sh cancel    cancels the unfinished Release runs of the commit
# Needs GH_TOKEN, SHA, CHECKS_WORKFLOW (checks.yml) and RELEASE_WORKFLOW (release.yml).
# Git Bash on the runner has no jq, so each query goes through gh --jq.
set -eu
repo=$GITHUB_REPOSITORY
# A TLS timeout or a 5xx from GitHub failed a whole release, so each call gets 4 tries, 10 s, 20 s, 40 s apart.
api() {
  wait=10
  for try in 1 2 3 4; do
    gh api "$@" && return 0
    [ "$try" = 4 ] && return 1
    echo "GitHub API call failed, try $((try + 1)) of 4 in ${wait}s." >&2
    sleep "$wait"
    wait=$((wait * 2))
  done
}
# The runs of SHA on main from a push or a run by hand, newest first, then a jq filter over them.
runs() { # workflow filter
  api "repos/$repo/actions/workflows/$1/runs?head_sha=$SHA&branch=main&per_page=20" \
    --jq "[.workflow_runs[] | select(.event == \"push\" or .event == \"workflow_dispatch\")] | sort_by(-.id) | $2"
}
job() { # run name: status/conclusion of the job with that name, or of its first matrix leg
  api "repos/$repo/actions/runs/$1/jobs" \
    --jq "[.jobs[] | select(.name == \"$2\" or (.name | startswith(\"$2 (\")))][0] | .status + \"/\" + (.conclusion // \"-\")"
}

case $1 in
  checks)
    result=$(runs "$CHECKS_WORKFLOW" '.[0] | if . == null then "none" elif .status != "completed" then "pending" else .conclusion end')
    echo "Checks of $SHA: $result"
    # A run by hand on a commit whose push Checks skipped. Run Checks by hand, and their end publishes it.
    [ "$result" != none ] || echo "No Checks run for $SHA. Start Checks by hand to publish this build."
    echo "checks=$result" >>"$GITHUB_OUTPUT"
    ;;
  build)
    run=$(runs "$RELEASE_WORKFLOW" '.[0].id // empty')
    if [ -z "$run" ]; then
      echo "No Release run built $SHA, so there is nothing to publish."
      exit 0
    fi
    # The build looks up Checks in its last step. A build in that step finishes within seconds.
    for _ in 1 2 3 4 5 6 7 8 9 10 11 12; do
      build=$(job "$run" build)
      case $build in completed/*) break ;; esac
      sleep 10
    done
    case $build in
      completed/success) ;;
      completed/*)
        echo "The build of $SHA ended $build, so there is nothing to publish."
        exit 0
        ;;
      *)
        echo "The build of $SHA still runs. It sees the Checks result at its end and publishes then."
        exit 0
        ;;
    esac
    publish=$(job "$run" publish)
    if [ "$publish" != completed/skipped ]; then
      echo "Run $run publishes $SHA itself ($publish)."
      exit 0
    fi
    echo "run=$run" >>"$GITHUB_OUTPUT"
    ;;
  cancel)
    for run in $(runs "$RELEASE_WORKFLOW" '.[] | select(.status != "completed") | .id'); do
      echo "Checks of $SHA failed, so Release run $run stops."
      gh run cancel "$run" --repo "$repo" || true
    done
    ;;
  *)
    echo "usage: release-gate.sh checks|build|cancel" >&2
    exit 2
    ;;
esac

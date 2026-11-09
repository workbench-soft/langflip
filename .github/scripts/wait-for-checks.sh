#!/bin/sh
# Waits up to an hour for the checks workflow run of this commit on this branch.
# Release builds beside the checks, and only its publication waits here.
set -eu
workflow=${CHECKS_WORKFLOW:-checks.yml}
attempt=0
while [ "$attempt" -lt 360 ]; do
  result=$(gh api "repos/$GITHUB_REPOSITORY/actions/workflows/$workflow/runs?head_sha=$GITHUB_SHA&per_page=100" \
    --jq '[.workflow_runs[] | select(.head_branch == env.GITHUB_REF_NAME and (.event == "push" or .event == "workflow_dispatch"))] | max_by(.id) | if . == null then "missing" elif .status != "completed" then "pending" else .conclusion end')
  case $result in
    success) exit 0 ;;
    missing | pending) sleep 10 ;;
    *)
      echo "::error::Checks for $GITHUB_SHA ended with $result. Publication stopped."
      exit 1
      ;;
  esac
  attempt=$((attempt + 1))
done
echo "::error::Checks for $GITHUB_SHA did not finish within 1 hour. Publication stopped."
exit 1

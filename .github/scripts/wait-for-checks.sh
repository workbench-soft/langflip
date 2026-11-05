#!/usr/bin/env bash
# Builds start beside Checks; only publication waits for the matching commit.
set -euo pipefail
checks_workflow=${CHECKS_WORKFLOW:-checks.yml}
for ((attempt = 0; attempt < 360; attempt++)); do
  result=$(gh api "repos/$GITHUB_REPOSITORY/actions/workflows/$checks_workflow/runs?head_sha=$GITHUB_SHA&per_page=100" \
    --jq '[.workflow_runs[] | select(.head_branch == env.GITHUB_REF_NAME and (.event == "push" or .event == "workflow_dispatch"))] | max_by(.id) | if . == null then "missing" elif .status != "completed" then "pending" else .conclusion end')
  case "$result" in
    success) exit 0 ;;
    missing | pending) sleep 10 ;;
    *)
      echo "::error::Checks for $GITHUB_SHA ended with $result. Publication stopped."
      exit 1
      ;;
  esac
done
echo "::error::Checks for $GITHUB_SHA did not finish within 1 hour. Publication stopped."
exit 1

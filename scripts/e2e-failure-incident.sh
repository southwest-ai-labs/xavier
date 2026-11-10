#!/bin/bash
# scripts/e2e-failure-incident.sh
# Upsert a structured GitHub issue for Jules when an E2E test or build fails in GitHub Actions
# (or close it automatically when the same pipeline goes green again).
#
# Conforms to GitCore Jules Issues template v3.1 (gitcore-jules-issues / issue-creation-protocol).
#
# Required env:
#   CONCLUSION, HEAD_SHA, RUN_ID, RUN_URL, WORKFLOW_NAME
# Optional env:
#   JOB_NAME, RUNNER_OS, HEAD_BRANCH, REPRO_COMMAND, TARGET_FILES, ERROR_LOG, LABEL

set -euo pipefail

: "${CONCLUSION:?CONCLUSION required}"
: "${HEAD_SHA:?HEAD_SHA required}"
: "${RUN_ID:?RUN_ID required}"
: "${RUN_URL:?RUN_URL required}"
: "${WORKFLOW_NAME:?WORKFLOW_NAME required}"

JOB_NAME="${JOB_NAME:-$WORKFLOW_NAME}"
RUNNER_OS="${RUNNER_OS:-Linux}"
HEAD_BRANCH="${HEAD_BRANCH:-main}"
LABEL="${LABEL:-e2e-failure}"
REPRO_COMMAND="${REPRO_COMMAND:-cargo test --package xavier --lib --features ci-safe}"
TARGET_FILES="${TARGET_FILES:-src/**, panel-ui/**, tests/**}"
GITHUB_REPOSITORY="${GITHUB_REPOSITORY:-southwest-ai-labs/xavier}"
DRY_RUN="${DRY_RUN:-0}"
SHORT_SHA=${HEAD_SHA:0:7}

# Deterministic HTML marker for deduplication and reconciliation
# One open issue per workflow + job + OS
MARKER="<!-- e2e-failure-incident workflow=${WORKFLOW_NAME} job=${JOB_NAME} os=${RUNNER_OS} -->"
TITLE="[fix(ci-e2e)] ${JOB_NAME} failed on ${RUNNER_OS} (${WORKFLOW_NAME})"

# Ensure labels exist in the repository
gh label create "$LABEL" --color B60205 --description "E2E test failure in CI runner VM" 2>/dev/null || true
gh label create "jules" --color 5319E7 --description "Delegable to Jules autonomous agent" 2>/dev/null || true
gh label create "bug" --color D73A4A --description "Something isn't working" 2>/dev/null || true

find_open_issue() {
  local search_hits number body page=1 per_page=100 max_pages=30 chunk count open_issues="[]"

  # Fast search path by marker
  if search_hits=$(gh issue list --state open --limit 50 --search "$MARKER" --json number,body 2>/dev/null); then
    if [ -n "$search_hits" ] && [ "$search_hits" != "[]" ]; then
      while IFS= read -r number; do
        [ -n "$number" ] || continue
        body=$(echo "$search_hits" | jq -r --argjson n "$number" '.[] | select(.number == $n) | .body // empty') || return $?
        if [[ "$body" == *"$MARKER"* ]]; then
          echo "$number"
          return 0
        fi
      done < <(echo "$search_hits" | jq -r '.[].number')
    fi
  fi

  # Exhaustive fallback scan
  while :; do
    chunk=$(gh api "repos/${GITHUB_REPOSITORY}/issues?state=open&per_page=${per_page}&page=${page}") || return $?
    count=$(echo "$chunk" | jq 'length') || return $?
    [ "$count" -gt 0 ] || break
    while IFS= read -r number; do
      [ -n "$number" ] || continue
      body=$(echo "$chunk" | jq -r --argjson n "$number" '.[] | select(.pull_request == null and .number == $n) | .body // empty') || return $?
      if [[ "$body" == *"$MARKER"* ]]; then
        echo "$number"
        return 0
      fi
    done < <(echo "$chunk" | jq -r '.[] | select(.pull_request == null) | .number')
    [ "$count" -lt "$per_page" ] && break
    page=$((page + 1))
    [ "$page" -gt "$max_pages" ] && break
  done
  return 0
}

set +e
EXISTING=$(find_open_issue)
LOOKUP_RC=$?
set -e
if [ "$LOOKUP_RC" -ne 0 ]; then
  echo "e2e-failure-incident: issue lookup failed (rc=$LOOKUP_RC); aborting without mutations" >&2
  exit "$LOOKUP_RC"
fi

# Fetch failed log snippet if ERROR_LOG not explicitly provided
if [ -z "${ERROR_LOG:-}" ]; then
  ERROR_LOG=$(gh run view "$RUN_ID" --log-failed 2>&1 | tail -n 50 || echo "Diagnostics unavailable from run view.")
fi

if [ "$CONCLUSION" = "failure" ] || [ "$CONCLUSION" = "cancelled" ] || [ "$CONCLUSION" = "timed_out" ]; then
  if [ -n "$EXISTING" ]; then
    echo "Updating existing incident #$EXISTING for $MARKER"
    {
      echo "### ⚠️ Repeated Failure in Run [$RUN_ID]($RUN_URL)"
      echo "- **Commit**: \`$SHORT_SHA\` on branch \`$HEAD_BRANCH\`"
      echo "- **Status**: \`$CONCLUSION\` on \`$RUNNER_OS\`"
      echo ""
      echo '```text'
      echo "$ERROR_LOG"
      echo '```'
      echo ""
      echo "@jules The E2E test failure persists in the latest run. Please review and provide a fix."
    } > /tmp/e2e-comment.md
    gh issue comment "$EXISTING" --body-file /tmp/e2e-comment.md
  else
    echo "Creating new structured Jules task issue for E2E failure..."
    {
      echo "# [fix(ci-e2e)] ${JOB_NAME} failed on ${RUNNER_OS}"
      echo ""
      echo "> Automated E2E failure incident report for Jules."
      echo "> Workflow: \`${WORKFLOW_NAME}\` | Job: \`${JOB_NAME}\` | OS: \`${RUNNER_OS}\` | Ref: \`${HEAD_BRANCH}\`"
      echo ""
      echo "---"
      echo ""
      echo "## Current State (MEASURABLE)"
      echo "- **Workflow**: \`${WORKFLOW_NAME}\` (Run [${RUN_ID}](${RUN_URL}))"
      echo "- **Job / Target**: \`${JOB_NAME}\`"
      echo "- **Runner OS**: \`${RUNNER_OS}\`"
      echo "- **Commit SHA**: \`${HEAD_SHA}\` (\`${SHORT_SHA}\`)"
      echo "- **Exit Status**: \`${CONCLUSION}\`"
      echo "- **Failing Command**: \`${REPRO_COMMAND}\`"
      echo ""
      echo "## Failure Diagnostics"
      echo '```text'
      echo "$ERROR_LOG"
      echo '```'
      echo ""
      echo "## Reproduction Steps (VERIFIABLE BY COMMAND)"
      echo "- [ ] Checkout branch/commit: \`git checkout ${HEAD_SHA}\`"
      echo "- [ ] Reproduce failing target: \`${REPRO_COMMAND}\`"
      echo "- [ ] Inspect error output and verify failure matches CI diagnostics above"
      echo ""
      echo "## Target File Islands (DISJOINT)"
      (
        set -f
        IFS=','
        for file in $TARGET_FILES; do
          file_trimmed=$(echo "$file" | xargs)
          [ -n "$file_trimmed" ] && echo "- \`$file_trimmed\`"
        done
      )
      echo ""
      echo "## Acceptance Criteria (VERIFIABLE BY COMMAND)"
      echo "- [ ] \`${REPRO_COMMAND}\` passes with exit code 0"
      echo "- [ ] All cross-platform E2E assertions succeed on ${RUNNER_OS}"
      echo "- [ ] No regression in related test suites (\`cargo test --features ci-safe\` or \`pnpm test\`)"
      echo "- [ ] PR contains valid source modifications (Anti-Empty-PR guard)"
      echo ""
      echo "## PR Delivery Requirements (ANTI-EMPTY-PR)"
      echo "- [ ] \`git status --porcelain\` shows modified/added files before pushing"
      echo "- [ ] \`git diff --stat HEAD\` is non-empty and contains substantive code changes"
      echo "- [ ] PR targets base branch \`sprint/xavier-2026-10\` (or task branch)"
      echo "- [ ] PR touches ONLY the declared file islands"
      echo ""
      echo "## Notes / Assumptions"
      echo "- Autonomous execution delegated to Jules."
      echo "- If failure is due to missing native system packages or headless display settings, adapt the runner configuration accordingly."
      echo ""
      echo "@jules Please inspect the failure above, create a branch \`sprint/xavier-2026-10-<task>\`, implement the fix with tests, and submit a PR to resolve this E2E failure."
      echo ""
      echo "$MARKER"
    } > /tmp/e2e-issue-body.md

    if [ "$DRY_RUN" = "1" ]; then
      echo "DRY_RUN=1: Would create issue with title: $TITLE"
      echo "--- ISSUE BODY ---"
      cat /tmp/e2e-issue-body.md
      echo "--- END ISSUE BODY ---"
    else
      CREATED_URL=$(gh issue create --title "$TITLE" --body-file /tmp/e2e-issue-body.md --label "$LABEL,jules,bug")
      echo "Created issue: $CREATED_URL"
    fi
  fi
elif [ "$CONCLUSION" = "success" ]; then
  if [ -n "$EXISTING" ]; then
    echo "Closing E2E failure incident #$EXISTING after green run"
    gh issue comment "$EXISTING" --body "E2E tests resolved and green in run [$RUN_ID]($RUN_URL) @ \`$SHORT_SHA\`."
    gh issue close "$EXISTING" --reason "completed"
  else
    echo "No open E2E failure incident found to close."
  fi
else
  echo "Ignoring conclusion: $CONCLUSION"
fi

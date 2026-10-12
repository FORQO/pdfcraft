# Only the latest push run on the integrated RU release branch may authorize a
# release tag. Ignore newer PR runs, other branches, and stale rerun attempts.
[.workflow_runs[]
 | select(.head_branch == "feature/ru-edition" and .event == "push" and .name == $flow)]
| sort_by(.run_number, .run_attempt)
| last | .conclusion // "missing"

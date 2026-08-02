---
name: update-ci-workflows
description: Workflow command scaffold for update-ci-workflows in wezterm.
allowed_tools: ["Bash", "Read", "Write", "Grep", "Glob"]
---

# /update-ci-workflows

Use this workflow when working on **update-ci-workflows** in `wezterm`.

## Goal

Update or regenerate GitHub Actions CI workflow files, often in bulk, to add new platforms, bump dependencies, or tweak build/test matrix.

## Common Files

- `.github/workflows/gen_*.yml`
- `ci/generate-workflows.py`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Edit or run ci/generate-workflows.py to update workflow templates.
- Regenerate or manually edit multiple .github/workflows/gen_*.yml files.
- Commit all changed workflow files together.

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.
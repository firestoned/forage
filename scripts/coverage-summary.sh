#!/usr/bin/env bash
# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: Apache-2.0
#
# Render a cargo-llvm-cov JSON summary as a Markdown table (ADR-0005).
#
#   scripts/coverage-summary.sh <summary.json> <title> [uncovered-lines.txt]
#
# Prints to stdout and, when running in GitHub Actions, also appends to
# $GITHUB_STEP_SUMMARY so the table shows on the workflow run page.

set -euo pipefail

[ "$#" -ge 2 ] || { echo "usage: $0 <summary.json> <title> [uncovered-lines.txt]" >&2; exit 2; }
summary="$1"
title="$2"
uncovered="${3:-}"
[ -f "${summary}" ] || { echo "ERROR: ${summary} not found" >&2; exit 1; }
command -v jq >/dev/null 2>&1 || { echo "ERROR: jq is required" >&2; exit 1; }

render() {
    echo "### ${title}"
    echo
    echo "| File | Lines | Functions | Regions |"
    echo "|---|---:|---:|---:|"
    jq -r --arg root "$(pwd)/" '
        def cell(m): "\(m.percent * 100 | round / 100)% (\(m.covered)/\(m.count))";
        (.data[0].files[]
            | "| `\(.filename | ltrimstr($root))` | \(cell(.summary.lines)) | \(cell(.summary.functions)) | \(cell(.summary.regions)) |"),
        (.data[0].totals
            | "| **Total** | **\(cell(.lines))** | **\(cell(.functions))** | **\(cell(.regions))** |")
    ' "${summary}"
    echo
    if [ -n "${uncovered}" ] && [ -f "${uncovered}" ]; then
        if grep -q "Uncovered Lines" "${uncovered}"; then
            echo "**Unexecuted lines:**"
            echo
            echo '```'
            sed -n '/Uncovered Lines/,$p' "${uncovered}" | tail -n +2 | sed "s|$(pwd)/||"
            echo '```'
        else
            echo "No unexecuted lines (llvm-cov per-line data)."
        fi
        echo
    fi
}

out="$(render)"
echo "${out}"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    echo "${out}" >> "${GITHUB_STEP_SUMMARY}"
fi

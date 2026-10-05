#!/usr/bin/env bash
# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: Apache-2.0
#
# Release packaging for build.yaml (ADR-0004). Kept here, not inline in the
# workflow, so the release layout can be reproduced and tested locally.
#
#   scripts/release.sh tarball <dir> <binary> <name>
#       Find <binary> under <dir> and pack it as <dir>/<name>.tar.gz.
#
#   scripts/release.sh assets <dir>
#       <dir> holds every artifact of the run, one subdirectory per artifact
#       (actions/download-artifact layout). Sort them into
#       <dir>/{release,sboms,signatures,provenance} and write
#       <dir>/release/checksums.sha256 over everything that is uploaded.

set -euo pipefail

usage() {
    echo "usage: $0 tarball <dir> <binary> <name> | assets <dir>" >&2
    exit 2
}

tarball() {
    local dir="$1" binary="$2" name="$3" path
    path="$(find "${dir}" -type f -name "${binary}" | head -1)"
    [ -n "${path}" ] || { echo "ERROR: ${binary} not found under ${dir}" >&2; find "${dir}" -ls >&2; exit 1; }
    chmod +x "${path}"
    tar czf "${dir}/${name}.tar.gz" -C "$(dirname "${path}")" "$(basename "${path}")"
    ls -lh "${dir}/${name}.tar.gz"
}

# Copy every file matching a glob (relative to the cwd) into a directory.
# Fails when nothing matches: a release missing a class of asset is broken.
collect() {
    local dest="$1" pattern="$2" found=false f
    for f in ${pattern}; do
        [ -f "${f}" ] || continue
        cp "${f}" "${dest}/"
        found=true
    done
    ${found} || { echo "ERROR: no release files match ${pattern}" >&2; exit 1; }
}

assets() {
    local dir="$1"
    cd "${dir}"
    mkdir -p release sboms signatures provenance
    collect release '*-signed/*.tar.gz'
    collect signatures '*-signed/*.tar.gz.bundle'
    collect sboms 'sbom-*/*.cdx.json'
    # The SLSA generator uploads a directory named after the provenance file.
    collect provenance '*.intoto.jsonl/*.intoto.jsonl'

    (cd release && sha256sum ./*.tar.gz | sed 's| \./| |' > checksums.sha256)
    local sub
    for sub in sboms signatures provenance; do
        (cd "${sub}" && sha256sum ./* | sed 's| \./| |') >> release/checksums.sha256
    done

    echo "Release assets prepared:"
    ls -lh release sboms signatures provenance
}

[ "$#" -ge 1 ] || usage
case "$1" in
    tarball) [ "$#" -eq 4 ] || usage; tarball "$2" "$3" "$4" ;;
    assets)  [ "$#" -eq 2 ] || usage; assets "$2" ;;
    *) usage ;;
esac

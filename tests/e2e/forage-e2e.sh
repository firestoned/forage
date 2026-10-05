#!/usr/bin/env bash
# Copyright (c) 2025 Erick Bourgeois, firestoned
# SPDX-License-Identifier: Apache-2.0
#
# forage end-to-end suites against a real Kubernetes API server (ADR-0003).
#
# forage never talks to a cluster; its whole contract is "the YAML it prints is
# accepted by bindy". Unit tests can only assert the structs forage serializes.
# Only a real API server holding bindy's real CRDs (pinned by BINDY_VERSION)
# can say whether those structs still match bindy's OpenAPI schema, CEL rules
# and field names, which is exactly the drift hand-written CRD structs
# (ADR-0002) are exposed to.
#
#   schema   server-side dry run with strict field validation, for every CLI
#            variant in SCHEMA_VARIANTS (flags, filters, JSON stream, debug)
#            over every fixture in SCHEMA_FIXTURES (basic, edge): every emitted
#            manifest is accepted, no unknown or misspelt fields
#   apply    real apply: object count matches the stream; every stored spec
#            equals the emitted spec field by field (nothing pruned or
#            defaulted away); each DNSZone's recordsFrom selector matches
#            exactly its own records; re-apply is a no-op; two runs of forage
#            emit identical bytes
#   update   re-import after the source zone changes: only the changed record
#            is reconfigured, a new record is created, everything else is
#            unchanged
#
# Each suite owns its own kind cluster, so CI runs them concurrently.
#
# Usage: tests/e2e/forage-e2e.sh {schema|apply|update|all|clean}
#
# Environment:
#   FORAGE_BIN      forage binary under test (default target/release/forage)
#   BINDY_VERSION   bindy tag whose CRDs are installed (default v0.7.1)
#   FIXTURE         named.conf fed to forage (default tests/fixtures/basic/named.conf)
#   KIND_CLUSTER    kind cluster name (default forage-e2e-<suite>, or forage-e2e-all)
#   KEEP_CLUSTER    non-empty: leave the cluster up for diagnostics
#   KIND_NODE_IMAGE optional kind node image override

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

readonly E2E_NAMESPACE="forage-e2e"
readonly ALT_NAMESPACE="forage-e2e-alt"
readonly CRD_ESTABLISH_TIMEOUT="60s"
readonly MANAGED_BY_SELECTOR="bindy.firestoned.io/managed-by=forage"
readonly ZONE_LABEL="bindy.firestoned.io/zone"

# The bindy CRDs forage emits resources for. Kept to the kinds forage maps, so
# a new bindy CRD does not silently widen what this suite claims to cover.
readonly CRD_FILES=(
    dnszones
    arecords
    aaaarecords
    cnamerecords
    mxrecords
    txtrecords
    srvrecords
    caarecords
)

# CLI variants the schema suite validates, against every fixture in
# SCHEMA_FIXTURES. Each line is a label, then the extra forage arguments;
# @FIXTURE_DIR@ is replaced with the fixture's directory. Every variant must
# be accepted by the API server.
readonly SCHEMA_VARIANTS=(
    "default|"
    "json-stream|--output json"
    "debug-logging|--debug"
    "cluster-ref|--cluster-ref forage-e2e-cluster"
    "alt-namespace|--namespace ${ALT_NAMESPACE}"
    "zone-dir|--zone-dir @FIXTURE_DIR@"
    "skip-records|--skip-records"
    "record-types|--record-types a,aaaa,mx"
    "suffix-filter|--zone-filter *.com"
    "prefix-filter|--zone-filter example.*"
    "exact-filter|--zone-filter example.com"
)

# basic: every record type forage maps. edge: options block, zone without a
# file option, missing zone file, zone file without SOA or $TTL, PTR,
# absolute owners, a relative target, a truncated CR name. generated: built at
# run time (absolute paths are machine-specific): an `options { directory }`
# base directory and an absolute zone-file path.
readonly SCHEMA_FIXTURES=(basic edge generated)

# write_generated_fixture <dir>: named.conf using a `directory` option and an
# absolute `file` path, pointing at the committed fixtures.
write_generated_fixture() {
    local dir="$1"
    mkdir -p "${dir}"
    cat > "${dir}/named.conf" <<CONF
options {
    directory "${PROJECT_ROOT}/tests/fixtures/basic";
};
zone "example.com" {
    type primary;
    file "db.example.com";
};
zone "example.edu" {
    type primary;
    file "${PROJECT_ROOT}/tests/fixtures/edge/db.example.edu";
};
CONF
}

FORAGE_BIN="${FORAGE_BIN:-${PROJECT_ROOT}/target/release/forage}"
BINDY_VERSION="${BINDY_VERSION:-v0.7.1}"
FIXTURE="${FIXTURE:-${PROJECT_ROOT}/tests/fixtures/basic/named.conf}"
WORK_DIR="${PROJECT_ROOT}/target/e2e"

usage() {
    echo "usage: $0 {schema|apply|update|all|clean}" >&2
    exit 2
}

log() { echo "==> $*"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

require() {
    local tool
    for tool in "$@"; do
        command -v "${tool}" >/dev/null 2>&1 || fail "${tool} not found on PATH"
    done
}

ctx() { echo "kind-${KIND_CLUSTER}"; }
kc() { kubectl --context "$(ctx)" "$@"; }
crd_kinds() { (IFS=,; echo "${CRD_FILES[*]}"); }

create_cluster() {
    if kind get clusters 2>/dev/null | grep -qx "${KIND_CLUSTER}"; then
        log "reusing kind cluster ${KIND_CLUSTER}"
        return
    fi
    log "creating kind cluster ${KIND_CLUSTER}"
    local args=(create cluster --name "${KIND_CLUSTER}" --wait 120s)
    if [ -n "${KIND_NODE_IMAGE:-}" ]; then
        args+=(--image "${KIND_NODE_IMAGE}")
    fi
    kind "${args[@]}"
}

delete_cluster() {
    if [ -n "${KEEP_CLUSTER:-}" ]; then
        log "KEEP_CLUSTER set; leaving ${KIND_CLUSTER} up"
        return
    fi
    kind delete cluster --name "${KIND_CLUSTER}" >/dev/null 2>&1 || true
}

install_crds() {
    local crd_dir="${WORK_DIR}/crds/${BINDY_VERSION}"
    local base="https://raw.githubusercontent.com/firestoned/bindy/${BINDY_VERSION}/deploy/operator/crds"
    mkdir -p "${crd_dir}"
    local crd
    for crd in "${CRD_FILES[@]}"; do
        if [ ! -s "${crd_dir}/${crd}.crd.yaml" ]; then
            curl -fsSL -o "${crd_dir}/${crd}.crd.yaml" "${base}/${crd}.crd.yaml" \
                || fail "could not fetch ${crd}.crd.yaml for bindy ${BINDY_VERSION}"
        fi
    done
    log "installing bindy ${BINDY_VERSION} CRDs (${#CRD_FILES[@]} kinds)"
    kc apply --server-side -f "${crd_dir}" >/dev/null
    kc wait --for condition=Established --timeout "${CRD_ESTABLISH_TIMEOUT}" \
        -f "${crd_dir}" >/dev/null
    local ns
    for ns in "${E2E_NAMESPACE}" "${ALT_NAMESPACE}"; do
        kc create namespace "${ns}" --dry-run=client -o yaml | kc apply -f - >/dev/null
    done
}

check_binary() {
    [ -f "${FORAGE_BIN}" ] || fail "forage binary not found: ${FORAGE_BIN}"
    # actions/download-artifact does not preserve the executable bit.
    [ -x "${FORAGE_BIN}" ] || chmod +x "${FORAGE_BIN}"
}

# render <out> <conf> [forage args...]: run forage, fail on an empty stream.
render() {
    local out="$1" conf="$2"
    shift 2
    check_binary
    # Default namespace unless the variant chooses its own (clap rejects a
    # repeated --namespace).
    local ns_args=(--namespace "${E2E_NAMESPACE}")
    case " $* " in *" --namespace "*) ns_args=() ;; esac
    "${FORAGE_BIN}" --conf "${conf}" "${ns_args[@]}" "$@" > "${out}"
    [ -s "${out}" ] || fail "forage emitted nothing for ${conf} $*"
}

# count_manifests <file>: number of manifests in a YAML or JSON stream.
count_manifests() {
    local yaml json
    yaml="$(grep -c '^kind:' "$1" || true)"
    json="$(grep -c '^  "kind":' "$1" || true)"
    echo $((yaml + json))
}

suite_schema() {
    local fixture conf fixture_dir variant label args manifest runs=0
    write_generated_fixture "${WORK_DIR}/fixtures/generated"
    for fixture in "${SCHEMA_FIXTURES[@]}"; do
        fixture_dir="${PROJECT_ROOT}/tests/fixtures/${fixture}"
        [ -d "${fixture_dir}" ] || fixture_dir="${WORK_DIR}/fixtures/${fixture}"
        conf="${fixture_dir}/named.conf"
        for variant in "${SCHEMA_VARIANTS[@]}"; do
            label="${variant%%|*}"
            args="${variant#*|}"
            args="${args//@FIXTURE_DIR@/${fixture_dir}}"
            manifest="${WORK_DIR}/${KIND_CLUSTER}-${fixture}-${label}.out"
            # shellcheck disable=SC2086  # args is a deliberate word list
            render "${manifest}" "${conf}" ${args} 2>/dev/null
            log "${fixture}/${label}: $(count_manifests "${manifest}") manifest(s), server-side dry run (strict)"
            kc apply --dry-run=server --validate=strict -f "${manifest}" >/dev/null \
                || fail "${fixture}/${label} rejected by bindy ${BINDY_VERSION} CRDs"
            runs=$((runs + 1))
        done
    done
    log "PASS: ${runs} fixture x variant runs accepted by bindy ${BINDY_VERSION} CRDs"

    log "a broken input must fail closed: non-zero exit, nothing for kubectl"
    local stdout status=0
    stdout="$("${FORAGE_BIN}" --conf "${WORK_DIR}/does-not-exist/named.conf" 2>/dev/null)" || status=$?
    [ "${status}" -ne 0 ] || fail "forage exited 0 on a missing named.conf"
    [ -z "${stdout}" ] || fail "forage printed manifests on a missing named.conf"
    log "PASS: missing named.conf exits ${status} with empty stdout"
}

# Every key forage set under .spec must be stored with the same value.
assert_specs_round_trip() {
    local json="$1" kind name want got checked=0
    while IFS=$'\t' read -r kind name want; do
        got="$(kc get "${kind}" "${name}" -n "${E2E_NAMESPACE}" -o json \
            | jq -cS --argjson want "${want}" '.spec | with_entries(select(.key as $k | $want | has($k)))')"
        [ "${got}" = "$(jq -cS . <<<"${want}")" ] \
            || fail "${kind}/${name} stored spec differs:\n want ${want}\n got  ${got}"
        checked=$((checked + 1))
    done < <(jq -r '[.kind, .metadata.name, (.spec | tojson)] | @tsv' "${json}")
    log "stored spec equals emitted spec for ${checked} object(s)"
}

# Each DNSZone selects its records by zone label; the selector must match
# exactly the records forage emitted for that zone.
assert_zone_selectors() {
    local json="$1" zone expected actual
    while IFS= read -r zone; do
        expected="$(jq -r --arg z "${zone}" \
            'select(.kind != "DNSZone" and .metadata.labels["'"${ZONE_LABEL}"'"] == $z) | .metadata.name' \
            "${json}" | wc -l | tr -d ' ')"
        actual="$(kc get "$(crd_kinds | sed 's/^dnszones,//')" -n "${E2E_NAMESPACE}" \
            -l "${ZONE_LABEL}=${zone}" -o name | wc -l | tr -d ' ')"
        [ "${actual}" -eq "${expected}" ] \
            || fail "zone ${zone}: selector matches ${actual} records, forage emitted ${expected}"
    done < <(jq -r 'select(.kind == "DNSZone") | .spec.recordsFrom[0].selector.matchLabels["'"${ZONE_LABEL}"'"]' "${json}")
    log "every DNSZone selector matches exactly its own records"
}

suite_apply() {
    local manifest="${WORK_DIR}/${KIND_CLUSTER}.yaml"
    local json="${WORK_DIR}/${KIND_CLUSTER}.json"
    local rerender="${WORK_DIR}/${KIND_CLUSTER}.rerun.yaml"
    render "${manifest}" "${FIXTURE}"
    render "${json}" "${FIXTURE}" --output json
    local expected
    expected="$(count_manifests "${manifest}")"

    log "apply ${expected} manifest(s)"
    kc apply --validate=strict -f "${manifest}" >/dev/null

    local actual
    actual="$(kc get "$(crd_kinds)" -n "${E2E_NAMESPACE}" -l "${MANAGED_BY_SELECTOR}" \
        -o name | wc -l | tr -d ' ')"
    [ "${actual}" -eq "${expected}" ] \
        || fail "expected ${expected} forage-managed objects, found ${actual}"
    log "object count matches (${actual})"

    assert_specs_round_trip "${json}"
    assert_zone_selectors "${json}"

    log "re-apply must be a no-op"
    local changed
    changed="$(kc apply -f "${manifest}" | grep -v ' unchanged$' || true)"
    [ -z "${changed}" ] || fail "re-apply changed objects: ${changed}"

    log "re-apply of the JSON stream must also be a no-op"
    changed="$(kc apply -f "${json}" | grep -v ' unchanged$' || true)"
    [ -z "${changed}" ] || fail "JSON re-apply changed objects: ${changed}"

    log "forage output must be deterministic"
    render "${rerender}" "${FIXTURE}"
    diff -u "${manifest}" "${rerender}" || fail "two runs of forage emitted different output"

    log "PASS: applied, round-tripped, selectors exact, idempotent, deterministic"
}

suite_update() {
    local src_dir edited
    src_dir="$(dirname "${FIXTURE}")"
    edited="${WORK_DIR}/${KIND_CLUSTER}-edited"
    rm -rf "${edited}"
    cp -R "${src_dir}" "${edited}"

    local before="${WORK_DIR}/${KIND_CLUSTER}.before.yaml"
    local after="${WORK_DIR}/${KIND_CLUSTER}.after.yaml"
    render "${before}" "${edited}/named.conf"
    kc apply --validate=strict -f "${before}" >/dev/null
    log "initial import applied ($(count_manifests "${before}") manifest(s))"

    # The source zone changes: api moves address, and a record is added.
    sed -i.bak 's/^api 300 IN A    203.0.113.5$/api 300 IN A    203.0.113.6/' "${edited}/db.example.com"
    grep -q '203.0.113.6' "${edited}/db.example.com" \
        || fail "fixture edit did not apply; update suite is out of sync with the fixture"
    # `zzz` sorts last: CR names carry a per-type index, so a record that sorts
    # earlier renames every later one (roadmap 01, stable CR names). Inserted
    # after `sip`, not appended, so the fixture's TXT stays the last line
    # (the golden files pin record order).
    awk '{ print } /^sip     IN A    198\.51\.100\.30$/ { print "zzz     IN A    198.51.100.99" }' \
        "${edited}/db.example.com" > "${edited}/db.example.com.new"
    mv "${edited}/db.example.com.new" "${edited}/db.example.com"
    grep -q '^zzz ' "${edited}/db.example.com" \
        || fail "fixture edit did not apply; update suite is out of sync with the fixture"
    render "${after}" "${edited}/named.conf"

    log "re-import after the zone changed"
    local result configured created unchanged
    result="$(kc apply --validate=strict -f "${after}")"
    configured="$(grep -c ' configured$' <<<"${result}" || true)"
    created="$(grep -c ' created$' <<<"${result}" || true)"
    unchanged="$(grep -c ' unchanged$' <<<"${result}" || true)"
    log "configured=${configured} created=${created} unchanged=${unchanged}"

    [ "${configured}" -eq 1 ] || fail "expected exactly 1 reconfigured object: ${result}"
    grep -q 'forage-example-com-api-a-0 configured$' <<<"${result}" \
        || fail "the changed api record was not the one reconfigured: ${result}"
    [ "${created}" -eq 1 ] || fail "expected exactly 1 new object: ${result}"
    grep -q 'forage-example-com-zzz-a-6 created$' <<<"${result}" \
        || fail "the new record was not created: ${result}"
    [ "${unchanged}" -eq "$(($(count_manifests "${before}") - 1))" ] \
        || fail "every other object must be unchanged: ${result}"
    [ "$(kc get arecord forage-example-com-api-a-0 -n "${E2E_NAMESPACE}" \
        -o jsonpath='{.spec.ipv4Addresses[0]}')" = "203.0.113.6" ] \
        || fail "api record does not hold the new address"

    log "PASS: re-import reconfigures and creates only what changed"
}

# run_suites <cluster> <suite>...: one cluster, CRDs installed once, suites in
# order (update last: it adds objects the apply suite would count).
run_suites() {
    KIND_CLUSTER="${KIND_CLUSTER_OVERRIDE:-$1}"
    shift
    trap delete_cluster EXIT
    create_cluster
    install_crds
    local s
    for s in "$@"; do "suite_${s}"; done
}

[ "$#" -eq 1 ] || usage
suite="$1"
KIND_CLUSTER_OVERRIDE="${KIND_CLUSTER:-}"

case "${suite}" in
    schema|apply|update)
        require kind kubectl curl jq
        mkdir -p "${WORK_DIR}"
        run_suites "forage-e2e-${suite}" "${suite}"
        ;;
    all)
        require kind kubectl curl jq
        mkdir -p "${WORK_DIR}"
        run_suites "forage-e2e-all" schema apply update
        ;;
    clean)
        require kind
        for cluster in forage-e2e-schema forage-e2e-apply forage-e2e-update forage-e2e-all forage-e2e-coverage; do
            kind delete cluster --name "${cluster}" >/dev/null 2>&1 || true
        done
        rm -rf "${WORK_DIR}"
        log "e2e clusters and work dir removed"
        ;;
    *) usage ;;
esac

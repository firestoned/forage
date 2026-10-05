<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# 01: Import fidelity

> Every record in the source must either reach the output or be reported.
> Silent loss is the failure mode that matters: applying an import that is
> missing a record removes that record from DNS at cut-over.

Found while auditing roadmap 00 and building the e2e fixture (2026-10-05).
Each item is a bug fix or small feature (TDD only) unless marked as needing an
ADR.

## Tasks

- [x] **CNAME/MX/TXT field names rejected by bindy.** forage emitted
      `alias`, `preference`/`exchange` and `value`; bindy v0.7.1 expects
      `target`, `priority`/`mailServer` and `text` (string array). Fixed
      2026-10-05 with `src/crd_tests.rs`; now guarded by `make e2e-schema`
      (ADR-0002, ADR-0003).
- [x] **hornet-bind9 0.1 zone-file parse loses or corrupts records**, with no
      warning. *Fixed 2026-10-05 by ADR-0006: forage now owns its parser;
      the regression tests are un-ignored and green.* Three cases, each pinned by an ignored test in
      `tests/cli_tests.rs`:
  - the record directly after a quoted `TXT` is dropped, whatever its type
    (first seen as SRV: `tests/fixtures/srv-after-txt/`);
  - the record directly after a one-line `SOA` is dropped (typically the
    zone's `NS`, so `DNSZone.spec.nameServers` comes out empty);
  - a blank owner (inherit the previous name) is read as owner `IN`, so the
    record is imported under the wrong name.

  All three pass against hornet's unreleased 0.2 parser (verified 2026-10-05
  with a path dependency; forage's 53 unit and 22 CLI tests also pass on it).
  (The basic fixture keeps its TXT last because the golden files pin that
  order.)
- [ ] **`include` statements are not followed.** The parser returns
      `Statement::Include(path)`; the mapper ignores it, so a `named.conf` that keeps its zones in included files
      (common) imports nothing and exits 0. Pinned by
      `test_zones_in_included_files_are_imported`. Resolve includes relative
      to `named.conf` (and `directory`), recursively, with a loop guard.
- [ ] **Relative targets are qualified at the root, not the zone.**
      `docs CNAME www` becomes target `www.` instead of `www.example.com.`;
      MX exchange, SRV target and NS names share the helper
      (`ensure_trailing_dot`). `$ORIGIN` is parsed but not applied either.
      Pinned by
      `test_map_relative_cname_target_is_qualified_with_the_zone`.
- [ ] **CR names are not stable.** Names carry a per-type index across the
      zone, so adding a record that sorts earlier renames every later record
      of that type: a re-import creates duplicates and orphans the old
      objects, which keep serving their old data. Pinned by
      `test_adding_a_record_does_not_rename_others` (the e2e `update` suite
      adds a record that sorts last for this reason). Breaking change to
      names: **needs an ADR** (content-derived names, e.g. owner + type + a
      short hash of the RRset key).
- [ ] **Truncated CR names can be invalid or collide.** Cutting at 253
      characters can leave a trailing `-` (rejected by the API server) and
      drops the `-<type>-<index>` suffix, so two long owners with a common
      prefix get the same name. Pinned by
      `test_truncated_cr_names_are_valid_and_unique`; solved by the same ADR.
- [x] **Warn when records are lost.** Every zone-file line the parser cannot
      read is reported as `<file>:<line>: not imported, could not parse`
      (2026-10-05, ADR-0006; `test_unparseable_zone_line_is_reported_not_dropped_silently`).
- [ ] **Relative `directory` option resolves against the caller's CWD.**
      `options { directory "."; }` makes forage look for zone files relative
      to wherever it was run, not to `named.conf`. BIND resolves it against
      named's working directory, which forage cannot know; resolving a
      relative `directory` against the `named.conf` parent (and logging it)
      is the useful behavior. `--zone-dir` remains the explicit override.
- [ ] **Zones inside `view` blocks are skipped** with only a log line. Decide
      how views map to bindy (one `DNSZone` per view needs distinct names and
      a way to express the view's match-clients). **Needs an ADR.**
- [ ] **Shell `DNSZone`s carry an invented SOA.** Secondary/stub/forward
      zones get `ns1.<zone>.`/`admin.<zone>.` and serial 0. Roadmap 00 Open
      Question 5 proposed marking them instead (annotation with the zone
      type). Decide with bindy what a non-primary import should look like.
      **Needs an ADR.**
- [ ] **`--dry-run`** from roadmap 00's Inputs table is not implemented.
- [ ] **`--record-types` help text** omits `NS` although roadmap 00 lists it;
      NS is folded into `DNSZone.spec.nameServers`, so either document that or
      reject `NS` explicitly.
- [ ] **CR names for the apex** come out as `forage-example-com---caa-0`
      (`@` sanitized away). Valid, but use `apex` (or similar) for
      readability. Changing names is a breaking change for anyone who already
      imported: note it in the CHANGELOG.

## Definition of done

All boxes ticked; every `#[ignore]` regression test (8 on 2026-10-05:
`cargo test -- --ignored`) is un-ignored and green; the threat model's
accepted risk R1 is closed or narrowed.

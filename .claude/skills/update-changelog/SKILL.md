---
name: update-changelog
description: Prepend an audit entry to .claude/CHANGELOG.md. MANDATORY after ANY code change: every entry MUST carry an **Author:** line, no exceptions.
---

<!--
  Copyright (c) 2025 Erick Bourgeois, firestoned
  SPDX-License-Identifier: Apache-2.0
-->

# update-changelog

Every change is auditable: prepend an entry to `.claude/CHANGELOG.md`
(newest first) in this exact format.

## Format

```markdown
## [YYYY-MM-DD HH:MM] - Brief Title

**Author:** <Name of requester or approver>

### Changed
- `path/to/file.rs`: Description of the change

### Why
Brief explanation of the business or technical reason.

### Impact
- [ ] Breaking change
- [ ] Config change only
- [ ] Documentation only
```

## Rules

- `**Author:**` is MANDATORY: the requester or approver (usually Erick
  Bourgeois), never Claude.
- Check the applicable `### Impact` boxes; a breaking change to the emitted
  manifests (names, labels, fields) also needs a README note.
- New dependencies: record why the crate was chosen.

## Verification

Entry has the `**Author:**` line, a timestamp, and at least one `### Changed`
item.

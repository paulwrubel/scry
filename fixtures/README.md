# Cross-language fixtures

Fixtures in this directory are shared, language-neutral test vectors for rules
that both the Rust core and the TypeScript UI must implement identically. They
are data, not code: each side loads the same files and checks the same
`expected` values.

## Layout

One directory per rule area, and exactly one `cases.json` inside it:

```text
fixtures/
  README.md
  <area>/
    cases.json
```

Each area belongs to the issue that introduces the dual-language rule it
covers. That issue owns the area's schema, its cases, and its registration in
both loaders.

## Envelope

`cases.json` is a versioned envelope:

```json
{
  "version": 1,
  "area": "positions",
  "description": "optional human-readable summary",
  "cases": []
}
```

| Field         | Type   | Required | Meaning                                   |
| ------------- | ------ | -------- | ----------------------------------------- |
| `version`     | number | yes      | Fixture format version.                   |
| `area`        | string | yes      | Area name; must equal the directory name. |
| `description` | string | no       | Human-readable summary of the area.       |
| `cases`       | array  | yes      | One or more cases.                        |

## Case

Each entry in `cases` has this shape:

```json
{
  "name": "insert-returns-key-between-neighbours",
  "description": "optional human-readable summary",
  "kind": "insert",
  "input": {},
  "expected": {}
}
```

| Field         | Type   | Required | Meaning                            |
| ------------- | ------ | -------- | ---------------------------------- |
| `name`        | string | yes      | Unique within the area.            |
| `description` | string | no       | Human-readable summary of the case. |
| `kind`        | string | yes      | Selects the operation under test.  |
| `input`       | object | yes      | Operation input.                   |
| `expected`    | object | yes      | Expected operation output.         |

`kind` names the operation a consumer should run; it is not a free-form label.
A loader that does not recognize a `kind` fails rather than skipping the case.

## Rules

- `version` starts at `1`. It is bumped only for an incompatible schema or
  semantic change. Adding cases is additive and does not bump it.
- Consumers reject any version other than `1`. There is no negotiation and no
  upgrade path; an unsupported version is a hard failure.
- Case names are unique within an area. Consumers must not disambiguate by
  position or by description.
- Unknown fields are rejected at both the envelope and case level, so a typo
  or a field meant for another area cannot be silently ignored.
- `input` and `expected` are always JSON objects, even when an operation takes
  a single scalar; wrap it.
- Every case must be consumed on both sides. A case checked only by Rust or
  only by TypeScript is a bug: it does not pin the contract.
- Every fixture area must be registered in both loaders. On the Rust side the
  registry is `REGISTERED_AREAS` in
  `crates/scry-core/tests/common.rs`; the TypeScript loader keeps the same
  list. An on-disk directory that is not registered fails
  `every_fixture_directory_is_registered`, so a new area cannot be silently
  ignored.
- Consumers read `version` and `area` from the file but do not trust them: the
  loader cross-checks the declared `area` against the directory it loaded and
  rejects a mismatch.

## Loaders

- Rust: `crates/scry-core/tests/common.rs` provides `fixture_dir`,
  `fixture_areas_on_disk`, `parse_fixture_file`, and `load_area`; the generic
  contract tests live in `crates/scry-core/tests/fixtures.rs`. Integration
  tests include the loader with `mod common;`.
- TypeScript: the UI package's loader (added with the first area) reads this
  same directory and registers the same areas.

Adding an area means adding `fixtures/<area>/cases.json`, registering `<area>`
in both loaders, and consuming every case on both sides. Until an area is
registered and consumed, the contract is not pinned.

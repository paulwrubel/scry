# Replica

## Purpose

The Replica owns the durable confirmed state and the durable outbox. Visible state is the confirmed state with the pending operations replayed over it, using the shared `op-replay` contract (`ui/src/contract/op-replay.ts`) rather than a second implementation of the rules.

The confirmed copy is disposable: it can be thrown away and re-fetched with a bootstrap. The outbox is not. Every operation that has not been acknowledged must survive a reload, a schema change, and a service-worker update, because it is the only record that the server has not seen the user's work.

## Storage choice

The Replica uses `idb`, a small promise wrapper around IndexedDB, rather than raw IndexedDB or Dexie. The schema is five simple object stores with explicit transactions; there is no need for a query framework, reactive layer, or migration DSL. `fake-indexeddb` supplies the IndexedDB API in Vitest, so the same code runs unmodified in tests.

## Database layout

One IndexedDB database named `scry-replica` holds:

- `projects`, `sections`, `tasks`, and `comments`, each keyed by the entity's UUID `id`;
- `outbox`, with an auto-increment `seq` key and a unique `operation_id` index;
- `meta`, holding `schema_version`, `needs_bootstrap`, and the four order arrays (`project_order`, `section_order`, `task_order`, `comment_order`).

IndexedDB returns records in key order, and UUID keys do not carry the order of `ClientState` arrays. The order arrays record that order so a reload can rebuild each project's `sections`, `tasks`, and `comments` exactly as they were persisted. Any entity not present in an order array is appended, so a partially written or hand-edited database still hydrates.

## Read and write path

At startup, `openReplica` reads the four confirmed stores plus `meta` in one readonly transaction and hydrates `ClientState`, then loads the outbox. `bootstrap` rewrites the confirmed stores and their order arrays in one readwrite transaction through `saveConfirmed`, which clears the stores first so the new state fully replaces the old one. `applySyncResult` applies only changes newer than the stored cursor and writes them together with the new cursor in one transaction, so a stale response cannot roll the confirmed state back.

Confirmed writes and outbox writes use separate transactions, so no atomicity is claimed between them. A crash between an outbox append and a confirmed rewrite leaves a pending operation that will replay on the next open, which is the intended outcome.

## Schema lifecycle

`CONFIRMED_SCHEMA_VERSION` versions the shape of the confirmed state. When the stored version does not match, the Replica clears only the four confirmed stores, writes the current version and `needs_bootstrap = true`, and returns an empty confirmed state. The `outbox` is left untouched, so pending operations survive the wipe and replay against the next bootstrap. `bootstrap` writes `needs_bootstrap = false` along with the fresh confirmed state.

IndexedDB structural upgrades are separate from this version and run through the `upgrade` callback. A future structural upgrade must preserve the `outbox` store and its contents unless the operation wire format itself is being replaced, as in the version 3 notes-to-comments upgrade, which cleared the outbox.

## Outbox

The outbox is FIFO by `seq`, the auto-increment key. Operations are stored with their original `operation_version`, and `acknowledge` deletes them by `operation_id` through the unique index. Nothing rewrites a queued operation's version.

Service-worker updates replace cached files; they do not touch IndexedDB. The outbox and the operation versions in it therefore survive a new deployment, and an old operation can still be sent after the app shell changes.

## Persistence and errors

Opening the database requests `navigator.storage.persist()`. Denial is not an error: the result is exposed as `persisted` so callers can warn or retry. Storage failures are mapped to `ReplicaError`, tagged with a `kind` of `blocked`, `quota`, or `storage`. A `QuotaExceededError` becomes `kind: 'quota'`; other IndexedDB failures become `kind: 'storage'`.

## Multiple tabs

Every tab opens its own connection and syncs independently; there is no leader election or Web Lock. `applyConfirmedChangesIfNewer` ignores responses that are not ahead of the stored cursor and advances the cursor in the same transaction as the changes, so concurrent tabs merge safely. The provider reloads the replica on `focus`, `visibilitychange`, and `pageshow` so a tab picks up writes made by another. The `blocking` callback closes this tab's connection when another connection wants to upgrade the database, and `onBlocked` reports the opposite case: this tab asked to upgrade and another connection refused to close.

## Testing

Tests use `fake-indexeddb/auto`, a fresh database name per test, and raw `openDB` reads to inspect the UUID-keyed stores and outbox directly. Covered cases include: reload preserving confirmed state, FIFO outbox order, offline create/edit/delete through visible state, acknowledgement removing operations, pull changes updating confirmed state, stable snapshots and subscriber notifications, a confirmed-schema bump clearing confirmed state while preserving the outbox, old operation versions surviving reopen, and persistence denial and grant.

## Luxide

Luxide has no local replica or outbox; the server round-trip is the only state path. This module follows the repository's `utils/` placement for non-React infrastructure. React bindings and hooks live in `ui/src/providers/Replica/`.

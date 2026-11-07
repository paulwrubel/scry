# Sync client

## Purpose

The sync client is the one place that speaks the #46 sync protocol against `/api/v1/sync` and `/api/v1/snapshot`, using the Replica's public methods. It owns no IndexedDB access; durable state and the outbox stay behind the `Replica` interface.

## Server contract

- `POST /api/v1/sync` takes `{ op_version, last_seen_seq, ops }` and returns `{ acks, changes, next_seq }`.
- `GET /api/v1/snapshot` returns `{ projects, last_seen_seq }`.

Both requests live in `ui/src/utils/api.ts`, so the client only decides when to send them and what to do with the responses.

## Bootstrap

When `replica.needsBootstrap` is true, the client fetches the snapshot, calls `replica.bootstrap({ projects }, last_seen_seq)`, and then continues with the normal push/pull. Continuing matters because a confirmed-schema wipe preserves the outbox, and that first push is what drains it.

## Cursor

The Replica stores `last_seen_seq`. The client reads it from `replica.getSnapshot().lastSeenSeq`, sends it as `last_seen_seq`, and writes the response's `next_seq` through `replica.setLastSeenSeq` after applying changes. A confirmed-schema bump resets the cursor to `0` and sets `needsBootstrap`, so the next sync starts from a fresh snapshot.

## Push/pull order

A sync attempt runs in this order:

1. Apply the returned `changes` with `replica.applyChanges`.
2. Classify every ack and collect the acknowledged `operation_id`s.
3. Remove those ids with `replica.acknowledge`.
4. Set the cursor from `next_seq`.
5. Publish the final status.

New operations queued while the request is in flight are not part of the pushed batch and are not removed, because acknowledgement is by `operation_id` rather than by position.

`sync()` coalesces concurrent calls onto the in-flight promise, so one client runs at most one request at a time.

## Acknowledgements

- `applied` acknowledgements leave the outbox.
- Rejections with any reason other than `unsupported_version` also leave the outbox; they are terminal and reported through `rejected`.
- A rejection with reason `unsupported_version` stays queued and puts the client in the `error` state, so the operation is retried once the client is upgraded.
- Rejected operations are exposed in the in-memory `SyncSnapshot` for the current session only; they are not persisted.

## Failures

Errors are tagged `SyncError` values with a `kind` of:

- `unauthorized` — a `401` from either endpoint.
- `unsupported_version` — a request-level `400` reporting an unsupported operation version, or a per-operation `unsupported_version` rejection.
- `network` — the fetch itself rejected (transport failure).
- `server` — any other non-OK response or an unreadable sync response body.
- `storage` — a `ReplicaError` raised while bootstrapping, applying, acknowledging, or writing the cursor.

The outbox is preserved on every failure except for ids the server explicitly acknowledged in that response. In the per-operation unsupported-version case, applied operations are still acknowledged and the cursor still advances before the client reports the error.

## Triggers

`ui/src/hooks/useSyncTriggers.ts` owns scheduling and calls the client; the client itself does not schedule anything. It syncs once at startup per client (guarded against StrictMode remounts), debounces growth of the outbox, polls only while the document is visible, syncs on `visibilitychange` when the tab becomes visible, syncs on `online`, and exposes a manual `syncNow`. A hidden tab does not poll.

Failures retry with exponential backoff, clamped at a configurable maximum; success resets the delay. 401 and `unsupported_version` stop automatic retries because they need a login or a client upgrade. Timing values are configurable through `SyncTiming`; the defaults are 1 s edit debounce, 15 s polling, 1 s initial retry, and 30 s cap.

React composition lives in `ui/src/providers/Sync/`: `SyncProvider` creates one client per ready Replica, `useSyncTriggers` supplies scheduling, and `useSync`/`useSyncSnapshot` expose the client and its snapshot. `LayoutNavbar/SyncButton` uses those hooks to show syncing/failure state and call `syncNow`.

Multi-tab coordination lives in `ui/src/hooks/useSyncLeadership.ts`. One tab holds an exclusive `scry-sync-leader` Web Lock for its lifetime; only that tab runs the triggers. The leader broadcasts `replica-updated` after a sync attempt, and followers reload from IndexedDB. A follower that queues an edit broadcasts `outbox-changed`; the leader reloads the outbox and syncs. Messages carry no state. When Web Locks or BroadcastChannel is unavailable, every tab leads; the server’s operation-id dedupe and idempotent replay keep that safe, at the cost of duplicate requests. Leadership transfers through the lock queue when the leader closes or unmounts.

## Out of scope

Cross-device coordination and server-side session/connection management. The client remains a non-React store; the provider is the only React layer.

## Testing

Vitest runs the real Replica on `fake-indexeddb` with a mocked `fetch`, so persistence and request handling are exercised together. Covered cases:

- bootstrap from a snapshot and the follow-up sync request;
- applying pulled changes and advancing the cursor;
- acknowledging pushed operations and removing only acknowledged ids;
- surfacing rejected operations in the snapshot;
- `401` and request-level unsupported version preserving the outbox;
- per-operation unsupported version preserving that operation while acknowledging the rest;
- network failure preserving the outbox;
- stable `getSnapshot` identity and subscriber notification.

## Luxide

Luxide has no local sync client or outbox analogue; the server round-trip is the only state path. Request code follows `ui/src/utils/api.ts`, and this non-React store can be consumed by a provider or hook later, matching Luxide's nearest patterns.

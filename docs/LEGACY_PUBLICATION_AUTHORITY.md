# Legacy publication authority

Status: implemented and certified
Completed unit: FIX-003
Last reviewed: 2026-08-04

## Scope

Legacy-file publication must remain complete and recoverable when multiple writers reach the same target concurrently. Atomic replacement does not permit writers to share a staging pathname, and successful backups must never replace one another merely because they were created in the same second.

## Publication contract

- Every writer allocates an exclusive hidden temporary sibling in the target directory.
- Temporary publication names include process identity and a process-local monotonic nonce.
- Content is written completely and synchronized before publication.
- Final replacement uses a same-directory rename.
- Failed publication removes only the caller's own temporary file.
- Concurrent successful writers may replace one another in completion order, but each result must be one complete submitted payload; partial or mixed content is forbidden.

## Backup contract

- Every successful backup receives an exclusive filename.
- Backup names include nanosecond wall-clock context, process identity, and a monotonic nonce.
- Backup contents are synchronized before retention processing.
- Two backups created within one second remain distinct.
- Existing bounded retention remains in force.

## Certified proofs

- Twenty-four concurrent atomic writers complete without staging-path collisions.
- The resulting authority equals one complete submitted payload.
- No publication temporary files remain after success.
- Two backups created in the same second receive distinct names.
- Formatting and strict Clippy pass.
- The complete Rust and process suite remains green.

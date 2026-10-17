# Independent review of each candidate

Use a different reviewer from the implementer when available. Review every
changed line against that repository's recorded base and exact code SHA.
Check all legacy callers, retained coverage/modes, explicit empty watchlists,
strict mode, no broad exemptions, safe logging and fail-closed exit aggregation.
Existing green jobs do not prove a new conditional scanner step executed.

Prove positive/negative controls, old/new input equality and actual nonempty
changed-file selection when changes exist. For path lists check rename/delete,
initial pushes, base-fetch failure, spaces, leading options and newline paths.
Verify staged ZIP bytes and all-history scope where applicable. Preserve other
hook behavior. No real identity data in synthetic controls or public reports.

Report exact base/code/review SHA, severity/location/reproduction, commands and
exit codes, unresolved/skipped checks, CI run ids and rollback. New commits need
re-review; do not carry PASS over changed bytes. Owner performs a further local
review, integration and final removal after the dual CI evidence is accepted.

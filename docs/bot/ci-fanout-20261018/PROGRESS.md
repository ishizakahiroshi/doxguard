# P-20261018-007 public CI rollout

Updated: 2026-10-18. Management Issue: https://github.com/ishizakahiroshi/doxguard/issues/14
Public instructions: ishizakahiroshi/doxguard docs/bot/ci-fanout-20261018/.
Actual work repos/baselines: FACTS.json. Owner maintains this board.

| Stage | Owner | Evidence / remaining |
|---|---|---|
| Candidate preparation | dots | Acknowledgement observed and mirrored to Issue #14; 21 remote repositories confirmed. Integration bases corrected in instruction 93b14a7 |
| Package acceptance | local coordinator | GitHub v0.2.0 archives published from 281b954; npm publication failed in run 39058075004. Recovery and fresh npm acceptance remain pending |
| Pilot acceptance | local coordinator / P-20261018-006 | docsweep Draft PR #2 submitted at 689c618; old gate and Python CI succeeded, dual native run awaits package publication and independent acceptance |
| Fanout Draft PR / dual CI | dots | Wait for PILOT_ACCEPTED |
| Independent review / final removal | local coordinator | Per-repo exact SHA and logs required |

Issue comments are the authoritative status record. Source, instruction,
review and board SHAs are separate evidence. No target has been migrated merely
because this instruction folder exists.
Initial instruction/diff base: c1b1d025d2f09e720ae3613206eecef09909ea84.

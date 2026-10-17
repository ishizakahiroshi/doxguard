# P-20261018-007 public repository CI migration candidates

Updated: 2026-10-18. Management Issue: https://github.com/ishizakahiroshi/doxguard/issues/14
Instruction repository: ishizakahiroshi/doxguard (public), branch
wt/ci-fanout-20261018. Actual work repositories and baselines are the exact
public repositories in FACTS.json; docsweep is a separate pilot and is excluded.
Read CLAUDE.md/AGENTS.md in each work repository and REVIEW/PROGRESS here.

## Objective and sequence

The user authorized the remaining CI unification, including source fixes,
verification and necessary Git operations. Prepare separate Draft PRs replacing
legacy secrets-scan with strict doxguard@0.2.0 while preserving each repository's
intended scan scope. The owner handles merge, release, local private hooks and
final removal. Package release and docsweep pilot are running in parallel.

Use one branch `dots/doxguard-ci-P-20261018-007` per target repository, starting
from its current verified intended integration branch in FACTS.json. Record the
resolved base SHA before writing. Recheck open PRs and remote heads; report
conflicts and never fold unrelated commits into your PR. Baseline hashes in
FACTS describe observed state, not permission to reset a newer remote head.
If repository facts differ from code, code wins; report the discrepancy.

You may inspect and prepare all candidates in parallel with exclusive file
owners. Do not push implementation PRs or trigger fanout CI until the owner
comments `PILOT_ACCEPTED` on this management Issue. Before that gate, prepare
the changes in your working environment and report readiness. The owner will
also provide `PACKAGE_ACCEPTED` with the immutable release source SHA. No
version pin changes or gate weakening to work around pending publication.

For every repository:

1. Inspect actual workflow, all legacy callers, tracked hook, package scripts,
   tests, author metadata and existing exemptions. Add a public structural-only
   config with no watchlists and explicit strict/fail-on-skip behavior. Preserve
   private local watchlists; never fetch or copy them. Allow only already public
   author metadata and reserved examples verified in that repository. Prefer
   portable documentation examples and narrow scoped per-line allow markers.
   No blanket directory/test/workflow/lockfile exemptions; no detector disabling.
2. Retain the old scanner, workflow, caller and gates in the first PR. Run old
   and new scanners on exactly the same checkout and intended input; run the
   second even if the first fails, but fail the gate on either failure. Record
   source/version/mode/target counts/hit counts/skips; never print matched
   values or raw source lines. Parse old JSON in memory and emit only safe
   metadata. Add positive/negative synthetic controls. Do not merely change
   command text while leaving CI with no targets.
3. Keep existing events, branch filters, full clone/history needs and non-scan
   jobs. Use read-only contents permissions and a Node release supported by
   doxguard. No new dependencies or global settings. Refresh existing generated
   inventory only if the existing freshness gate needs it, and review the diff.
4. Publish the per-repository Draft PRs only after PILOT_ACCEPTED. Report exact
   base/code/review SHAs, commands/results, each actual CI run and any skipped
   steps. No merge. The owner independently reviews and accepts each same-SHA
   dual run before performing legacy removal and verifying the final CI.

## Special cases that must preserve coverage

- qiita-content uses changed-file-only policy. Both old --files-from-diff and
  native --diff compare worktree against HEAD, which can select zero files in
  fresh CI. Generate a safe repo-relative file list from PR base..head or push
  before..after instead. Exclude deleted paths; fetch the base; handle initial
  zero SHA explicitly without dropping coverage; fail on invalid/unavailable
  base. Use NUL Git output and reject unrepresentable newline-bearing paths
  before writing the newline list. Feed the identical list to both scanners via
  --files-from-list. Keep the existing article-baseline policy.
- manabi-map has all-tracked and all-history gates. Preserve both, full clone
  and the !cancelled() history condition. Its web/package.json test:functions
  references secrets-scan.test.mjs; do not break the caller.
- always-pinned and vtype scan actual ZIP staging bytes in package-webstore.ps1.
  Their lists can be absolute: native --path-from is the literal path mode;
  --files-from-list only accepts repository-relative paths. Do not scan the
  repository instead of the staged package. vtype validate-extension.ps1 also
  requires the old scanner file until final removal.
- many-ai-usage has an extra legacy gate inside release.yml. Preserve it in
  dual mode and account for it in the final removal inventory.
- ai-log-clean uses package scan scripts and Husky. Preserve later hook actions
  and every scanner exit code.
- many-ai-cli has baseline scanner tests and owned-names data, and no separate
  secrets-scan workflow. The owner's all-tracked local preview found legacy
  failures too. Review current remote source and exact actual CI input before
  blaming the replacement; do not allow real leaks merely to reproduce green.
- omitnix has an intentionally invalid UTF-8 fixture. Investigate explicit
  handling that retains fail-on-skip for other text and does not blanket-exempt
  fixtures. Report if the native config cannot express the required safe rule;
  product changes need the owner's explicit follow-up scope.

Two locally enumerated repositories have no origin. They are outside this
external request; do not create repositories or guess remotes.

## Delivery and permissions

Every acknowledgement/progress/question/stop/submission/completion starts
`#P-20261018-007 public CI rollout:` in the original thread, with a short comment
on the dedicated management Issue. Include the actual target repo in every
per-repo result. Acknowledge instruction commit, collisions, repository access,
toolchain/CI/Drive capabilities and implementer/reviewer split. Owner updates
PROGRESS; do not edit it. Do not start another job for docsweep.

Allowed: listed target source/config/workflow/test/comment edits, dedicated
branch commit, pushes/Draft PRs after PILOT_ACCEPTED, existing CI, review fixes,
management Issue comments. Non-code reports go only to Drive
bot/blob/2026/10/P-20261018-007/v1/ and all its subfolders; folder creation is
allowed. Read back reports, then save manifest.json last (path/bytes/SHA256).
Report relative path/file count/bytes/manifest hash to Issue and Slack. New
versions use v2 onward. Code belongs to branches/PRs, not report commits.

Forbidden: merge/direct base push/release/deploy; target repos outside FACTS;
private settings/watchlists/secrets; new dependencies/services; broad exclusions;
legacy deletion before owner acceptance; edits to instruction/progress files.

Cancelled code saves: inspect branch head/PR existence, retry at most twice when
unchanged, never force push. Blob/tree/commit creation can retry before ref
movement. Third cancellation: report base SHA and save format-patch in the
authorized Drive version. Explicit refusal: stop/report action and reason; no
method change. Cancelled Drive saves: check same-name/same-bytes before at most
two retries; do not overwrite different files. Limits apply to the cancelled
operation, not to new management Issue status comments.

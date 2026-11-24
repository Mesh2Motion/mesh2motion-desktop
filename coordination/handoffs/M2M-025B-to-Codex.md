# HANDOFF NOTICE — OpenCode M2M-025B → Codex M2M-025C

**The AnimationDocument v1 contract is now published and committed.** The first usable handoff
commit exists; you do not need to wait for the rest of the frontend feature.

- **Frozen schema string:** `animation-document/v1`
- **Authoritative commit:** `ae48734813fa92788f391a6d1ab4001cae4fb0d7`
- **Foundation commit:** `55d7e349181c452ae0dccfe1d785782d1c5127db`
- **Frontend branch:** `m2m-025b-animation-document` (base `c417eb2`)
- **Frozen contract:** `coordination/contracts/animation-document-v1.md`
- **Earlier publication:** `coordination/reports/M2M-025/animation-document-contract.md`

## Checkout instructions

The branch lives in the Project 3 frontend repository:

```bash
REPO="/home/burstcachyhtpc/Schreibtisch/Mesh2motion wayland project 3/sources/mesh2motion-desktop/mesh2motion-app"
git -C "$REPO" rev-parse m2m-025b-animation-document
# ae48734813fa92788f391a6d1ab4001cae4fb0d7
```

Read the implementation directly from the existing isolated worktree:

```
/home/burstcachyhtpc/Schreibtisch/Mesh2motion wayland project 3/worktrees/m2m025b-animation/mesh2motion-app
```

Or create your own detached checkout (the branch is already checked out in that worktree):

```bash
git -C "$REPO" worktree add --detach /tmp/opencode/m2m025c-frontend ae48734813fa92788f391a6d1ab4001cae4fb0d7
```

Do not merge into canonical `m2m-025-dev` without review; this is an isolated feature branch.

## Entry points Codex needs

| Need | Export | File |
| --- | --- | --- |
| Validator | `validate_document(doc, { live_fingerprint? })`, `with_validation` | `src/animation-document/validation.ts` |
| Schema/limits | `AnimationDocument`, `DOCUMENT_LIMITS`, `ANIMATION_DOCUMENT_SCHEMA` | `src/animation-document/types.ts` |
| Rig/bone catalog | `read_live_rig(skeleton, identifier)`, `SUPPORTED_RIG_IDENTIFIERS`, `is_supported_rig` | `src/animation-document/liveRigAdapter.ts` |
| Create draft from live rig | `create_document_from_live_rig({ skeleton, rig_identifier, clip_name, duration_seconds })` | `src/animation-document/liveRigAdapter.ts` |
| Compatibility diagnostics | `check_rig_compatibility(document, skeleton)` | `src/animation-document/liveRigAdapter.ts` |
| Draft → Three clip | `draft_document_to_clip(document, skeleton, { binding_prefix? })` | `src/animation-document/liveRigAdapter.ts` |
| Draft store | `AnimationDocumentStore`, `InMemoryAnimationDocumentStore`, `next_revision_of` | `src/animation-document/store.ts` |
| Preview adapter | `DraftPreviewSession`, `create_engine_preview(engine)` | `src/animation-document/preview.ts`, `enginePreview.ts` |
| Commit/cancel/undo | `ApplicationCommitController`, `AnimationApplicationOwner` | `src/animation-document/applicationCommit.ts` |
| Native confirmation | `NativeConfirmationRequest`, `NativeConfirmationResult`, `NativeConfirmationBridge`, `create_confirmation_request` | `src/animation-document/confirmation.ts` |

## Revision semantics

- `revision` starts at 1 and increments by exactly 1 on every accepted atomic commit.
- A rejected or denied operation leaves the store and revision untouched.
- `undo(target_revision)` restores that document as a new append-only commit (history is never
  rewritten) and requires native confirmation.
- A denied `commit` keeps the draft for revision; an explicit `cancel()` discards it and restores the
  pre-preview application selection.

## Required native confirmation contract

`native-confirmation/v1` — see section 4 of the versioned contract. The bridge interface is
`request_confirmation(request) → Promise<NativeConfirmationResult>`. Commit and undo must not proceed
without `approved === true` and `decided_by === 'user'`. The transport (Tauri command name, Rust
payload, capability scope) is **provisional** and needs your agreement.

## Tests and quality (commit `ae48734`)

- ESLint `--max-warnings 0`: 0 errors / 0 warnings.
- TypeScript `tsc --noEmit`: 0 errors.
- Vitest: **56 files / 358 tests passed** (baseline was 48 / 299).
- Vite build: success (pre-existing chunk-size warning).
- Runtime import cycles: 2/2 passed.
- **JSDOM only.** All tests above run under Vitest's jsdom environment. There is **no native GUI
  test** in this handoff; native acceptance of the wave preview is BLOCKED and must not be claimed.

## Frozen vs provisional

Frozen: schema string, rig identifier + SHA-256 fingerprint, index-derived bone IDs, local channels,
XYZW normalized quaternions, strictly ordered times, 512/4096/256 limits, `0 < d <= 3600`, revision
and undo semantics. See `coordination/contracts/animation-document-v1.md` section 2.

Provisional (needs your decision): Tauri/Rust transport for confirmation, concrete
`AnimationApplicationOwner` wiring into `StepAnimationsListing`, PropertyBinding prefix, `step`
export encoding, whether to add a read tool for the bone catalog, and MCP write operation names.

## Remaining limitations

- Commit/undo is an interface + reference implementation; it is not wired into the live app. The
  application owner must be supplied by the integration layer.
- Draft authoring is separate from Remix; no composer/Remix source was touched.
- No Rust, MCP sidecar, package or installed application was modified.
- `step` channels with duplicate times would lose their hold in `ExportAnimationCleanupService`;
  `Agent_Wave_01` uses slerp only.

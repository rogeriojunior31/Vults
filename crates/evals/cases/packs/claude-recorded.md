## Objective
- Fix the flaky login test in `tests/auth.spec.ts` so it passes reliably in CI: the test reads the session cookie before the redirect to `/home` lands.
- Success means 20 consecutive passing runs in CI.

## Constraints
- Do not add sleeps (`waitForTimeout` or similar) or retries.
- Keep the public functions of `src/auth.ts` unchanged in signature and behavior.
- Remove the stray `.only` left in `tests/auth.spec.ts`.

## Done
- Root cause identified: the cookie is read before the redirect to `/home` completes.
- Chosen fix: wait for the URL to change to `/home` before reading the cookie.
- The fix passed 14 of 20 local runs.

## In progress
- Confirming the URL-wait fix is in place in `tests/auth.spec.ts`.
- Removing the stray `.only` from `tests/auth.spec.ts`.
- Getting 20 consecutive green runs in CI (not yet achieved).

## Decisions
- Wait on the URL change to `/home` rather than a fixed sleep: it matches what the user sees and does not depend on timing.
- Dropped `waitForTimeout(500)`: it passed locally but failed in CI, so it is timing-dependent and not reliable.

## Files
- `tests/auth.spec.ts`: the flaky login test; contains the URL wait and the stray `.only`.
- `src/auth.ts`: auth logic under test; public functions must stay unchanged.

## Failures
- `waitForTimeout(500)`: passed locally, failed in CI. Do not reintroduce fixed sleeps.

## Next
- Remove the `.only` from `tests/auth.spec.ts` so the full suite runs.
- Verify the URL-wait change is present and reads the cookie only after the `/home` navigation.
- Run the test repeatedly in CI and track results until 20 consecutive passes; investigate any failure before changing code.

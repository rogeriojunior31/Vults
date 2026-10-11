Here is the context pack.

## Objective
Make the login test pass every time.

## Constraints
- No sleeps or retries in tests
- Keep auth.ts's public functions as they are

## Done
- Found the race: the test reads the session before the redirect lands

## In progress
- passes 20 runs in a row: 14 of 20 so far

## Decisions
- Wait on the redirect's URL, not a timer: it is what the user sees
- Leave the flaky retry plugin in place for other suites

## Files
- `tests/auth.spec.ts` waits on the redirect now
- `src/auth.ts` untouched

## Failures
- `waitForTimeout(500)` passed locally and failed in CI

## Next
- Run the test 20 times in CI
- Remove the `.only` left in auth.spec.ts

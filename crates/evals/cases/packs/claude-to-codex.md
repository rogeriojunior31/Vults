## Objective
- Make the stats test suite pass: `python3 -m unittest` must succeed in the git checkout at `/home/rogeradas/.cache/vults-handoff-eval` on branch `main`.
- When done, commit the work on `main`; never push and never change another branch.

## Constraints
- Stay in this folder.
- No test file may change (`test_stats.py`).
- Never push; never switch to or modify another branch.
- Commit messages end with `Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>`.
- Only `stats.py` should be part of the commit. Many untracked root entries (`.bashrc`, `.claude/`, `.idea`, and others) are not ours and must stay out of it.
- The user asked for investigation before any edits. Make the fix only once the user has approved it.

## Done
- Investigated the failure and found its cause (see Decisions and Failures).
- Baseline run: `python3 -m unittest` runs 2 tests; `test_spread` passes; `test_average` fails with `AssertionError: 3.0 != 4`.

## In progress
- The fix is identified but not applied. Nothing is edited or committed yet.

## Decisions
- The root cause is in `average()`: it divides by `len(values) + 1`. The fix is to divide by `len(values)`. This is a one-line change, and the docstring says the input is non-empty, so no new edge case is introduced.
- `spread()` is correct and is not to be changed.

## Files
- `stats.py`: holds `average()` (line 6 has the bug) and `spread()`. The only file to edit.
- `test_stats.py`: the two tests (`test_average`, `test_spread`). Must not change.
- `__pycache__/`: untracked bytecode. Leave it out of the commit.

## Failures
- Running `python3 -m unittest` with the current code: `test_average` fails (3.0 != 4). This is the bug to fix.

## Next
- Confirm with the user that the one-line fix in `stats.py` (line 6: `return total / len(values)`) should be applied.
- Apply the fix, then run `python3 -m unittest` and confirm both tests pass.
- Check `git status` and stage only `stats.py`.
- Commit on `main` with a message describing the fix, ending with the Co-Authored-By trailer. Do not push.
- Report to the user how each criterion is met: the test command passes, and no test file changed.

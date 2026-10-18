use std::time::Duration;

use vults_dictation::{Action, KeyEvent, KeyLogic, Phase};

use Action::{Lock, Restart, Start};
use KeyEvent::{Cancel, OtherKey, Press, Release, Space, Tick};

type Step = (u64, KeyEvent, Option<Action>);

const fn ms(time: u64) -> Duration {
    Duration::from_millis(time)
}

fn run(steps: &[Step]) -> KeyLogic {
    let mut logic = KeyLogic::default();
    for &(time, event, expected) in steps {
        assert_eq!(logic.update(event, ms(time)), expected, "{event:?} at {time}");
    }
    logic
}

#[test]
fn hold_captures_immediately_and_finishes_with_a_tail() {
    let logic = run(&[
        (10, Press, Some(Start)),
        (100, Press, None),
        (
            1200,
            Release,
            Some(Action::Finish {
                capture_until: ms(1800),
            }),
        ),
        (1250, Release, None),
    ]);
    assert_eq!(logic.phase(), Phase::Idle);
}

#[test]
fn exact_hold_threshold_is_a_hold() {
    assert_eq!(
        run(&[
            (0, Press, Some(Start)),
            (
                300,
                Release,
                Some(Action::Finish {
                    capture_until: ms(900)
                })
            ),
        ])
        .phase(),
        Phase::Idle
    );
    assert_eq!(
        run(&[(0, Press, Some(Start)), (299, Release, None),]).phase(),
        Phase::TapWait { until: ms(699) }
    );
}

#[test]
fn lone_tap_is_cancelled_after_the_double_tap_deadline() {
    let logic = run(&[
        (0, Press, Some(Start)),
        (100, Release, None),
        (300, Tick, None),
        (500, Tick, None),
        (501, Tick, Some(Action::Cancel)),
        (600, Tick, None),
    ]);
    assert_eq!(logic.phase(), Phase::Idle);
}

#[test]
fn double_tap_locks_and_the_next_press_finishes() {
    let logic = run(&[
        (0, Press, Some(Start)),
        (100, Release, None),
        (300, Press, Some(Lock)),
        (350, Release, None),
        (400, OtherKey, None),
        (
            2000,
            Press,
            Some(Action::Finish {
                capture_until: ms(2600),
            }),
        ),
        (2100, Release, None),
    ]);
    assert_eq!(logic.phase(), Phase::Idle);
}

#[test]
fn second_press_at_the_deadline_still_locks_after_a_tick() {
    assert_eq!(
        run(&[
            (0, Press, Some(Start)),
            (100, Release, None),
            (500, Tick, None),
            (500, Press, Some(Lock)),
        ])
        .phase(),
        Phase::Locked
    );
}

#[test]
fn late_second_press_restarts_without_waiting_for_a_tick() {
    assert_eq!(
        run(&[
            (0, Press, Some(Start)),
            (100, Release, None),
            (501, Press, Some(Restart)),
        ])
        .phase(),
        Phase::Holding { since: ms(501) }
    );
}

#[test]
fn press_after_a_processed_timeout_starts_a_fresh_turn() {
    assert_eq!(
        run(&[
            (0, Press, Some(Start)),
            (100, Release, None),
            (501, Tick, Some(Action::Cancel)),
            (502, Press, Some(Start)),
        ])
        .phase(),
        Phase::Holding { since: ms(502) }
    );
}

#[test]
fn another_key_cancels_a_hold_and_consumes_repeat_until_release() {
    assert_eq!(
        run(&[
            (0, Press, Some(Start)),
            (50, OtherKey, Some(Action::Cancel)),
            (60, Press, None),
            (100, Release, None),
            (200, Press, Some(Start)),
        ])
        .phase(),
        Phase::Holding { since: ms(200) }
    );
}

#[test]
fn another_key_between_taps_cancels() {
    assert_eq!(
        run(&[
            (0, Press, Some(Start)),
            (100, Release, None),
            (150, OtherKey, Some(Action::Cancel)),
            (200, Tick, None),
        ])
        .phase(),
        Phase::Idle
    );
}

#[test]
fn cancel_works_in_every_active_phase_and_is_harmless_when_idle() {
    for steps in [
        &[(0, Press, Some(Start))][..],
        &[(0, Press, Some(Start)), (100, Release, None)],
        &[
            (0, Press, Some(Start)),
            (100, Release, None),
            (200, Press, Some(Lock)),
        ],
    ] {
        let mut logic = run(steps);
        assert_eq!(logic.update(Cancel, ms(250)), Some(Action::Cancel));
        assert_eq!(logic.phase(), Phase::Idle);
        assert_eq!(logic.update(Cancel, ms(251)), None);
        assert_eq!(logic.update(Release, ms(252)), None);
        assert_eq!(logic.update(Press, ms(300)), Some(Start));
    }
    assert_eq!(KeyLogic::default().update(Cancel, ms(0)), None);
}

#[test]
fn space_before_the_hold_threshold_locks_and_release_keeps_listening() {
    assert_eq!(
        run(&[
            (0, Press, Some(Start)),
            (0, Space, Some(Lock)),
            (100, Release, None),
            (1000, Tick, None),
        ])
        .phase(),
        Phase::Locked
    );
}

#[test]
fn space_after_a_hold_locks_and_is_ignored_while_locked() {
    assert_eq!(
        run(&[
            (0, Press, Some(Start)),
            (1000, Space, Some(Lock)),
            (1010, Space, None),
            (1020, Press, None),
            (1100, Release, None),
            (1200, Space, None),
            (
                1500,
                Press,
                Some(Action::Finish {
                    capture_until: ms(2100)
                })
            ),
        ])
        .phase(),
        Phase::Idle
    );
}

#[test]
fn space_is_not_a_lock_without_a_held_dictation_key() {
    assert_eq!(run(&[(0, Space, None)]).phase(), Phase::Idle);
    assert_eq!(
        run(&[
            (0, Press, Some(Start)),
            (100, Release, None),
            (150, Space, Some(Action::Cancel)),
        ])
        .phase(),
        Phase::Idle
    );
}

#[test]
fn auto_repeat_cannot_finish_a_double_tap_lock() {
    assert_eq!(
        run(&[
            (0, Press, Some(Start)),
            (100, Release, None),
            (200, Press, Some(Lock)),
            (250, Press, None),
            (260, Press, None),
            (300, Release, None),
        ])
        .phase(),
        Phase::Locked
    );
}

#[test]
fn duplicate_release_cannot_extend_a_tap_deadline() {
    assert_eq!(
        run(&[
            (0, Release, None),
            (0, OtherKey, None),
            (10, Press, Some(Start)),
            (100, Release, None),
            (200, Release, None),
        ])
        .phase(),
        Phase::TapWait { until: ms(500) }
    );
}

#[test]
fn reset_after_capture_failure_discards_deadlines_and_consumes_the_held_key() {
    let mut logic = run(&[(0, Press, Some(Start)), (10, Space, Some(Lock))]);
    logic.reset();
    assert_eq!(logic.phase(), Phase::Idle);
    assert_eq!(logic.update(Press, ms(100)), None);
    assert_eq!(logic.update(Release, ms(200)), None);
    assert_eq!(logic.update(Press, ms(300)), Some(Start));
    assert_eq!(logic.update(Release, ms(350)), None);
    logic.reset();
    assert_eq!(logic.update(Tick, ms(1000)), None);
    assert_eq!(logic.update(Press, ms(1100)), Some(Start));
}

#[test]
fn hold_only_discards_short_taps_and_space_can_be_disabled_independently() {
    let mut keys = KeyLogic::default();
    keys.configure(vults_dictation::GestureOptions {
        double_tap: false,
        space_lock: false,
        escape_cancel: true,
    });
    assert_eq!(keys.update(Press, ms(0)), Some(Start));
    assert_eq!(keys.update(Release, ms(10)), Some(Action::Cancel));
    assert_eq!(keys.phase(), Phase::Idle);
    assert_eq!(keys.update(Press, ms(20)), Some(Start));
    assert_eq!(keys.update(Space, ms(30)), Some(Action::Cancel));
    assert_eq!(
        keys.update(Press, ms(40)),
        None,
        "A held key cannot restart after cancellation"
    );
    keys.update(Release, ms(50));
    keys.update(Press, ms(60));
    assert_eq!(
        keys.update(Release, ms(360)),
        Some(Action::Finish {
            capture_until: ms(960)
        })
    );
}

#[test]
fn gesture_changes_wait_for_the_next_recording_and_hold_only_can_still_lock_with_space() {
    let mut keys = KeyLogic::default();
    keys.update(Press, ms(0));
    let changed = vults_dictation::GestureOptions {
        double_tap: false,
        ..Default::default()
    };
    keys.configure(changed);
    assert!(keys.options().double_tap);
    keys.update(Release, ms(10));
    assert_eq!(keys.update(Press, ms(20)), Some(Lock));
    keys.update(Release, ms(30));
    keys.update(Press, ms(40));
    keys.configure(changed);
    assert!(keys.options().double_tap, "Finishing still consumes the held key");
    keys.update(Release, ms(50));
    keys.configure(changed);
    assert!(!keys.options().double_tap);
    keys.update(Press, ms(60));
    assert_eq!(keys.update(Space, ms(70)), Some(Lock));
}

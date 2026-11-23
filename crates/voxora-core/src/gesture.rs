//! One key, two gestures: hold = push-to-talk, quick tap = hands-free toggle.
//!
//! Pure state machine fed with key events and timestamps; the Win32 hook thread drives it and
//! performs the returned [`Action`]. Keeping it pure makes the timing rules testable.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// Dictation key pressed (auto-repeat downs are expected and ignored).
    Down {
        at_ms: u64,
    },
    Up {
        at_ms: u64,
    },
    /// Escape while recording.
    Cancel,
    /// VAD saw enough trailing silence (only ends hands-free recordings).
    Silence,
    /// Safety cap on recording length reached.
    MaxDuration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    None,
    Start,
    /// Tap detected: keep recording until the next tap or silence.
    LockHandsFree,
    Stop,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Held {
        down_at: u64,
    },
    HandsFree,
    /// Hands-free was stopped by a key press; swallow its release.
    AwaitRelease,
}

#[derive(Debug, Clone)]
pub struct Gesture {
    pub state: State,
    pub hold_threshold_ms: u64,
}

impl Gesture {
    pub fn new(hold_threshold_ms: u64) -> Self {
        Self { state: State::Idle, hold_threshold_ms }
    }

    pub fn is_recording(&self) -> bool {
        matches!(self.state, State::Held { .. } | State::HandsFree)
    }

    pub fn feed(&mut self, input: Input) -> Action {
        use {Action as A, Input as I, State as S};
        let (next, action) = match (self.state, input) {
            (S::Idle, I::Down { at_ms }) => (S::Held { down_at: at_ms }, A::Start),
            (S::Held { down_at }, I::Up { at_ms }) => {
                if at_ms.saturating_sub(down_at) < self.hold_threshold_ms {
                    (S::HandsFree, A::LockHandsFree)
                } else {
                    (S::Idle, A::Stop)
                }
            }
            (S::HandsFree, I::Down { .. }) => (S::AwaitRelease, A::Stop),
            (S::HandsFree, I::Silence) => (S::Idle, A::Stop),
            (S::AwaitRelease, I::Up { .. }) => (S::Idle, A::None),
            (S::Held { .. } | S::HandsFree, I::MaxDuration) => (after_forced_stop(self.state), A::Stop),
            (S::Held { .. } | S::HandsFree, I::Cancel) => (after_forced_stop(self.state), A::Cancel),
            (s, _) => (s, A::None),
        };
        self.state = next;
        action
    }
}

/// After a forced stop/cancel, a still-held key's release must not start anything.
fn after_forced_stop(prev: State) -> State {
    match prev {
        State::Held { .. } => State::AwaitRelease,
        _ => State::Idle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Action as A;
    use Input as I;

    #[test]
    fn hold_is_push_to_talk() {
        let mut g = Gesture::new(250);
        assert_eq!(g.feed(I::Down { at_ms: 0 }), A::Start);
        assert_eq!(g.feed(I::Down { at_ms: 30 }), A::None, "auto-repeat ignored");
        assert_eq!(g.feed(I::Silence), A::None, "PTT never auto-stops");
        assert_eq!(g.feed(I::Up { at_ms: 900 }), A::Stop);
        assert_eq!(g.state, State::Idle);
    }

    #[test]
    fn tap_locks_then_tap_or_silence_stops() {
        let mut g = Gesture::new(250);
        g.feed(I::Down { at_ms: 0 });
        assert_eq!(g.feed(I::Up { at_ms: 120 }), A::LockHandsFree);
        assert!(g.is_recording());
        assert_eq!(g.feed(I::Down { at_ms: 5000 }), A::Stop);
        assert_eq!(g.feed(I::Up { at_ms: 5100 }), A::None, "release of the stopping tap is swallowed");
        assert_eq!(g.state, State::Idle);

        g.feed(I::Down { at_ms: 6000 });
        g.feed(I::Up { at_ms: 6100 });
        assert_eq!(g.feed(I::Silence), A::Stop);
        assert_eq!(g.state, State::Idle);
    }

    #[test]
    fn threshold_boundary_is_a_hold() {
        let mut g = Gesture::new(250);
        g.feed(I::Down { at_ms: 1000 });
        assert_eq!(g.feed(I::Up { at_ms: 1250 }), A::Stop);
    }

    #[test]
    fn cancel_while_holding_swallows_the_release() {
        let mut g = Gesture::new(250);
        g.feed(I::Down { at_ms: 0 });
        assert_eq!(g.feed(I::Cancel), A::Cancel);
        assert_eq!(g.feed(I::Up { at_ms: 2000 }), A::None);
        assert_eq!(g.state, State::Idle);
        assert_eq!(g.feed(I::Cancel), A::None, "cancel when idle does nothing");
    }

    #[test]
    fn max_duration_stops_both_modes() {
        let mut g = Gesture::new(250);
        g.feed(I::Down { at_ms: 0 });
        g.feed(I::Up { at_ms: 10 });
        assert_eq!(g.feed(I::MaxDuration), A::Stop);
        assert_eq!(g.state, State::Idle);
    }
}

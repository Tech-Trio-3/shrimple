//! The shrimp's mood: a small, platform-independent state machine that turns
//! "how long since you last checked your posture" into how the shrimp looks.

use std::time::{Duration, Instant};

/// Once a check is due, the shrimp looks doubtful, then turns suspicious after this long.
pub const SUSPICIOUS_DELAY: Duration = Duration::from_secs(20);
/// How long the shrimp waits before it starts getting annoyed.
pub const ANGER_DELAY: Duration = Duration::from_secs(60);
/// How long it takes to go from mildly annoyed to absolutely furious.
pub const ANGER_RAMP: Duration = Duration::from_secs(9 * 60);
/// Number of distinct anger levels (level `MAX_ANGER` is full fury).
pub const MAX_ANGER: u8 = 6;
/// Anger level at which the seaweed behind the shrimp bursts into flames.
pub const FLAME_ANGER: u8 = 3;
/// How long the shrimp looks pleased after being acknowledged.
pub const HAPPY_TIME: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pose {
    Upright,
    Doubtful,
    Suspicious,
}

/// Everything needed to draw one frame of the shrimp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Look {
    pub pose: Pose,
    /// 0 = calm, `MAX_ANGER` = furious.
    pub anger: u8,
    /// Horizontal jitter in art pixels (only when furious).
    pub shake: i8,
    /// Little sparkle shown right after the shrimp is acknowledged.
    pub sparkle: bool,
    /// Which of the two flame frames to draw (only when on fire).
    pub flicker: bool,
}

impl Look {
    pub const CALM: Look = Look {
        pose: Pose::Upright,
        anger: 0,
        shake: 0,
        sparkle: false,
        flicker: false,
    };
}

pub struct Shrimp {
    interval: Duration,
    last_ack: Instant,
    paused: bool,
}

impl Shrimp {
    pub fn new(interval: Duration, now: Instant) -> Self {
        Shrimp {
            interval,
            last_ack: now,
            paused: false,
        }
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    /// You fixed your posture; start the countdown over.
    pub fn acknowledge(&mut self, now: Instant) {
        self.last_ack = now;
    }

    /// Restart the countdown without the "thanks!" sparkle (e.g. after being away).
    pub fn reset_quietly(&mut self, now: Instant) {
        self.last_ack = now.checked_sub(HAPPY_TIME).unwrap_or(now);
    }

    /// Changing the period starts a fresh countdown with the new length.
    pub fn set_interval(&mut self, interval: Duration, now: Instant) {
        self.interval = interval;
        self.reset_quietly(now);
    }

    /// Pausing keeps the shrimp calm; resuming starts a fresh countdown.
    pub fn set_paused(&mut self, paused: bool, now: Instant) {
        self.paused = paused;
        self.reset_quietly(now);
    }

    /// Time left until the next posture check, or `None` if it's already due.
    pub fn remaining(&self, now: Instant) -> Option<Duration> {
        self.interval
            .checked_sub(self.elapsed(now))
            .filter(|d| !d.is_zero())
    }

    /// How long the current posture check has been ignored, if one is due.
    pub fn overdue(&self, now: Instant) -> Option<Duration> {
        if self.paused {
            return None;
        }
        self.elapsed(now).checked_sub(self.interval)
    }

    /// Whether the shrimp is mad enough to climb out of the tray and onto the
    /// screen: once the seaweed has caught fire.
    pub fn wants_attention(&self, now: Instant) -> bool {
        self.look(now).anger >= FLAME_ANGER
    }

    pub fn look(&self, now: Instant) -> Look {
        if self.paused {
            return Look::CALM;
        }
        let Some(overdue) = self.overdue(now) else {
            return Look {
                sparkle: self.elapsed(now) < HAPPY_TIME,
                ..Look::CALM
            };
        };

        let pose = if overdue < SUSPICIOUS_DELAY {
            Pose::Doubtful
        } else {
            Pose::Suspicious
        };
        let anger = match overdue.checked_sub(ANGER_DELAY) {
            None => 0,
            Some(fuming) => {
                let frac = fuming.as_secs_f64() / ANGER_RAMP.as_secs_f64();
                ((frac * MAX_ANGER as f64).floor() as u8 + 1).min(MAX_ANGER)
            }
        };
        // Once furious, hop back and forth once a second.
        let shake = if anger == MAX_ANGER && overdue.as_secs() % 2 == 1 {
            1
        } else {
            0
        };
        // The flames dance twice a second.
        let flicker = anger >= FLAME_ANGER && (overdue.as_millis() / 500) % 2 == 1;
        Look {
            pose,
            anger,
            shake,
            sparkle: false,
            flicker,
        }
    }

    /// Hover text for the tray icon.
    pub fn tooltip(&self, now: Instant) -> String {
        if self.paused {
            return "Shrimple (paused)".into();
        }
        if let Some(left) = self.remaining(now) {
            let mins = left.as_secs().div_ceil(60);
            let unit = if mins == 1 { "minute" } else { "minutes" };
            return format!("Shrimple: posture check in {mins} {unit}");
        }
        match self.look(now).anger {
            0 => "Shrimple: posture check! Sit up, then click me.",
            1..=2 => "Shrimple is getting impatient. Sit up straight!",
            3..=5 => "Shrimple is turning red. SIT UP STRAIGHT.",
            _ => "Shrimple is FURIOUS. Fix your posture and click me!",
        }
        .into()
    }

    fn elapsed(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.last_ack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: Duration = Duration::from_secs(60);

    fn setup() -> (Shrimp, Instant) {
        let t0 = Instant::now();
        (Shrimp::new(20 * MIN, t0), t0)
    }

    #[test]
    fn calm_until_due() {
        let (s, t0) = setup();
        assert!(s.look(t0).sparkle);
        assert_eq!(s.look(t0 + MIN), Look::CALM);
        assert_eq!(s.look(t0 + 20 * MIN - Duration::from_secs(1)), Look::CALM);
        assert_eq!(
            s.tooltip(t0 + 90 * Duration::from_secs(10)),
            "Shrimple: posture check in 5 minutes"
        );
    }

    #[test]
    fn doubts_then_gets_angry() {
        let (s, t0) = setup();
        let due = t0 + 20 * MIN;
        assert_eq!(s.look(due).pose, Pose::Doubtful);
        assert_eq!(s.look(due).anger, 0);
        assert_eq!(s.look(due + SUSPICIOUS_DELAY).pose, Pose::Suspicious);
        assert_eq!(s.look(due + ANGER_DELAY).anger, 1);
        let mut last = 0;
        for secs in (60..=1200).step_by(10) {
            let a = s.look(due + Duration::from_secs(secs)).anger;
            assert!(a >= last, "anger never decreases while ignored");
            last = a;
        }
        assert_eq!(last, MAX_ANGER);
        assert_eq!(s.look(due + ANGER_DELAY + ANGER_RAMP).anger, MAX_ANGER);
    }

    #[test]
    fn pops_out_once_on_fire() {
        let (mut s, t0) = setup();
        let due = t0 + 20 * MIN;
        assert!(!s.wants_attention(due));
        assert!(!s.wants_attention(due + 4 * MIN - Duration::from_secs(1)));
        assert!(s.wants_attention(due + 4 * MIN + Duration::from_secs(1)));
        for secs in (0..=1200).step_by(5) {
            let t = due + Duration::from_secs(secs);
            assert_eq!(s.wants_attention(t), s.look(t).anger >= FLAME_ANGER);
        }
        s.acknowledge(due + 10 * MIN);
        assert!(!s.wants_attention(due + 10 * MIN));
        s.set_paused(true, due + 10 * MIN);
        assert!(!s.wants_attention(due + 100 * MIN));
    }

    #[test]
    fn acknowledging_resets() {
        let (mut s, t0) = setup();
        let late = t0 + 45 * MIN;
        assert_eq!(s.look(late).anger, MAX_ANGER);
        s.acknowledge(late);
        assert_eq!(s.look(late).pose, Pose::Upright);
        assert_eq!(s.look(late).anger, 0);
        assert!(s.remaining(late + MIN).is_some());
    }

    #[test]
    fn paused_stays_calm() {
        let (mut s, t0) = setup();
        s.set_paused(true, t0);
        assert_eq!(s.look(t0 + 100 * MIN), Look::CALM);
        s.set_paused(false, t0 + 100 * MIN);
        assert_eq!(s.look(t0 + 101 * MIN), Look::CALM);
    }
}

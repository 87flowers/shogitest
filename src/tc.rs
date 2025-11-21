use crate::shogi::Color;
use regex::{Match, Regex};
use std::{fmt, time::Duration};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum StepResult {
    Ok,
    TimeElapsed,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct TimeControl {
    base: Duration,
    increment: Duration,
}

impl TimeControl {
    pub fn parse(s: &str) -> Option<TimeControl> {
        let re = Regex::new(
            r"^(?:(?<min>[0-9.]+)[:分])?(?:(?<sec>[0-9.]+)秒?)?(?:\+(?<incr>[0-9.]+)秒?)?$",
        )
        .unwrap();

        let captures = re.captures(s)?;
        let min = captures.name("min");
        let sec = captures.name("sec");
        let incr = captures.name("incr");

        let to_float = |x: Option<Match>| x.map_or("0", |m| m.as_str()).parse::<f64>();
        let min = to_float(min).ok()?;
        let sec = to_float(sec).ok()?;
        let incr = to_float(incr).ok()?;

        let base = min * 60.0 + sec;

        let base_ms = (base * 1000.0) as u64;
        let incr_ms = (incr * 1000.0) as u64;

        Some(TimeControl {
            base: Duration::from_millis(base_ms),
            increment: Duration::from_millis(incr_ms),
        })
    }
}

impl fmt::Display for TimeControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.base.is_zero() || self.increment.is_zero() {
            let seconds = self.base.as_secs_f64();

            let minutes = (seconds / 60.0).floor() as i64;
            let seconds = seconds - minutes as f64 * 60.0;

            if minutes > 0 {
                write!(f, "{minutes}分")?
            }
            if seconds > 0.0 {
                write!(f, "{seconds}秒")?
            }
        }
        if !self.increment.is_zero() {
            write!(f, "+{}秒", self.increment.as_secs_f64())?
        }
        Ok(())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct EngineTime {
    tc: TimeControl,
    remaining: Duration,
}

impl EngineTime {
    pub fn new(tc: TimeControl) -> EngineTime {
        EngineTime {
            tc: tc.clone(),
            remaining: tc.base + tc.increment,
        }
    }

    pub fn step(&mut self, duration: Duration) -> StepResult {
        if self.remaining < duration {
            self.remaining = Duration::ZERO;
            return StepResult::TimeElapsed;
        }
        self.remaining -= duration;
        self.remaining += self.tc.increment;
        StepResult::Ok
    }

    pub fn to_usi_string(&self, c: Color) -> String {
        let c = match c {
            Color::Sente => 'b',
            Color::Gote => 'w',
        };
        format!(
            "{c}time {} {c}inc {}",
            self.remaining.as_millis(),
            self.tc.increment.as_millis(),
        )
    }
}

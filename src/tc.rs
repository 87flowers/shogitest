use crate::shogi::Color;
use regex::{Match, Regex};
use std::{fmt, time::Duration};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum StepResult {
    Ok,
    TimeElapsed,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum TimeControl {
    #[default]
    None,
    Nodes(u64),
    MoveTime(Duration),
    Byoyomi {
        base: Duration,
        byoyomi: Duration,
    },
    Fischer {
        base: Duration,
        increment: Duration,
    },
}

impl TimeControl {
    pub fn parse(s: &str) -> Option<TimeControl> {
        None.or_else(|| Self::try_parse_fischer(s))
            .or_else(|| Self::try_parse_byoyomi(s))
            .or_else(|| Self::try_parse_movetime(s))
            .or_else(|| Self::try_parse_nodes(s))
    }

    fn try_parse_fischer(s: &str) -> Option<TimeControl> {
        let re = Regex::new(
            r"^(?:(?<min>[0-9.]+)[:分m])?(?:(?<sec>[0-9.]+)[秒s]?)?(?:\+(?<incr>[0-9.]+)[秒s]?)?$",
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

        Some(TimeControl::Fischer {
            base: Duration::from_millis(base_ms),
            increment: Duration::from_millis(incr_ms),
        })
    }

    fn try_parse_byoyomi(s: &str) -> Option<TimeControl> {
        let re = Regex::new(
            r"^(?:(?<min>[0-9.]+)[:分m])?(?:(?<sec>[0-9.]+)[秒s]?)?[,、;](?<byoyomi>[0-9.]+)(?:[秒s](未満)?)?$",
        )
        .unwrap();

        let captures = re.captures(s)?;
        let min = captures.name("min");
        let sec = captures.name("sec");
        let byoyomi = captures.name("byoyomi");

        let to_float = |x: Option<Match>| x.map_or("0", |m| m.as_str()).parse::<f64>();
        let min = to_float(min).ok()?;
        let sec = to_float(sec).ok()?;
        let byoyomi = to_float(byoyomi).ok()?;

        let base = min * 60.0 + sec;

        let base_ms = (base * 1000.0) as u64;
        let byoyomi_ms = (byoyomi * 1000.0) as u64;

        Some(TimeControl::Byoyomi {
            base: Duration::from_millis(base_ms),
            byoyomi: Duration::from_millis(byoyomi_ms),
        })
    }

    fn try_parse_movetime(s: &str) -> Option<TimeControl> {
        let re = Regex::new(r"^([0-9.]+)秒未満|movetime=([0-9.]+)[s秒]?$").unwrap();

        let captures = re.captures(s)?;
        let (_, [movetime]) = captures.extract();

        let movetime = movetime.parse::<f64>().ok()?;

        let movetime_ms = (movetime * 1000.0) as u64;

        Some(TimeControl::MoveTime(Duration::from_millis(movetime_ms)))
    }

    fn try_parse_nodes(s: &str) -> Option<TimeControl> {
        let re = Regex::new(r"^N=([0-9]+)$").unwrap();

        let captures = re.captures(s)?;
        let (_, [nodes]) = captures.extract();

        let nodes = nodes.parse::<u64>().ok()?;

        Some(TimeControl::Nodes(nodes))
    }
}

impl fmt::Display for TimeControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeControl::None => write!(f, "infinite")?,
            TimeControl::Nodes(nodes) => write!(f, "N={nodes}")?,
            TimeControl::MoveTime(duration) => write!(f, "movetime={}s", duration.as_secs_f64())?,
            TimeControl::Byoyomi { base, byoyomi } => {
                let seconds = base.as_secs_f64();

                let minutes = (seconds / 60.0).floor() as i64;
                let seconds = seconds - minutes as f64 * 60.0;

                if minutes > 0 {
                    write!(f, "{minutes}m")?
                }
                if seconds > 0.0 {
                    write!(f, "{seconds}s")?
                }
                write!(f, ",{}s", byoyomi.as_secs_f64())?
            }
            TimeControl::Fischer { base, increment } => {
                if !base.is_zero() || increment.is_zero() {
                    let seconds = base.as_secs_f64();

                    let minutes = (seconds / 60.0).floor() as i64;
                    let seconds = seconds - minutes as f64 * 60.0;

                    if minutes > 0 {
                        write!(f, "{minutes}m")?
                    }
                    if seconds > 0.0 {
                        write!(f, "{seconds}s")?
                    }
                }
                if !increment.is_zero() {
                    write!(f, "+{}s", increment.as_secs_f64())?
                }
            }
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
            remaining: match tc {
                TimeControl::None | TimeControl::MoveTime(_) | TimeControl::Nodes(_) => {
                    Duration::ZERO
                }
                TimeControl::Byoyomi { base, byoyomi: _ } => base,
                TimeControl::Fischer { base, increment } => base + increment,
            },
        }
    }

    pub fn step(&mut self, duration: Duration) -> StepResult {
        match self.tc {
            TimeControl::None | TimeControl::Nodes(_) => StepResult::Ok,
            TimeControl::MoveTime(max_duration) => {
                if duration > max_duration {
                    StepResult::TimeElapsed
                } else {
                    StepResult::Ok
                }
            }
            TimeControl::Byoyomi { base: _, byoyomi } => {
                let duratioon = if self.remaining < duration {
                    let rem = self.remaining;
                    self.remaining = Duration::ZERO;
                    duration - rem
                } else {
                    self.remaining -= duration;
                    Duration::ZERO
                };
                if duration > byoyomi {
                    StepResult::TimeElapsed
                } else {
                    StepResult::Ok
                }
            }
            TimeControl::Fischer { base: _, increment } => {
                if self.remaining < duration {
                    self.remaining = Duration::ZERO;
                    return StepResult::TimeElapsed;
                }
                self.remaining -= duration;
                self.remaining += increment;
                StepResult::Ok
            }
        }
    }
}

pub fn to_usi_string(color: Color, sente_time: &EngineTime, gote_time: &EngineTime) -> String {
    let (stm, nstm) = match color {
        Color::Sente => ('b', 'w'),
        Color::Gote => ('w', 'b'),
    };
    let (stm_time, nstm_time) = match color {
        Color::Sente => (sente_time, gote_time),
        Color::Gote => (gote_time, sente_time),
    };

    let stm_part = match stm_time.tc {
        TimeControl::None => String::new(),
        TimeControl::MoveTime(duration) => format!("{stm}time 0 byoyomi {}", duration.as_millis()),
        TimeControl::Nodes(nodes) => format!("nodes {nodes}"),
        TimeControl::Byoyomi { base: _, byoyomi } => format!(
            "{stm}time {} byoyomi {}",
            stm_time.remaining.as_millis(),
            byoyomi.as_millis()
        ),
        TimeControl::Fischer { base: _, increment } => format!(
            "{stm}time {} {stm}inc {}",
            stm_time.remaining.as_millis(),
            increment.as_millis()
        ),
    };

    let nstm_part = match nstm_time.tc {
        TimeControl::None | TimeControl::MoveTime(_) | TimeControl::Nodes(_) => String::new(),
        TimeControl::Byoyomi {
            base: _,
            byoyomi: _,
        } => {
            format!(" {nstm}time {}", nstm_time.remaining.as_millis())
        }
        TimeControl::Fischer { base: _, increment } => format!(
            " {nstm}time {} {nstm}inc {}",
            nstm_time.remaining.as_millis(),
            increment.as_millis()
        ),
    };

    stm_part + &nstm_part
}

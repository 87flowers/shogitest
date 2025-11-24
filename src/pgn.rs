use crate::{cli, engine::Score, shogi, tournament};
use std::fs::File;
use std::io::{Error, Write};

#[derive(Debug)]
pub struct PgnWriter {
    file: File,
    engine_names: Vec<String>,
    options: cli::PgnOutOptions,
    meta: cli::MetaDataOptions,
}

impl PgnWriter {
    pub fn new(
        options: &cli::PgnOutOptions,
        meta: &cli::MetaDataOptions,
        engine_names: Vec<String>,
    ) -> Result<PgnWriter, Error> {
        Ok(PgnWriter {
            file: File::create_new(&options.file)?,
            engine_names,
            options: options.clone(),
            meta: meta.clone(),
        })
    }

    fn write_header(file: &mut File, key: &str, value: &str) -> Result<(), Error> {
        writeln!(file, "[{} {:?}]", key, value)?;
        Ok(())
    }

    pub fn write(&mut self, match_result: &tournament::MatchResult) -> Result<(), Error> {
        let f = &mut self.file;
        let ticket = &match_result.ticket;
        let date_str = match_result.game_start.format("%Y-%m-%d").to_string();
        let result_str = match match_result.outcome.winner() {
            Some(shogi::Color::Sente) => "1-0",
            Some(shogi::Color::Gote) => "0-1",
            None if match_result.outcome.is_draw() => "1/2-1/2",
            None => "undetermined"
        };

        Self::write_header(f, "Event", &self.meta.event_name)?;
        Self::write_header(f, "Event", &self.meta.site_name)?;
        Self::write_header(f, "Date", &date_str)?;
        Self::write_header(f, "Round", &ticket.id.to_string())?;
        Self::write_header(f, "Black", &self.engine_names[ticket.engines[0]])?;
        Self::write_header(f, "Sente", &self.engine_names[ticket.engines[0]])?;
        Self::write_header(f, "White", &self.engine_names[ticket.engines[1]])?;
        Self::write_header(f, "Gote", &self.engine_names[ticket.engines[1]])?;
        Self::write_header(f, "Result", result_str)?;

        // TODO: If book pos != Startpos emit SetUp and FEN

        Self::write_header(f, "PlyCount", &match_result.moves.len().to_string())?;
        Self::write_header(f, "Termination", match_result.outcome.to_string())?;

        // TODO : "GameDuration" "GameStartTime" "GameEndTime" "PlyCount" "Termination" "TimeControl" "WhiteTimeControl" "BlackTimeControl"

        writeln!(f)?;

        for m in &match_result.moves {
            let mstr = if m.mstr.is_empty() {
                "output-was-empty"
            } else {
                &m.mstr
            };
            let score_str = match m.score {
                Score::None => String::from("none"),
                Score::Cp(cp) => format!("{:+.2}", cp as f64 / 100.0),
                Score::Mate(x) => {
                    format!("{}M{}", if x > 0 { "+" } else { "-" }, x.abs())
                }
            };
            let mut comment = format!("{score_str} {}", m.depth);
            if self.options.track_seldepth {
                comment = format!("{comment}/{}", m.seldepth);
            }
            if self.options.track_nodes {
                comment = format!("{comment} n={}", m.nodes);
            }
            if self.options.track_nps {
                comment = format!("{comment} nps={}", m.nps);
            }
            if self.options.track_hashfull {
                comment = format!("{comment} hashfull={}", m.hashfull);
            }
            if self.options.track_timeleft && let Some(time_left) = m.time_left {
                comment = format!("{comment} timeleft={}s", time_left.as_secs_f64());
            }
            if self.options.track_latency {
                let latency = m.measured_time.as_secs_f64() - m.engine_time as f64 / 1000.0;
                comment = format!("{comment} latency={latency}s");
            }
            comment = format!("{comment} t={}s", m.measured_time.as_secs_f64());
            writeln!(f, "{mstr} {{{comment}}}",)?;
        }

        writeln!(f, "{result_str}")?;
        writeln!(f)?;

        Ok(())
    }
}

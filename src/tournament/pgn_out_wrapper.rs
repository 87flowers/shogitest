use crate::{
    cli, pgn,
    tournament::{MatchResult, MatchTicket, Tournament, TournamentState},
};

pub struct PgnOutWrapper {
    inner: Box<dyn Tournament>,
    pgn: pgn::PgnWriter,
}

impl PgnOutWrapper {
    pub fn new(
        inner: Box<dyn Tournament>,
        options: &cli::PgnOutOptions,
        meta: &cli::MetaDataOptions,
        engine_options: Vec<cli::EngineOptions>,
        engine_names: Vec<String>,
    ) -> Result<PgnOutWrapper, std::io::Error> {
        Ok(PgnOutWrapper {
            inner,
            pgn: pgn::PgnWriter::new(options, meta, engine_options, engine_names)?,
        })
    }
}

impl Tournament for PgnOutWrapper {
    fn next(&mut self) -> Option<MatchTicket> {
        self.inner.as_mut().next()
    }
    fn match_started(&mut self, ticket: MatchTicket) {
        self.inner.as_mut().match_started(ticket);
    }
    fn match_complete(&mut self, result: MatchResult) -> TournamentState {
        self.pgn.write(&result).unwrap();
        self.inner.as_mut().match_complete(result)
    }
    fn print_interval_report(&self) {
        self.inner.print_interval_report()
    }
    fn tournament_complete(&self) {
        self.inner.tournament_complete()
    }
    fn expected_maximum_match_count(&self) -> Option<u64> {
        self.inner.as_ref().expected_maximum_match_count()
    }
}

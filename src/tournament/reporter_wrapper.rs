use crate::{
    shogi,
    tournament::{MatchResult, MatchTicket, Tournament, TournamentState},
};

pub struct ReporterWrapper {
    inner: Box<dyn Tournament>,
    engine_names: Vec<String>,
}

impl ReporterWrapper {
    pub fn new(inner: Box<dyn Tournament>, engine_names: Vec<String>) -> ReporterWrapper {
        ReporterWrapper {
            inner,
            engine_names,
        }
    }
}

impl ReporterWrapper {
    fn format_of_max_string(&self) -> String {
        match self.expected_maximum_match_count() {
            Some(count) => format!(" of {count}"),
            None => String::from(" of infinite"),
        }
    }
}

impl Tournament for ReporterWrapper {
    fn next(&mut self) -> Option<MatchTicket> {
        self.inner.as_mut().next()
    }
    fn match_started(&mut self, ticket: MatchTicket) {
        println!(
            "Started game {}{} ({} vs {})",
            ticket.id + 1,
            self.format_of_max_string(),
            &self.engine_names[ticket.engines[0]],
            &self.engine_names[ticket.engines[1]]
        );
        self.inner.as_mut().match_started(ticket)
    }
    fn match_complete(&mut self, result: MatchResult) -> TournamentState {
        let ticket = &result.ticket;
        println!(
            "Finished game {} ({} vs {}): {} {{{}}}",
            ticket.id + 1,
            &self.engine_names[ticket.engines[0]],
            &self.engine_names[ticket.engines[1]],
            match result.outcome.winner() {
                Some(shogi::Color::Sente) => "1-0",
                Some(shogi::Color::Gote) => "0-1",
                None if result.outcome.is_draw() => "1/2-1/2",
                None => "undetermined",
            },
            result.outcome.to_string(),
        );
        self.inner.as_mut().match_complete(result)
    }
    fn print_interval_report(&self) {
        self.inner.print_interval_report()
    }
    fn tournament_complete(&self) {
        println!("Tournament finished");
        self.inner.tournament_complete()
    }
    fn expected_maximum_match_count(&self) -> Option<u64> {
        self.inner.as_ref().expected_maximum_match_count()
    }
}

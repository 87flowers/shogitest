use crate::shogi;

#[derive(Debug, Clone)]
pub struct MatchTicket {
    pub id: usize,
    pub engines: [usize; 2],
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub ticket: MatchTicket,
    pub outcome: shogi::GameOutcome,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TournamentState {
    Continue,
    Stop,
}

pub trait Tournament {
    fn next(&mut self) -> Option<MatchTicket>;
    fn match_complete(&mut self, result: MatchResult) -> TournamentState;
}

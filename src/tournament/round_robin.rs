use crate::{book, cli, tournament};

fn pairings_count(players: usize) -> u64 {
    (players * (players - 1) / 2) as u64
}

#[derive(Debug)]
pub struct RoundRobin {
    match_index: u64,
    completed_matches: u64,
    next_players: [usize; 2],
    total_matches: Option<u64>,
    players: usize,
    options: cli::CliOptions,
    openings: book::OpeningBook,
}

impl RoundRobin {
    pub fn new(options: &cli::CliOptions, openings: book::OpeningBook) -> RoundRobin {
        let players = options.engines.len();
        RoundRobin {
            match_index: 0,
            completed_matches: 0,
            next_players: [0, 1],
            players: players,
            total_matches: options
                .games
                .map(|g| pairings_count(players) * options.rounds * g),
            options: options.clone(),
            openings: openings,
        }
    }
}

impl tournament::Tournament for RoundRobin {
    fn next(&mut self) -> Option<tournament::MatchTicket> {
        let id = self.match_index;
        let opening = self.openings.current();

        let mut players = self.next_players.clone();
        if id % self.options.rounds % 2 == 1 {
            players.reverse();
        }

        self.match_index += 1;

        if self.match_index % self.options.rounds == 0 {
            self.openings.advance();
            self.next_players[1] += 1;
            if self.next_players[1] >= self.players {
                self.next_players[0] += 1;
                self.next_players[1] = self.next_players[0] + 1;
                if self.next_players[1] >= self.players {
                    self.next_players = [0, 1];
                }
            }
        }

        if let Some(total_matches) = self.total_matches
            && id >= total_matches
        {
            None
        } else {
            Some(tournament::MatchTicket {
                id: id,
                opening: opening,
                engines: players,
            })
        }
    }
    fn match_complete(&mut self, result: tournament::MatchResult) -> tournament::TournamentState {
        self.completed_matches += 1;

        if let Some(total_matches) = self.total_matches
            && self.completed_matches >= total_matches
        {
            tournament::TournamentState::Stop
        } else {
            tournament::TournamentState::Continue
        }
    }
    fn expected_maximum_match_count(&self) -> Option<u64> {
        self.total_matches
    }
}

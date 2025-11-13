#![feature(str_split_whitespace_remainder)]

use flexi_logger;
use log::info;

mod cli;
mod engine;
mod runner;
mod shogi;
mod tournament;

struct TestTournament {
    match_index: usize,
    completed_matches: usize,
}

impl tournament::Tournament for TestTournament {
    fn next(&mut self) -> Option<tournament::MatchTicket> {
        let id = self.match_index;
        let i = self.match_index % 2;
        let j = 1 - i;

        self.match_index += 1;

        if id < 10 {
            dbg!(&id);
            Some(tournament::MatchTicket {
                id,
                engines: [i, j],
            })
        } else {
            None
        }
    }
    fn match_complete(&mut self, result: tournament::MatchResult) -> tournament::TournamentState {
        dbg!(result);
        self.completed_matches += 1;
        dbg!(&self.completed_matches);
        if self.completed_matches < 10 {
            tournament::TournamentState::Continue
        } else {
            tournament::TournamentState::Stop
        }
    }
}

fn main() -> std::io::Result<()> {
    flexi_logger::init();

    let cli_options = cli::parse();
    info!("{:#?}", &cli_options);

    if cli_options.engines.len() != 2 {
        eprintln!("Only exactly two engines supported currently");
        return Ok(());
    }

    let mut tournament = TestTournament {
        match_index: 0,
        completed_matches: 0,
    };
    let r = runner::Runner::new(cli_options.engines, cli_options.concurrency);
    r.run(&mut tournament);

    Ok(())
}

use std::{cmp::Ordering, collections::HashMap, path::Path};

use crate::{
    cli,
    shogi::{Color, GameOutcome},
    stats::{Penta, Wdl},
    tournament::{MatchResult, MatchTicket, Tournament, TournamentState},
};

pub struct StatsWrapper {
    inner: Box<dyn Tournament>,
    engine_names: Vec<String>,
    engine_options: Vec<cli::EngineOptions>,
    book_name: Option<String>,
    wdl_board: HashMap<(usize, usize), Wdl>,
    penta_board: HashMap<(usize, usize), Penta>,
    pending_pairing: HashMap<u64, ((usize, usize), GameOutcome)>,
}

impl StatsWrapper {
    pub fn new(
        inner: Box<dyn Tournament>,
        engine_names: Vec<String>,
        engine_options: Vec<cli::EngineOptions>,
        book_name: Option<String>,
    ) -> StatsWrapper {
        StatsWrapper {
            inner,
            engine_names,
            engine_options,
            book_name,
            wdl_board: HashMap::new(),
            penta_board: HashMap::new(),
            pending_pairing: HashMap::new(),
        }
    }
    fn add_result(&mut self, match_id: u64, (a, b): (usize, usize), result: Option<Color>) {
        self.add_wdl((a, b), result);
        self.add_wdl((b, a), result.map(|c| !c));
        self.add_penta_half(match_id, (a, b), result);
    }
    fn add_wdl(&mut self, key: (usize, usize), result: Option<Color>) {
        let wdl = match result {
            Some(Color::Sente) => Wdl::ONE_WIN,
            None => Wdl::ONE_DRAW,
            Some(Color::Gote) => Wdl::ONE_LOSS,
        };

        let old_value = self.wdl_board.get(&key).cloned().unwrap_or_default();
        self.wdl_board.insert(key, old_value + wdl);
    }
    pub fn all_stats_for(&self, engine_id: usize) -> Wdl {
        (0..self.engine_names.len())
            .map(|i| (engine_id, i))
            .map(|k| self.wdl_board.get(&k).cloned().unwrap_or_default())
            .sum()
    }
    pub fn print_stats(&self) {
        if self.engine_names.len() == 2 {
            self.print_head_to_head()
        } else {
            self.print_table()
        }
    }
    pub fn print_head_to_head(&self) {
        let wdl = self.all_stats_for(1);
        let (lelo, lelo_diff) = wdl.logistic_elo();

        let tc = compare(|i| self.engine_options[i].time_control.to_string());
        let threads = compare(|i| {
            self.engine_options[i]
                .builder
                .get_usi_option_value("Threads")
                .unwrap_or("null")
                .to_string()
        });
        let hash = compare(|i| {
            self.engine_options[i]
                .builder
                .get_usi_option_value("Hash")
                .unwrap_or("null")
                .to_string()
        });
        let book = self
            .book_name
            .as_ref()
            .and_then(|p| Path::new(p).file_name())
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or("null".to_string());

        println!(
            "Results of {} vs {} ({tc}, {threads}, {hash}, {book}):",
            self.engine_names[0], self.engine_names[1]
        );
        println!("Elo: {lelo:.2} +/- {lelo_diff:.2}");
        println!(
            "Games: {}, Wins: {}, Draws: {}, Losses: {}",
            wdl.game_count(),
            wdl.w,
            wdl.d,
            wdl.l
        );
    }
    pub fn print_table(&self) {
        let mut table = Vec::<(&str, f64, f64, u64, f64)>::new();
        let mut max_name_len = 25;

        for (i, name) in self.engine_names.iter().enumerate() {
            max_name_len = max_name_len.max(name.len());

            let wdl = self.all_stats_for(i);
            let (elo, elo_diff) = wdl.logistic_elo();
            let game_count = wdl.game_count();
            let points = wdl.points();

            table.push((name, elo, elo_diff, game_count, points));
        }

        table.sort_by(|x, y| {
            if x.1 == y.1 {
                Ordering::Equal
            } else if x.1 < y.1 || x.1.is_nan() {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        });

        println!(
            "{:>4} {:<max_name_len$} {:>10} {:>10} {:>10} {:>10}",
            "Rank", "Name", "Elo", "+/-", "Games", "Score"
        );
        for (i, (name, elo, elo_diff, game_count, points)) in table.iter().enumerate() {
            let rank = i + 1;
            println!(
                "{rank:>4} {name:<max_name_len$} {elo:>10.2} {elo_diff:>10.2} {game_count:>10} {points:>10.1}"
            );
        }
    }
}

impl Tournament for StatsWrapper {
    fn next(&mut self) -> Option<MatchTicket> {
        self.inner.as_mut().next()
    }
    fn match_started(&mut self, ticket: MatchTicket) {
        self.inner.as_mut().match_started(ticket)
    }
    fn match_complete(&mut self, result: MatchResult) -> TournamentState {
        let e = &result.ticket.engines;
        self.add_result(result.ticket.id, (e[0], e[1]), result.outcome.winner());
        self.inner.as_mut().match_complete(result)
    }
    fn print_interval_report(&self) {
        self.print_stats();
        self.inner.print_interval_report()
    }
    fn tournament_complete(&self) {
        self.print_stats();
        self.inner.tournament_complete()
    }
    fn expected_maximum_match_count(&self) -> Option<u64> {
        self.inner.as_ref().expected_maximum_match_count()
    }
}

fn compare<F>(f: F) -> String
where
    F: Fn(usize) -> String,
{
    let first = f(0);
    let second = f(1);
    if first == second {
        first
    } else {
        format!("{first} - {second}")
    }
}

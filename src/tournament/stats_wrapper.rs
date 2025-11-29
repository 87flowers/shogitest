use std::{cmp::Ordering, collections::HashMap, path::Path};

use crate::{
    cli,
    shogi::Color,
    sprt::SprtParameters,
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
    pending_pairing: HashMap<u64, ((usize, usize), Option<Color>)>,
    sprt: Option<SprtParameters>,
    match_ticket_count: u64,
    match_complete_count: u64,
    should_terminate: bool,
}

impl StatsWrapper {
    pub fn new(
        inner: Box<dyn Tournament>,
        engine_names: Vec<String>,
        engine_options: Vec<cli::EngineOptions>,
        book_name: Option<String>,
        sprt: Option<SprtParameters>,
    ) -> StatsWrapper {
        assert!(engine_names.len() == engine_options.len());
        if sprt.is_some() {
            assert!(engine_names.len() == 2);
        }
        StatsWrapper {
            inner,
            engine_names,
            engine_options,
            book_name,
            wdl_board: HashMap::new(),
            penta_board: HashMap::new(),
            pending_pairing: HashMap::new(),
            sprt,
            match_ticket_count: 0,
            match_complete_count: 0,
            should_terminate: false,
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
    fn add_penta_half(&mut self, match_id: u64, (a, b): (usize, usize), result1: Option<Color>) {
        let sibling = match_id ^ 1;
        if let Some(((b2, a2), result2)) = self.pending_pairing.remove(&sibling) {
            assert!(a == a2 && b == b2);

            let penta = match (result1, result2.map(|c| !c)) {
                (Some(Color::Sente), Some(Color::Sente)) => Penta::ONE_WW,
                (Some(Color::Sente), None) => Penta::ONE_WD,
                (None, Some(Color::Sente)) => Penta::ONE_WD,
                (None, None) => Penta::ONE_DD,
                (Some(Color::Gote), Some(Color::Sente)) => Penta::ONE_WL,
                (Some(Color::Sente), Some(Color::Gote)) => Penta::ONE_WL,
                (Some(Color::Gote), None) => Penta::ONE_DL,
                (None, Some(Color::Gote)) => Penta::ONE_DL,
                (Some(Color::Gote), Some(Color::Gote)) => Penta::ONE_LL,
            };

            let mut insert = |key: (usize, usize), penta: Penta| {
                let old_value = self.penta_board.get(&key).cloned().unwrap_or_default();
                self.penta_board.insert(key, old_value + penta);
            };

            insert((a, b), penta);
            insert((b, a), penta.flip());
        } else {
            self.pending_pairing.insert(match_id, ((a, b), result1));
        }
    }
    pub fn all_wdl_for(&self, engine_id: usize) -> Wdl {
        (0..self.engine_names.len())
            .map(|i| (engine_id, i))
            .map(|k| self.wdl_board.get(&k).cloned().unwrap_or_default())
            .sum()
    }
    pub fn all_penta_for(&self, engine_id: usize) -> Penta {
        (0..self.engine_names.len())
            .map(|i| (engine_id, i))
            .map(|k| self.penta_board.get(&k).cloned().unwrap_or_default())
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
        let wdl = self.all_wdl_for(1);
        let penta = self.all_penta_for(1);
        let (lelo, lelo_diff) = penta.logistic_elo();
        let (nelo, nelo_diff) = penta.normalized_elo();

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
        println!("Elo: {lelo:.2} +/- {lelo_diff:.2}, nElo: {nelo:.2} +/- {nelo_diff:.2}");
        println!(
            "Games: {}, Wins: {}, Draws: {}, Losses: {} (Score: {:.2}%)",
            wdl.game_count(),
            wdl.w,
            wdl.d,
            wdl.l,
            wdl.score() * 100.0
        );
        println!(
            "Pntml(0-2): {penta}, DD/WL Ratio: {:.2}",
            penta.dd_wl_ratio()
        );
        if let Some(sprt) = self.sprt
            && penta.pair_count() > 0
        {
            let llr = sprt.llr(penta);
            let (llr_lower_bound, llr_upper_bound) = sprt.llr_bounds();
            let (nelo_lower_bound, nelo_upper_bound) = sprt.nelo_bounds();
            println!(
                "LLR: {llr:.2} ({llr_lower_bound:.2}, {llr_upper_bound:.2}) [{nelo_lower_bound:.2}, {nelo_upper_bound:.2}]"
            );
        }
    }
    pub fn print_table(&self) {
        let mut table = Vec::<(&str, f64, Wdl, Penta)>::new();
        let mut max_name_len = 20;
        let mut max_penta_len = 2;

        for (i, name) in self.engine_names.iter().enumerate() {
            let wdl = self.all_wdl_for(i);
            let penta = self.all_penta_for(i);
            let (lelo, _) = penta.logistic_elo();

            table.push((name, lelo, wdl, penta));

            max_name_len = max_name_len.max(name.len());
            max_penta_len = max_penta_len.max(format!("{penta}").len());
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
            "{:>4} {:<max_name_len$} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}  {:>max_penta_len$}",
            "Rank", "Name", "Elo", "+/-", "nElo", "+/-", "Games", "Score", "Penta"
        );
        for (i, (name, lelo, wdl, penta)) in table.iter().enumerate() {
            let rank = i + 1;
            let (_, lelo_diff) = penta.logistic_elo();
            let (nelo, nelo_diff) = penta.normalized_elo();
            let game_count = wdl.game_count();
            let score = wdl.score() * 100.0;
            let penta = format!("{penta}");
            println!(
                "{rank:>4} {name:<max_name_len$} {lelo:>8.2} {lelo_diff:>8.2} {nelo:>8.2} {nelo_diff:>8.2} {game_count:>8} {score:>7.2}%  {penta:>max_penta_len$}"
            );
        }
    }
    fn next(&mut self) {
        self.match_ticket_count += 1;
    }
    fn next_should_terminate(&self) -> bool {
        self.should_terminate
    }
    fn match_complete(&mut self) {
        self.match_complete_count += 1;
        if let Some(sprt) = self.sprt
            && !self.should_terminate
        {
            let penta = self.all_penta_for(1);
            self.should_terminate = sprt.should_terminate(penta);
        }
    }
    fn match_completete_should_terminate(&self) -> bool {
        self.should_terminate && self.match_ticket_count == self.match_complete_count
    }
}

impl Tournament for StatsWrapper {
    fn next(&mut self) -> Option<MatchTicket> {
        if self.next_should_terminate() {
            None
        } else {
            self.next();
            self.inner.as_mut().next()
        }
    }
    fn match_started(&mut self, ticket: MatchTicket) {
        self.inner.as_mut().match_started(ticket)
    }
    fn match_complete(&mut self, result: MatchResult) -> TournamentState {
        let e = &result.ticket.engines;
        self.add_result(result.ticket.id, (e[0], e[1]), result.outcome.winner());
        self.match_complete();
        let state = self.inner.as_mut().match_complete(result);
        if self.match_completete_should_terminate() {
            TournamentState::Stop
        } else {
            state
        }
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

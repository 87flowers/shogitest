#![feature(duration_constants)]
#![feature(if_let_guard)]
#![feature(str_split_whitespace_remainder)]

use log::info;
use rand::SeedableRng;

mod book;
mod cli;
mod engine;
mod pgn;
mod runner;
mod shogi;
mod sprt;
mod stats;
mod tc;
mod tournament;
mod util;

fn main() -> std::io::Result<()> {
    flexi_logger::Logger::try_with_env().unwrap().start().ok();

    let Some(cli_options) = cli::parse() else {
        return Ok(());
    };
    info!("{:#?}", &cli_options);

    if cli_options.engines.len() < 2 {
        eprintln!("We require at least two engines to be supplied.");
        return Ok(());
    }

    if cli_options.book.is_none() {
        eprintln!("Openings file required.");
        return Ok(());
    }

    let engine_names = cli_options.engine_names();

    let opening_book = {
        let mut rng = match cli_options.rand_seed {
            Some(seed) => rand_chacha::ChaCha8Rng::seed_from_u64(seed),
            None => rand_chacha::ChaCha8Rng::from_os_rng(),
        };
        book::OpeningBook::new(cli_options.book.as_ref().unwrap(), &mut rng).unwrap()
    };

    let mut tournament: Box<dyn tournament::Tournament> =
        Box::new(tournament::RoundRobin::new(&cli_options, opening_book));

    if let Some(pgn) = cli_options.pgn {
        tournament = Box::new(tournament::PgnOutWrapper::new(
            tournament,
            &pgn,
            &cli_options.meta,
            cli_options.engines.clone(),
            engine_names.clone(),
        )?);
    }

    let sprt_parameters = cli_options
        .sprt
        .map(|sprt| sprt::SprtParameters::new(sprt.nelo0, sprt.nelo1, sprt.alpha, sprt.beta));

    tournament = Box::new(tournament::StatsWrapper::new(
        tournament,
        engine_names.clone(),
        cli_options.engines.clone(),
        cli_options.book.map(|b| b.file.clone()),
        sprt_parameters,
    ));

    tournament = Box::new(tournament::ReporterWrapper::new(
        tournament,
        engine_names.clone(),
    ));

    let r = runner::Runner::new(
        cli_options.engines,
        cli_options.concurrency,
        cli_options.adjudication,
        cli_options.report_interval,
    );
    r.run(tournament);

    Ok(())
}

#![feature(str_split_whitespace_remainder)]

use flexi_logger;
use log::info;

mod cli;
mod engine;
mod pgn;
mod runner;
mod shogi;
mod tc;
mod tournament;

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

    let engine_names = cli_options.engine_names();

    let mut tournament: Box<dyn tournament::Tournament> =
        Box::new(tournament::RoundRobin::new(&cli_options));

    if let Some(pgn) = cli_options.pgn {
        tournament = Box::new(tournament::PgnOutWrapper::new(
            tournament,
            &pgn,
            &cli_options.meta,
            engine_names.clone(),
        )?);
    }

    tournament = Box::new(tournament::ReporterWrapper::new(
        tournament,
        engine_names.clone(),
    ));

    let r = runner::Runner::new(cli_options.engines, cli_options.concurrency);
    r.run(tournament);

    Ok(())
}

#![feature(str_split_whitespace_remainder)]

use flexi_logger;
use log::info;

mod cli;
mod engine;
mod runner;
mod shogi;

fn main() -> std::io::Result<()> {
    flexi_logger::init();

    let cli_options = cli::parse();
    info!("{:#?}", &cli_options);

    if cli_options.engines.len() != 2 {
        eprintln!("Only exactly two engines supported currently");
        return Ok(());
    }

    let r = runner::Runner::new(cli_options.engines, cli_options.concurrency);

    let mut match_index = 0;
    let mut completed_matches = 0;

    r.run(
        || {
            let id = match_index;
            let i = match_index % 2;
            let j = 1 - i;

            match_index += 1;

            if id < 10 {
                dbg!(&id);
                Some(runner::MatchTicket {
                    id,
                    engines: [i, j],
                })
            } else {
                None
            }
        },
        |res| {
            dbg!(res);
            completed_matches += 1;
            dbg!(&completed_matches);
            completed_matches < 10
        },
    );

    Ok(())
}

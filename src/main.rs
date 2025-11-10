#![feature(str_split_whitespace_remainder)]

use crate::shogi::GameOutcome;
use flexi_logger;
use log::info;

mod cli;
mod engine;
mod shogi;

fn main() -> std::io::Result<()> {
    flexi_logger::init();

    let cli_options = cli::parse();
    info!("{:#?}", &cli_options);

    if cli_options.engines.len() != 2 {
        eprintln!("Only exactly two engines supported currently");
        return Ok(());
    }

    let mut engine0 = cli_options.engines[0].builder.init()?;
    let mut engine1 = cli_options.engines[1].builder.init()?;

    engine0.isready()?;
    engine1.isready()?;

    engine0.usinewgame()?;
    engine1.usinewgame()?;

    let mut game = shogi::Game::new(shogi::Position::default());
    loop {
        let stm = game.stm();

        let current_engine = match stm {
            shogi::Color::Sente => &mut engine0,
            shogi::Color::Gote => &mut engine1,
        };

        current_engine.position(&game)?;

        current_engine.write_line("go movetime 100")?;
        current_engine.flush()?;

        let m = current_engine.wait_for_bestmove()?;

        let outcome = match m {
            None => GameOutcome::LossByIllegal(stm),
            Some(m) => game.do_move(m),
        };

        if outcome.is_determined() {
            dbg!(&outcome);
            break;
        }
    }

    Ok(())
}

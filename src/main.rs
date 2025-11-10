#![feature(str_split_whitespace_remainder)]

use crate::shogi::GameOutcome;

mod cli;
mod engine;
mod shogi;

fn main() -> std::io::Result<()> {
    let cli_options = cli::parse();
    dbg!(&cli_options);

    if cli_options.engines.len() != 2 {
        eprintln!("Only exactly two engines supported currently");
        return Ok(());
    }

    let mut engine0 = cli_options.engines[0].builder.init()?;
    let mut engine1 = cli_options.engines[1].builder.init()?;

    dbg!(&engine0);
    dbg!(&engine1);

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

        current_engine.write_line("go movetime 1000")?;
        current_engine.flush()?;

        let m = current_engine.wait_for_bestmove()?;
        dbg!(&m);

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

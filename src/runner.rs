use crate::{cli, engine, shogi};
use crossbeam_channel;
use std::thread;

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

#[derive(Debug)]
pub struct Runner {
    engines: Vec<cli::EngineOptions>,
    concurrency: u64,
}

impl Runner {
    pub fn new(engines: Vec<cli::EngineOptions>, concurrency: u64) -> Runner {
        Runner {
            engines,
            concurrency,
        }
    }

    pub fn run<Next, Consume>(&self, mut next: Next, mut consume: Consume)
    where
        Next: FnMut() -> Option<MatchTicket>,
        Consume: FnMut(MatchResult) -> bool,
    {
        let (send_ticket, recv_ticket) = crossbeam_channel::bounded(0);
        let (send_result, recv_result) = crossbeam_channel::bounded(0);

        let mut thread_handles = vec![];

        for i in 0..self.concurrency {
            let recv_ticket = recv_ticket.clone();
            let send_result = send_result.clone();
            let engines = self.engines.clone();
            thread_handles.push(thread::spawn(move || {
                runner_thread_main(engines, i, recv_ticket, send_result);
            }));
        }

        let mut ok = true;
        let mut ticket = next();
        while ok {
            if ticket.is_none() {
                ok = consume(recv_result.recv().unwrap());
            } else {
                crossbeam_channel::select! {
                    recv(recv_result) -> result => ok = consume(result.unwrap()),
                    send(send_ticket, ticket) -> result => {
                        assert!(result.is_ok());
                        ticket = next();
                    }
                }
            }
        }

        for i in 0..self.concurrency {
            send_ticket.send(None).unwrap();
        }

        while let Some(h) = thread_handles.pop() {
            h.join().expect("could not join thread");
        }
    }
}

fn runner_thread_main(
    engine_options: Vec<cli::EngineOptions>,
    thread_index: u64,
    recv: crossbeam_channel::Receiver<Option<MatchTicket>>,
    send: crossbeam_channel::Sender<MatchResult>,
) {
    let mut engines: Vec<_> = engine_options
        .iter()
        .map(|o| o.builder.init().unwrap())
        .collect();

    loop {
        match recv.recv().unwrap() {
            None => break,
            Some(ticket) => {
                assert!(ticket.engines[0] != ticket.engines[1]);
                let outcome = run_match(&mut engines, &ticket).unwrap();
                send.send(MatchResult { ticket, outcome }).unwrap();
            }
        }
    }
}

fn run_match(
    engines: &mut Vec<engine::Engine>,
    ticket: &MatchTicket,
) -> Result<shogi::GameOutcome, std::io::Error> {
    for i in 0..2 {
        engines[ticket.engines[i]].isready()?;
        engines[ticket.engines[i]].usinewgame()?;
    }

    let mut game = shogi::Game::new(shogi::Position::default());
    loop {
        let stm = game.stm();

        let current_engine = &mut engines[ticket.engines[stm.to_index()]];

        current_engine.position(&game)?;

        current_engine.write_line("go movetime 100")?;
        current_engine.flush()?;

        let m = current_engine.wait_for_bestmove()?;

        let outcome = match m {
            None => shogi::GameOutcome::LossByIllegal(stm),
            Some(m) => game.do_move(m),
        };

        if outcome.is_determined() {
            return Ok(outcome);
        }
    }
}

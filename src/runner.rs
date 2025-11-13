use crate::tournament::{MatchResult, MatchTicket, Tournament, TournamentState};
use crate::{cli, engine, shogi};
use crossbeam_channel;
use log::info;
use std::thread;

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

    pub fn run<T>(&self, tournament: &mut T)
    where
        T: Tournament,
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

        let mut state = TournamentState::Continue;
        let mut ticket = None;
        while state != TournamentState::Stop {
            if ticket.is_none() {
                ticket = tournament.next();
            }
            if ticket.is_none() {
                crossbeam_channel::select! {
                    recv(recv_result) -> result => state = tournament.match_complete(result.unwrap()),
                }
            } else {
                crossbeam_channel::select! {
                    recv(recv_result) -> result => state = tournament.match_complete(result.unwrap()),
                    send(send_ticket, ticket.clone()) -> result => {
                        assert!(result.is_ok());
                        ticket = None;
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

    while let Some(ticket) = recv.recv().unwrap() {
        assert!(ticket.engines[0] != ticket.engines[1]);
        info!("Thread {thread_index} received ticket: {:?}", &ticket);

        let outcome = run_match(&mut engines, &ticket).unwrap();

        let result = MatchResult { ticket, outcome };
        info!("Thread {thread_index} sending result: {:?}", &result);
        send.send(result).unwrap();
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

        current_engine.write_line("go movetime 10")?;
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

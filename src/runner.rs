use crate::shogi::{Color, GameOutcome};
use crate::tc::StepResult;
use crate::tournament::{MatchResult, MatchTicket, Tournament, TournamentState};
use crate::{cli, engine, shogi, tc};
use chrono::Utc;
use crossbeam_channel;
use log::info;
use std::thread;
use std::time::Instant;

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

    pub fn run(&self, mut tournament: Box<dyn Tournament>) {
        let tournament = tournament.as_mut();

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

        for _ in 0..self.concurrency {
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

        let result = run_match(&engine_options, &mut engines, &ticket).unwrap();

        info!("Thread {thread_index} sending result: {:?}", &result);
        send.send(result).unwrap();
    }
}

fn run_match(
    engine_options: &[cli::EngineOptions],
    engines: &mut [engine::Engine],
    ticket: &MatchTicket,
) -> Result<MatchResult, std::io::Error> {
    let mut match_result = MatchResult {
        ticket: ticket.clone(),
        game_start: Utc::now(),
        outcome: shogi::GameOutcome::Undetermined,
        moves: vec![],
    };

    let mut engine_time = [
        tc::EngineTime::new(engine_options[ticket.engines[0]].time_control),
        tc::EngineTime::new(engine_options[ticket.engines[1]].time_control),
    ];

    for i in 0..2 {
        engines[ticket.engines[i]].isready()?;
        engines[ticket.engines[i]].usinewgame()?;
    }

    let mut game = shogi::Game::new(shogi::Position::default());
    loop {
        let stm = game.stm();

        let current_engine = &mut engines[ticket.engines[stm.to_index()]];

        let now = Instant::now();
        current_engine.position(&game)?;

        current_engine.write_line(&format!(
            "go {}",
            tc::to_usi_string(stm, &engine_time[0], &engine_time[1])
        ))?;
        current_engine.flush()?;

        let move_record = current_engine.wait_for_bestmove()?;

        let duration = Instant::now() - now;
        let time_outcome = engine_time[stm.to_index()].step(duration);

        let m = move_record.m;
        match_result.moves.push(move_record);
        match_result.outcome = game.do_move(m);

        if time_outcome == StepResult::TimeElapsed {
            match_result.outcome = GameOutcome::LossByClock(stm);
        }

        if match_result.outcome.is_determined() {
            return Ok(match_result);
        }
    }
}

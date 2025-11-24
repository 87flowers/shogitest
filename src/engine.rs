use crate::shogi;
use log::{error, info, trace};
use std::{
    env,
    io::{BufRead, BufReader, Result, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::Duration,
};
use wait_timeout::ChildExt;

#[derive(Debug, Clone, Default)]
pub enum Score {
    #[default]
    None,
    Cp(i32),
    Mate(i32),
}

#[derive(Debug, Clone, Default)]
pub struct MoveRecord {
    pub m: shogi::Move,
    pub mstr: String,
    pub score: Score,
    pub depth: u32,
    pub seldepth: u32,
    pub nodes: u64,
    pub nps: u64,
    pub engine_time: u64,
    pub hashfull: u32,
    pub measured_time: Duration,
    pub time_left: Option<Duration>,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct EngineBuilder {
    pub dir: String,
    pub cmd: String,
    pub name: Option<String>,
    pub usi_options: Vec<(String, String)>,
}

impl EngineBuilder {
    pub fn init(&self) -> Result<Engine> {
        let working_directory = env::current_dir()?.join(&self.dir);

        let mut child = Command::new(&self.cmd)
            .current_dir(working_directory)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;

        let stdout = BufReader::new(child.stdout.take().unwrap());
        let stdin = child.stdin.take().unwrap();

        let mut engine = Engine {
            child,
            stdout,
            stdin,
            name: self.name.clone().unwrap_or(self.cmd.to_string()),
            builder: self.clone(),
        };

        engine.write_line("usi")?;

        loop {
            let input = engine.read_line()?;
            let mut it = input.split_whitespace();
            match it.next() {
                Some("usiok") => break,
                Some("id") => match it.next() {
                    Some("name") => {
                        if self.name.is_none()
                            && let Some(name) = it.remainder()
                        {
                            engine.name = name.trim().to_string();
                        }
                    }
                    Some("author") => {}
                    s => {
                        dbg!(s);
                    }
                },
                _ => {}
            }
        }

        for (k, v) in &self.usi_options {
            engine.write_line(&format!("setoption name {k} value {v}"))?;
        }

        info!("Engine {} started", engine.name);

        Ok(engine)
    }
    pub fn get_usi_option_value(&self, key: &str) -> Option<&str> {
        self.usi_options
            .iter()
            .filter_map(|(k, v)| if k == key { Some(v.as_ref()) } else { None })
            .next_back()
    }
}

#[derive(Debug)]
pub struct Engine {
    child: Child,
    stdout: BufReader<ChildStdout>,
    stdin: ChildStdin,
    name: String,
    builder: EngineBuilder,
}

impl Drop for Engine {
    fn drop(&mut self) {
        info!("Quitting engine {}...", self.name);
        match self.write_line("quit") {
            Ok(_) => {}
            Err(_) => error!("Failed to write quit to engine {}", self.name),
        };
        match self.child.wait_timeout(Duration::from_secs(10)) {
            Ok(Some(_)) => info!("Quit engine {} successfully", self.name),
            Ok(None) | Err(_) => {
                info!(
                    "Timed out quitting engine {}, attempting to kill...",
                    self.name
                );
                match self.child.kill() {
                    Ok(_) => info!("Engine {} killed", self.name),
                    Err(_) => info!("Failed to kill engine {}, giving up", self.name),
                }
            }
        }
    }
}

impl Engine {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn write_line(&mut self, line: &str) -> Result<()> {
        trace!("{} < {line}", self.name());
        writeln!(self.stdin, "{line}")
    }

    pub fn isready(&mut self) -> Result<()> {
        self.write_line("isready")?;
        self.flush()?;
        loop {
            // TODO: Timeout
            let line = self.read_line()?;
            if line.trim().eq_ignore_ascii_case("readyok") {
                return Ok(());
            }
        }
    }

    pub fn usinewgame(&mut self) -> Result<()> {
        self.write_line("usinewgame")?;
        self.flush()?;
        Ok(())
    }

    pub fn position(&mut self, game: &shogi::Game) -> Result<()> {
        let position = format!("position {}", game.usi_string());
        self.write_line(&position)?;
        self.flush()?;
        Ok(())
    }

    pub fn wait_for_bestmove(&mut self) -> Result<MoveRecord> {
        let mut mr = MoveRecord::default();
        loop {
            // TODO: Timeout
            let line = self.read_line()?;
            if line.trim().starts_with("info") {
                let mut it = line.trim().split(' ').skip(1);
                while let Some(tok) = it.next() {
                    match tok {
                        "string" => break,
                        "depth" => {
                            if let Some(value) = it.next()
                                && let Ok(value) = value.parse::<u32>()
                            {
                                mr.depth = value;
                            }
                        }
                        "seldepth" => {
                            if let Some(value) = it.next()
                                && let Ok(value) = value.parse::<u32>()
                            {
                                mr.seldepth = value;
                            }
                        }
                        "nodes" => {
                            if let Some(value) = it.next()
                                && let Ok(value) = value.parse::<u64>()
                            {
                                mr.nodes = value;
                            }
                        }
                        "nps" => {
                            if let Some(value) = it.next()
                                && let Ok(value) = value.parse::<u64>()
                            {
                                mr.nps = value;
                            }
                        }
                        "time" => {
                            if let Some(value) = it.next()
                                && let Ok(value) = value.parse::<u64>()
                            {
                                mr.engine_time = value;
                            }
                        }
                        "hashfull" => {
                            if let Some(value) = it.next()
                                && let Ok(value) = value.parse::<u32>()
                            {
                                mr.hashfull = value;
                            }
                        }
                        "score" => match it.next() {
                            Some(x) => match x {
                                "cp" => {
                                    if let Some(value) = it.next()
                                        && let Ok(value) = value.parse::<i32>()
                                    {
                                        mr.score = Score::Cp(value);
                                    }
                                }
                                "mate" => {
                                    if let Some(value) = it.next()
                                        && let Ok(value) = value.parse::<i32>()
                                    {
                                        mr.score = Score::Mate(value);
                                    }
                                }
                                _ => continue,
                            },
                            None => continue,
                        },
                        _ => continue,
                    }
                }
            } else if line.trim().starts_with("bestmove") {
                let mstr = line.trim().split(' ').nth(1).unwrap_or("");
                mr.mstr = mstr.to_string();
                if let Some(m) = shogi::Move::parse(mstr) {
                    mr.m = m;
                } else {
                    error!(
                        "{} (cmd={}) gave us invalid move: {mstr}",
                        self.name(),
                        self.builder.cmd
                    );
                }
                return Ok(mr);
            }
        }
    }

    pub fn flush(&mut self) -> Result<()> {
        self.stdin.flush()
    }

    pub fn read_line(&mut self) -> Result<String> {
        let mut input = String::new();
        let count = self.stdout.read_line(&mut input)?;
        if count == 0 {
            error!("{} (cmd={}) disconnected", self.name(), self.builder.cmd);
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Read 0 bytes",
            ))
        } else {
            trace!("{} > {}", self.name(), input.trim());
            Ok(input)
        }
    }
}

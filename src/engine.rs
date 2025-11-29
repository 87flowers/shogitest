use crate::shogi;
use log::{error, info, trace};
use std::{
    env,
    io::{BufRead, BufReader, Result, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::Duration,
};
use wait_timeout::ChildExt;

#[cfg(unix)]
use std::os::fd::AsRawFd;

#[derive(Debug, Clone, Default)]
pub enum Score {
    #[default]
    None,
    Cp(i32),
    Mate(i32),
}

#[derive(Debug, Copy, Clone)]
pub enum ReadResult {
    Ok,
    Timeout,
    Disconnected,
}

#[derive(Debug, Copy, Clone)]
pub enum ReadState {
    Continue,
    Stop,
}

#[derive(Debug, Clone, Default)]
pub struct MoveRecord {
    pub stm: Option<shogi::Color>,
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
            read_buf: Vec::new(),
            stdin,
            name: self.name.clone().unwrap_or(self.cmd.to_string()),
            builder: self.clone(),
        };

        engine.write_line("usi")?;

        let mut usi_name: Option<String> = None;
        engine.read_with_timeout(Some(5 * Duration::SECOND), |line| {
            let mut it = line.split_whitespace();
            match it.next() {
                Some("usiok") => ReadState::Stop,
                Some("id") => {
                    match it.next() {
                        Some("name") => {
                            if let Some(name) = it.remainder() {
                                usi_name = Some(name.trim().to_string());
                            }
                        }
                        Some("author") => {}
                        s => {
                            dbg!(s);
                        }
                    }
                    ReadState::Continue
                }
                _ => ReadState::Continue,
            }
        })?;

        if let Some(usi_name) = usi_name
            && self.name.is_none()
        {
            engine.name = usi_name;
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
    read_buf: Vec<u8>,
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
        self.read_with_timeout(Some(5 * Duration::SECOND), |line| {
            if line.trim().eq_ignore_ascii_case("readyok") {
                ReadState::Stop
            } else {
                ReadState::Continue
            }
        })?;
        Ok(())
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

    pub fn wait_for_bestmove(&mut self, timeout: Option<Duration>) -> Result<MoveRecord> {
        let mut mr = MoveRecord::default();
        self.read_with_timeout(timeout, |line| {
            let mut it = line.split_ascii_whitespace();
            match it.next() {
                Some("info") => {
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
                    ReadState::Continue
                }
                Some("bestmove") => {
                    let mstr = line.trim().split(' ').nth(1).unwrap_or("");
                    mr.mstr = mstr.to_string();
                    if let Some(m) = shogi::Move::parse(mstr) {
                        mr.m = m;
                    }
                    ReadState::Stop
                }
                _ => ReadState::Continue,
            }
        })?;
        Ok(mr)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.stdin.flush()
    }

    #[cfg(unix)]
    pub fn read_with_timeout<F>(
        &mut self,
        timeout: Option<Duration>,
        mut f: F,
    ) -> Result<ReadResult>
    where
        F: FnMut(String) -> ReadState,
    {
        let timeout_ms = match timeout {
            Some(timeout) => timeout.as_millis().clamp(0, i32::MAX as u128) as i32,
            None => -1,
        };

        loop {
            let mut fds: [libc::pollfd; 1] = unsafe { std::mem::zeroed() };
            fds[0].fd = self.stdout.get_mut().as_raw_fd();
            fds[0].events = libc::POLLIN;

            let ready_count = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as u64, timeout_ms) };
            if ready_count < 0 {
                let err = std::io::Error::last_os_error();
                match err.raw_os_error() {
                    Some(libc::EINTR) | Some(libc::EAGAIN) => continue,
                    _ => return Err(err),
                }
            }

            assert!(ready_count as usize <= fds.len());

            if ready_count == 0 {
                return Ok(ReadResult::Timeout);
            }

            let read_buf = self.stdout.fill_buf()?;
            let read_buf_len = read_buf.len();
            self.read_buf.extend_from_slice(read_buf);
            self.stdout.consume(read_buf_len);

            while let Some(i) = memchr::memchr(b'\n', self.read_buf.as_slice()) {
                let line = {
                    let line = self.read_buf.drain(0..(i + 1));
                    let Ok(line) = str::from_utf8(line.as_slice()) else {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Received Invalid UTF-8",
                        ));
                    };
                    line.to_string()
                };

                trace!("{} > {}", self.name(), line.trim());

                match f(line) {
                    ReadState::Continue => {}
                    ReadState::Stop => return Ok(ReadResult::Ok),
                }
            }
        }
    }
}

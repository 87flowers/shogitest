use crate::shogi;
use log::{error, info, trace};
use std::{
    env,
    io::{Result, Write},
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

#[derive(Debug)]
pub enum EngineResult<T> {
    Ok(T),
    Timeout,
    Disconnected,
    Err(std::io::Error),
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

        let stdout = child.stdout.take().unwrap();
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
        match engine.read_with_timeout(Some(5 * Duration::SECOND), |line| {
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
        }) {
            EngineResult::Ok(()) => {}
            EngineResult::Err(err) => return Err(err),
            EngineResult::Timeout => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("Timed-out waiting for usiok for {}", engine.name),
                ));
            }
            EngineResult::Disconnected => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    format!(
                        "Engine {} disconnected while waiting for usiok",
                        engine.name
                    ),
                ));
            }
        }

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
    stdout: ChildStdout,
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

    pub fn restart(&mut self) -> Result<()> {
        *self = self.builder.init()?;
        Ok(())
    }

    pub fn write_line(&mut self, line: &str) -> Result<()> {
        trace!("{} < {line}", self.name());
        writeln!(self.stdin, "{line}")
    }

    pub fn isready(&mut self) -> Result<()> {
        self.write_line("isready")?;
        self.flush()?;
        match self.read_with_timeout(Some(5 * Duration::SECOND), |line| {
            if line.trim().eq_ignore_ascii_case("readyok") {
                ReadState::Stop
            } else {
                ReadState::Continue
            }
        }) {
            EngineResult::Ok(()) => Ok(()),
            EngineResult::Err(err) => Err(err),
            EngineResult::Timeout => Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("Timed-out waiting for readyok for {}", self.name),
            )),
            EngineResult::Disconnected => Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "Engine {} disconnected while waiting for readyok",
                    self.name
                ),
            )),
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

    pub fn wait_for_bestmove(
        &mut self,
        stm: crate::shogi::Color,
        timeout: Option<Duration>,
    ) -> EngineResult<MoveRecord> {
        let mut mr = MoveRecord {
            stm: Some(stm),
            ..MoveRecord::default()
        };
        match self.read_with_timeout(timeout, |line| {
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
                    let mstr = it.next().unwrap_or("");
                    mr.mstr = mstr.to_string();
                    if let Some(m) = shogi::Move::parse(mstr) {
                        mr.m = m;
                    }
                    ReadState::Stop
                }
                _ => ReadState::Continue,
            }
        }) {
            EngineResult::Ok(()) => EngineResult::Ok(mr),
            EngineResult::Err(err) => EngineResult::Err(err),
            EngineResult::Timeout => EngineResult::Timeout,
            EngineResult::Disconnected => EngineResult::Disconnected,
        }
    }

    pub fn flush(&mut self) -> Result<()> {
        self.stdin.flush()
    }

    #[cfg(unix)]
    pub fn read_with_timeout<F>(&mut self, timeout: Option<Duration>, mut f: F) -> EngineResult<()>
    where
        F: FnMut(String) -> ReadState,
    {
        use std::io::Read;
        use std::os::fd::AsRawFd;

        let timeout_ms = match timeout {
            Some(timeout) => timeout.as_millis().clamp(0, i32::MAX as u128) as i32,
            None => -1,
        };

        loop {
            let mut fds: [libc::pollfd; 1] = unsafe { std::mem::zeroed() };
            fds[0].fd = self.stdout.as_raw_fd();
            fds[0].events = libc::POLLIN;

            let ready_count = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as u64, timeout_ms) };
            if ready_count < 0 {
                let err = std::io::Error::last_os_error();
                match err.raw_os_error() {
                    Some(libc::EINTR) | Some(libc::EAGAIN) => continue,
                    _ => return EngineResult::Err(err),
                }
            }

            assert!(ready_count as usize <= fds.len());

            if ready_count == 0 {
                return EngineResult::Timeout;
            }

            let count = {
                self.read_buf.reserve(4096);
                let old_len = self.read_buf.len();
                let spare_cap = self.read_buf.spare_capacity_mut();
                let spare_cap = unsafe {
                    std::slice::from_raw_parts_mut(
                        spare_cap.as_mut_ptr() as *mut u8,
                        spare_cap.len(),
                    )
                };
                match self.stdout.read(spare_cap) {
                    Err(err) => return EngineResult::Err(err),
                    Ok(count) => {
                        unsafe { self.read_buf.set_len(old_len + count) };
                        count
                    }
                }
            };

            if count == 0 {
                return EngineResult::Disconnected;
            }

            match self.process_read_buf(&mut f) {
                Ok(ReadState::Continue) => {}
                Ok(ReadState::Stop) => return EngineResult::Ok(()),
                Err(err) => return EngineResult::Err(err),
            }
        }
    }

    #[cfg(windows)]
    pub fn read_with_timeout<F>(&mut self, timeout: Option<Duration>, mut f: F) -> EngineResult<()>
    where
        F: FnMut(String) -> ReadState,
    {
        use std::os::windows::io::AsRawHandle;
        use windows::{
            Win32::Foundation::*, Win32::Storage::FileSystem::*, Win32::System::IO::*,
            Win32::System::Threading::*,
        };

        let timeout_ms = match timeout {
            Some(timeout) => timeout.as_millis().clamp(0, i32::MAX as u128) as u32,
            None => INFINITE,
        };

        loop {
            unsafe {
                let handle = HANDLE(self.stdout.as_raw_handle());

                let mut overlapped = OVERLAPPED::default();
                overlapped.hEvent =
                    CreateEventW(None, true, false, None).expect("Could not create event");

                let old_read_buf_len = self.read_buf.len();

                let write_buf = {
                    self.read_buf.reserve(4096);
                    let spare_cap = self.read_buf.spare_capacity_mut();
                    std::slice::from_raw_parts_mut(
                        spare_cap.as_mut_ptr() as *mut u8,
                        spare_cap.len(),
                    )
                };

                if let Err(err) = ReadFile(handle, Some(write_buf), None, Some(&mut overlapped))
                    && err.code() != ERROR_IO_PENDING.into()
                {
                    let _ = CloseHandle(overlapped.hEvent);
                    return EngineResult::Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("ReadFile Failed: {:?}", err),
                    ));
                }

                match WaitForSingleObject(overlapped.hEvent, timeout_ms) {
                    WAIT_TIMEOUT => {
                        let _ = CancelIo(handle);
                        let _ = CloseHandle(overlapped.hEvent);
                        return EngineResult::Timeout;
                    }
                    WAIT_OBJECT_0 => {}
                    _ => {
                        let _ = CloseHandle(overlapped.hEvent);
                        return EngineResult::Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "WaitForSingleObject Failed",
                        ));
                    }
                }

                let mut bytes_read: u32 = 0;
                if let Err(err) = GetOverlappedResult(handle, &overlapped, &mut bytes_read, false) {
                    let _ = CloseHandle(overlapped.hEvent);
                    return EngineResult::Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("GetOverlappedResult Failed: {:?}", err),
                    ));
                }

                let _ = CloseHandle(overlapped.hEvent);

                self.read_buf
                    .set_len(old_read_buf_len + bytes_read as usize);

                if bytes_read == 0 {
                    return EngineResult::Disconnected;
                }

                match self.process_read_buf(&mut f) {
                    Ok(ReadState::Continue) => {}
                    Ok(ReadState::Stop) => return EngineResult::Ok(()),
                    Err(err) => return EngineResult::Err(err),
                }
            }
        }
    }

    fn process_read_buf<F>(&mut self, mut f: F) -> Result<ReadState>
    where
        F: FnMut(String) -> ReadState,
    {
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
                ReadState::Stop => return Ok(ReadState::Stop),
            }
        }

        Ok(ReadState::Continue)
    }
}

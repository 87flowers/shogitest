use crate::shogi;
use std::{
    env,
    io::{BufRead, BufReader, Result, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct EngineBuilder {
    pub path: String,
}

impl EngineBuilder {
    pub fn init(&self) -> Result<Engine> {
        let mut absolute_path = env::current_dir()?;
        absolute_path.push(&self.path);

        let mut child = Command::new(&absolute_path)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;

        let stdout = BufReader::new(child.stdout.take().unwrap());
        let stdin = child.stdin.take().unwrap();

        let mut engine = Engine {
            child,
            stdout,
            stdin,
            name: self.path.to_string(),
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
                        if let Some(name) = it.remainder() {
                            engine.name = name.trim().to_string();
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        Ok(engine)
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

impl Engine {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn builder(&self) -> &EngineBuilder {
        &self.builder
    }

    pub fn write_line(&mut self, line: &str) -> Result<()> {
        println!("{} < {line}", self.name());
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

    pub fn wait_for_bestmove(&mut self) -> Result<Option<shogi::Move>> {
        loop {
            // TODO: Timeout
            let line = self.read_line()?;
            if line.trim().starts_with("bestmove ") {
                let mstr = line.trim().split(' ').nth(1).unwrap_or("");
                dbg!(&mstr);
                return Ok(shogi::Move::parse(mstr));
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
            println!("{} disconnected", self.name());
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Read 0 bytes",
            ))
        } else {
            println!("{} > {}", self.name(), input.trim());
            Ok(input)
        }
    }
}

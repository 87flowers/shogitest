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
        writeln!(self.stdin, "{line}")
    }

    pub fn flush(&mut self) -> Result<()> {
        self.stdin.flush()
    }

    pub fn read_line(&mut self) -> Result<String> {
        let mut input = String::new();
        let count = self.stdout.read_line(&mut input)?;
        if count == 0 {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Read 0 bytes",
            ))
        } else {
            Ok(input)
        }
    }
}

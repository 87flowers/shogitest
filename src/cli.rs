use crate::engine;

#[derive(Debug)]
pub struct CliOptions {
    pub engines: Vec<EngineOptions>,
    pub openings_file: Option<String>,
    pub rounds: Option<u64>,
    pub concurrency: u64,
}

impl Default for CliOptions {
    fn default() -> Self {
        CliOptions {
            engines: vec![],
            openings_file: None,
            rounds: None,
            concurrency: 1,
        }
    }
}

#[derive(Debug, Default)]
pub struct EngineOptions {
    pub builder: engine::EngineBuilder,
    pub time_control: String,
}

pub fn parse() -> CliOptions {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut options = CliOptions::default();
    let mut it = args.iter().peekable();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "-engine" => {
                let mut engine = EngineOptions::default();
                loop {
                    let Some(option) = it.peek() else { break };
                    if option.starts_with("-") {
                        break;
                    };
                    let Some((name, value)) = option.split_once('=') else {
                        break;
                    };
                    it.next(); // consume token

                    match name {
                        "cmd" => {
                            engine.builder.path = String::from(value);
                        }
                        "tc" => {
                            engine.time_control = String::from(value);
                        }
                        _ => {
                            dbg!(&name);
                            dbg!(&value);
                        }
                    }
                }
                options.engines.push(engine);
            }

            "-openings" => {
                loop {
                    let Some(option) = it.peek() else { break };
                    if option.starts_with("-") {
                        break;
                    };
                    let Some((name, value)) = option.split_once('=') else {
                        break;
                    };
                    it.next(); // consume token

                    match name {
                        "file" => {
                            options.openings_file = Some(String::from(value));
                        }
                        _ => {
                            dbg!(&name);
                            dbg!(&value);
                        }
                    }
                }
            }

            "-concurrency" => {
                let Some(option) = it.next() else { break };
                if let Ok(option) = option.parse::<u64>() {
                    options.concurrency = option;
                } else {
                    eprint!("invalid concurrency value {option}");
                }
            }

            "-rounds" => {
                let Some(option) = it.next() else { break };
                if let Ok(option) = option.parse::<u64>() {
                    options.rounds = Some(option);
                } else {
                    eprint!("invalid rounds value {option}");
                }
            }

            _ => {
                dbg!(&flag);
            }
        }
    }

    options
}

use crate::engine;
use crate::tc;

#[derive(Debug, Clone)]
pub struct MetaDataOptions {
    pub event_name: String,
    pub site_name: String,
}

#[derive(Debug, Clone)]
pub struct CliOptions {
    pub engines: Vec<EngineOptions>,
    pub openings_file: Option<String>,
    pub games: Option<u64>,
    pub rounds: u64,
    pub concurrency: u64,
    pub meta: MetaDataOptions,
    pub pgn: Option<PgnOutOptions>,
}

impl CliOptions {
    pub fn engine_names(&self) -> Vec<String> {
        self.engines
            .iter()
            .map(|e| e.builder.init().unwrap().name().to_string())
            .collect()
    }
}

impl Default for CliOptions {
    fn default() -> Self {
        CliOptions {
            engines: vec![],
            openings_file: None,
            games: None,
            rounds: 2,
            concurrency: 1,
            meta: MetaDataOptions {
                event_name: String::from("?"),
                site_name: String::from("?"),
            },
            pgn: None,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct EngineOptions {
    pub builder: engine::EngineBuilder,
    pub time_control: tc::TimeControl,
}

#[derive(Debug, Default, Clone)]
pub struct PgnOutOptions {
    pub file: String,
    pub track_nodes: bool,
    pub track_seldepth: bool,
    pub track_nps: bool,
    pub track_hashfull: bool,
    pub track_timeleft: bool,
    pub track_latency: bool,
}

pub fn parse() -> Option<CliOptions> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut options = CliOptions::default();
    let mut it = args.iter().peekable();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "-version" | "--version" => {
                println!("Shogitest version 0.0.0");
                return None;
            }

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
                        "name" => {
                            engine.builder.name = Some(String::from(value));
                        }
                        "dir" => {
                            engine.builder.dir = String::from(value);
                        }
                        "cmd" => {
                            engine.builder.cmd = String::from(value);
                        }
                        "tc" => {
                            if let Some(tc) = tc::TimeControl::parse(&value) {
                                engine.time_control = tc;
                            } else {
                                eprint!("Invalid time control specification {value}");
                            }
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
                    if option == 0 {
                        eprint!("invalid concurrency value {option} (must be bigger than zero)");
                        return None;
                    }
                    options.concurrency = option;
                } else {
                    eprint!("invalid concurrency value {option} (must be unsigned integer)");
                    return None;
                }
            }

            "-games" => {
                let Some(option) = it.next() else { break };
                if let Ok(option) = option.parse::<u64>() {
                    if option == 0 {
                        eprint!("invalid games value {option} (must be bigger than zero)");
                        return None;
                    }
                    options.games = Some(option);
                } else {
                    eprint!("invalid games value {option} (must be unsigned integer)");
                    return None;
                }
            }

            "-rounds" => {
                let Some(option) = it.next() else { break };
                if let Ok(option) = option.parse::<u64>() {
                    if option == 0 {
                        eprint!("invalid rounds value {option} (must be bigger than zero)");
                        return None;
                    }
                    if option > 2 && option % 2 == 1 {
                        eprint!("odd value for rounds {option}! expected an even value.");
                        return None;
                    }
                    options.rounds = option;
                } else {
                    eprint!("invalid rounds value {option} (must be unsigned integer)");
                    return None;
                }
            }

            "-repeat" => {
                options.rounds = 2;
            }

            "-pgnout" => {
                let mut pgn_out = PgnOutOptions::default();
                loop {
                    let Some(option) = it.peek() else { break };
                    if option.starts_with("-") {
                        break;
                    };
                    let Some((name, value)) = option.split_once('=') else {
                        break;
                    };
                    it.next(); // consume token

                    let value_as_bool = || -> Option<bool> {
                        match value {
                            "true" => Some(true),
                            "false" => Some(false),
                            _ => None,
                        }
                    };

                    match name {
                        "file" => {
                            pgn_out.file = String::from(value);
                        }
                        "nodes" => {
                            pgn_out.track_nodes = value_as_bool()?;
                        }
                        "seldepth" => {
                            pgn_out.track_seldepth = value_as_bool()?;
                        }
                        "nps" => {
                            pgn_out.track_nps = value_as_bool()?;
                        }
                        "hashfull" => {
                            pgn_out.track_hashfull = value_as_bool()?;
                        }
                        "timeleft" => {
                            pgn_out.track_timeleft = value_as_bool()?;
                        }
                        "latency" => {
                            pgn_out.track_latency = value_as_bool()?;
                        }
                        _ => {
                            dbg!(&name);
                            dbg!(&value);
                        }
                    }

                    if pgn_out.file.is_empty() {
                        eprintln!("output file required for -pgnout option");
                        return None;
                    }
                }
                options.pgn = Some(pgn_out);
            }

            _ => {
                dbg!(&flag);
            }
        }
    }

    Some(options)
}

use std::time::Duration;

use crate::engine;
use crate::tc;

#[derive(Debug, Clone)]
pub struct MetaDataOptions {
    pub event_name: String,
    pub site_name: String,
}

#[derive(Debug, Clone)]
pub struct BookOptions {
    pub file: String,
    pub random_order: bool,
    pub start_index: usize,
}

impl Default for BookOptions {
    fn default() -> Self {
        BookOptions {
            file: String::from("<none>"),
            random_order: false,
            start_index: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdjudicationOptions {
    pub max_moves: Option<u64>,
    pub draw: Option<DrawAdjudicationOptions>,
    pub resign: Option<ResignAdjudicationOptions>,
}

impl Default for AdjudicationOptions {
    fn default() -> Self {
        AdjudicationOptions {
            max_moves: Some(512),
            draw: None,
            resign: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DrawAdjudicationOptions {
    pub move_number: usize,
    pub move_count: usize,
    pub score: i32,
}

impl Default for DrawAdjudicationOptions {
    fn default() -> Self {
        DrawAdjudicationOptions {
            move_number: 0,
            move_count: 1,
            score: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResignAdjudicationOptions {
    pub move_count: usize,
    pub score: i32,
    pub two_sided: bool,
}

impl Default for ResignAdjudicationOptions {
    fn default() -> Self {
        ResignAdjudicationOptions {
            move_count: 1,
            score: 0,
            two_sided: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CliOptions {
    pub engines: Vec<EngineOptions>,
    pub book: Option<BookOptions>,
    pub games: Option<u64>,
    pub rounds: u64,
    pub concurrency: u64,
    pub rand_seed: Option<u64>,
    pub meta: MetaDataOptions,
    pub pgn: Option<PgnOutOptions>,
    pub adjudication: AdjudicationOptions,
    pub report_interval: Option<u64>,
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
            book: None,
            games: None,
            rounds: 2,
            concurrency: 1,
            rand_seed: None,
            meta: MetaDataOptions {
                event_name: String::from("?"),
                site_name: String::from("?"),
            },
            pgn: None,
            adjudication: AdjudicationOptions::default(),
            report_interval: Some(10),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct EngineOptions {
    pub builder: engine::EngineBuilder,
    pub time_control: tc::TimeControl,
}

#[derive(Debug, Clone)]
pub struct PgnOutOptions {
    pub file: String,
    pub track_nodes: bool,
    pub track_seldepth: bool,
    pub track_nps: bool,
    pub track_hashfull: bool,
    pub track_timeleft: bool,
    pub track_latency: bool,
}

impl Default for PgnOutOptions {
    fn default() -> Self {
        PgnOutOptions {
            file: String::default(),
            track_nodes: true,
            track_seldepth: true,
            track_nps: false,
            track_hashfull: false,
            track_timeleft: false,
            track_latency: false,
        }
    }
}

fn parse_engine_option(engine: &mut EngineOptions, name: &str, value: &str) {
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
            if engine.time_control != tc::TimeControl::None {
                eprint!("Warning; Specifying multiple time controls!");
            }
            if let Some(tc) = tc::TimeControl::parse(value) {
                engine.time_control = tc;
            } else {
                eprint!("Invalid time control specification {value}");
            }
        }
        "st" => {
            if engine.time_control != tc::TimeControl::None {
                eprint!("Warning; Specifying multiple time controls!");
            }
            match value.parse::<u64>() {
                Ok(value) => {
                    engine.time_control = tc::TimeControl::MoveTime(Duration::from_millis(value));
                }
                Err(_) => {
                    eprintln!("Expected number for st option");
                }
            }
        }
        "nodes" => {
            if engine.time_control != tc::TimeControl::None {
                eprint!("Warning; Specifying multiple time controls!");
            }
            match value.parse::<u64>() {
                Ok(value) => engine.time_control = tc::TimeControl::Nodes(value),
                Err(_) => {
                    eprintln!("Expected number for st option");
                }
            }
        }
        name if let Some(optionname) = name.strip_prefix("option.") => {
            engine
                .builder
                .usi_options
                .push((optionname.to_string(), value.to_string()));
        }
        _ => {
            dbg!(&name);
            dbg!(&value);
        }
    }
}

pub fn parse() -> Option<CliOptions> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut options = CliOptions::default();
    let mut each_options = Vec::<(String, String)>::new();

    let mut it = args.iter().peekable();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "-version" | "--version" => {
                println!("Shogitest version 0.0.0");
                return None;
            }

            "-event" => {
                let Some(value) = it.next() else { break };
                options.meta.event_name = value.to_string();
            }

            "-site" => {
                let Some(value) = it.next() else { break };
                options.meta.site_name = value.to_string();
            }

            "-engine" => {
                let mut engine = EngineOptions::default();
                while let Some(option) = it.peek()
                    && !option.starts_with("-")
                    && let Some((name, value)) = option.split_once('=')
                {
                    it.next(); // consume token

                    parse_engine_option(&mut engine, name, value);
                }
                options.engines.push(engine);
            }

            "-each" => {
                while let Some(option) = it.peek()
                    && !option.starts_with("-")
                    && let Some((name, value)) = option.split_once('=')
                {
                    it.next(); // consume token

                    each_options.push((name.to_string(), value.to_string()));
                }
            }

            "-openings" => {
                if options.book.is_some() {
                    eprintln!("Duplicate -openings flag");
                    return None;
                }

                let mut book = BookOptions::default();
                while let Some(option) = it.peek()
                    && !option.starts_with("-")
                    && let Some((name, value)) = option.split_once('=')
                {
                    it.next(); // consume token

                    match name {
                        "file" => {
                            book.file = String::from(value);
                        }
                        "order" => {
                            book.random_order = value == "random";
                        }
                        "start" => {
                            if let Ok(value) = value.parse::<usize>() {
                                if value == 0 {
                                    eprint!(
                                        "invalid opening start index {value} (must be bigger than zero)"
                                    );
                                    return None;
                                }
                                book.start_index = value;
                            } else {
                                eprint!(
                                    "invalid opening start index {value} (must be unsigned integer)"
                                );
                                return None;
                            }
                        }
                        _ => {
                            dbg!(&name);
                            dbg!(&value);
                        }
                    }
                }
                options.book = Some(book);
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

            "-srand" => {
                let Some(option) = it.next() else { break };
                if let Ok(option) = option.parse::<u64>() {
                    options.rand_seed = Some(option);
                } else {
                    eprint!("invalid random seed {option} (must be unsigned integer)");
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
                    if option % 2 != 0 {
                        eprint!("odd value for rounds {option}! expected an even value.");
                        return None;
                    }
                    if option > 2 {
                        eprint!(
                            "Warning; There is often no good reason for a round to have more than two games. (Current value: {option})"
                        );
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
                while let Some(option) = it.peek()
                    && !option.starts_with("-")
                    && let Some((name, value)) = option.split_once('=')
                {
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

            "-maxmoves" => {
                let Some(value) = it.next() else { break };
                options.adjudication.max_moves = match value.to_lowercase().as_str() {
                    "inf" | "infinite" => None,
                    _ if let Ok(value) = value.parse::<u64>()
                        && value > 0 =>
                    {
                        Some(value)
                    }
                    _ => {
                        eprint!(
                            "invalid maxmoves value {value} (must be non-zero unsigned integer)"
                        );
                        return None;
                    }
                };
            }

            "-draw" => {
                let mut draw = DrawAdjudicationOptions::default();
                while let Some(option) = it.peek()
                    && !option.starts_with("-")
                    && let Some((name, value)) = option.split_once('=')
                {
                    it.next(); // consume token

                    match name {
                        "movenumber" => {
                            draw.move_number = match value.parse::<usize>() {
                                Ok(value) => value,
                                _ => {
                                    eprintln!("Invalid movenumber {value} for -draw");
                                    return None;
                                }
                            };
                        }
                        "movecount" => {
                            draw.move_count = match value.parse::<usize>() {
                                Ok(value) if value > 0 => value,
                                _ => {
                                    eprintln!("Invalid movecount {value} for -draw");
                                    return None;
                                }
                            };
                        }
                        "score" => {
                            draw.score = match value.parse::<i32>() {
                                Ok(value) if value >= 0 => value,
                                _ => {
                                    eprintln!("Invalid score {value} for -draw");
                                    return None;
                                }
                            };
                        }
                        _ => {
                            eprintln!("Invalid key {name} for -draw");
                            return None;
                        }
                    }
                }
                options.adjudication.draw = Some(draw);
            }

            "-resign" => {
                let mut resign = ResignAdjudicationOptions::default();
                while let Some(option) = it.peek()
                    && !option.starts_with("-")
                    && let Some((name, value)) = option.split_once('=')
                {
                    it.next(); // consume token

                    match name {
                        "movecount" => {
                            resign.move_count = match value.parse::<usize>() {
                                Ok(value) if value > 0 => value,
                                _ => {
                                    eprintln!("Invalid movecount {value} for -resign");
                                    return None;
                                }
                            };
                        }
                        "score" => {
                            resign.score = match value.parse::<i32>() {
                                Ok(value) if value >= 0 => value,
                                _ => {
                                    eprintln!("Invalid score {value} for -resign");
                                    return None;
                                }
                            };
                        }
                        "twosided" => {
                            resign.two_sided = match value.to_lowercase().as_ref() {
                                "true" => true,
                                "false" => false,
                                _ => {
                                    eprintln!("Invalid boolean {value} for twosided for -resign");
                                    return None;
                                }
                            };
                        }
                        _ => {
                            eprintln!("Invalid key {name} for -resign");
                            return None;
                        }
                    }
                }
                options.adjudication.resign = Some(resign);
            }

            "-ratinginterval" => {
                let Some(option) = it.next() else { break };
                if let Ok(option) = option.parse::<u64>() {
                    options.report_interval = if option == 0 { None } else { Some(option) };
                } else {
                    eprint!("invalid games value {option} (must be unsigned integer)");
                    return None;
                }
            }

            "-testEnv" => {
                options.report_interval = None;
            }

            _ => {
                dbg!(&flag);
            }
        }
    }

    for (name, value) in each_options {
        for engine in &mut options.engines {
            parse_engine_option(engine, &name, &value);
        }
    }

    Some(options)
}

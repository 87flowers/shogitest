use crate::{cli, shogi, util};
use rand::{Rng, seq::SliceRandom};

#[derive(Debug)]
pub struct OpeningBook {
    openings: Vec<shogi::Position>,
    current: usize,
}

impl OpeningBook {
    pub fn new<R>(options: &cli::BookOptions, rng: &mut R) -> Option<OpeningBook>
    where
        R: Rng + ?Sized,
    {
        let Ok(lines) = util::read_lines(&options.file) else {
            eprintln!("Unable to read file for opening book: {}", options.file);
            return None;
        };

        let mut openings = Vec::<shogi::Position>::new();
        for line in lines.map_while(Result::ok).filter(|l| !l.trim().is_empty()) {
            let Some(position) = shogi::Position::parse(line.trim()) else {
                eprintln!("Invalid sfen: {line}");
                return None;
            };
            openings.push(position);
        }

        if options.random_order {
            // Fisher-Yates Shuffle
            openings.shuffle(rng);
        }

        let openings_len = openings.len();
        Some(OpeningBook {
            openings,
            current: (options.start_index - 1) % openings_len,
        })
    }

    pub fn current(&self) -> shogi::Position {
        self.openings[self.current]
    }

    pub fn advance(&mut self) {
        self.current = (self.current + 1) % self.openings.len();
    }
}

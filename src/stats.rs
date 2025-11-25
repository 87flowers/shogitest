const NORM_PPF_0_975: f64 = 1.959963984540054;

fn score<const N: usize>(probs: [f64; N]) -> f64 {
    probs
        .iter()
        .enumerate()
        .map(|(i, &p)| (i as f64 / (N - 1) as f64) * p)
        .sum()
}

fn variance<const N: usize>(probs: [f64; N], mu: f64) -> f64 {
    probs
        .iter()
        .enumerate()
        .map(|(i, &p)| ((i as f64 / (N - 1) as f64) - mu).powi(2) * p)
        .sum()
}

fn logistic_elo(score: f64) -> f64 {
    let score = score.clamp(1e-6, 1.0 - 1e-6);
    -400.0 * (1.0 / score - 1.0).log10()
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Wdl {
    pub w: u64,
    pub d: u64,
    pub l: u64,
}

impl std::ops::Add for Wdl {
    type Output = Wdl;

    fn add(self, other: Wdl) -> Self::Output {
        Wdl {
            w: self.w + other.w,
            d: self.d + other.d,
            l: self.l + other.l,
        }
    }
}

impl std::iter::Sum for Wdl {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Wdl::default(), |a, b| a + b)
    }
}

impl Wdl {
    pub const ONE_WIN: Wdl = Wdl { w: 1, d: 0, l: 0 };
    pub const ONE_DRAW: Wdl = Wdl { w: 0, d: 1, l: 0 };
    pub const ONE_LOSS: Wdl = Wdl { w: 0, d: 0, l: 1 };

    pub fn game_count(&self) -> u64 {
        self.w + self.d + self.l
    }

    pub fn to_probs(self) -> [f64; 3] {
        let gc = self.game_count() as f64;
        [self.l as f64 / gc, self.d as f64 / gc, self.w as f64 / gc]
    }

    pub fn points(&self) -> f64 {
        self.w as f64 * 1.0 + self.d as f64 * 0.5
    }

    pub fn score(&self) -> f64 {
        score(self.to_probs())
    }

    pub fn variance(&self) -> f64 {
        variance(self.to_probs(), self.score())
    }

    pub fn logistic_elo(&self) -> (f64, f64) {
        let score = self.score();
        let variance = self.variance();
        let per_game_variance = variance / self.game_count() as f64;
        let score_lower = score - NORM_PPF_0_975 * per_game_variance.sqrt();
        let score_upper = score + NORM_PPF_0_975 * per_game_variance.sqrt();

        let elo_lower = logistic_elo(score_lower);
        let elo = logistic_elo(score);
        let elo_upper = logistic_elo(score_upper);

        (elo, (elo_upper - elo_lower) / 2.0)
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Penta {
    pub ll: u64,
    pub dl: u64,
    pub dd: u64,
    pub wl: u64,
    pub wd: u64,
    pub ww: u64,
}

impl std::ops::Add for Penta {
    type Output = Penta;

    fn add(self, other: Penta) -> Self::Output {
        Penta {
            ll: self.ll + other.ll,
            dl: self.dl + other.dl,
            dd: self.dd + other.dd,
            wl: self.wl + other.wl,
            wd: self.wd + other.wd,
            ww: self.ww + other.ww,
        }
    }
}

impl std::iter::Sum for Penta {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Penta::default(), |a, b| a + b)
    }
}

impl Penta {
    pub const ONE_WW: Penta = Penta {
        ll: 0,
        dl: 0,
        dd: 0,
        wl: 0,
        wd: 0,
        ww: 1,
    };
    pub const ONE_WD: Penta = Penta {
        ll: 0,
        dl: 0,
        dd: 0,
        wl: 0,
        wd: 1,
        ww: 0,
    };
    pub const ONE_WL: Penta = Penta {
        ll: 0,
        dl: 0,
        dd: 0,
        wl: 1,
        wd: 0,
        ww: 0,
    };
    pub const ONE_DD: Penta = Penta {
        ll: 0,
        dl: 0,
        dd: 1,
        wl: 0,
        wd: 0,
        ww: 0,
    };
    pub const ONE_DL: Penta = Penta {
        ll: 0,
        dl: 1,
        dd: 0,
        wl: 0,
        wd: 0,
        ww: 0,
    };
    pub const ONE_LL: Penta = Penta {
        ll: 1,
        dl: 0,
        dd: 0,
        wl: 0,
        wd: 0,
        ww: 0,
    };

    pub fn flip(&self) -> Penta {
        Penta {
            ll: self.ww,
            dl: self.wd,
            dd: self.dd,
            wl: self.wl,
            wd: self.dl,
            ww: self.ll,
        }
    }

    pub fn game_count(&self) -> u64 {
        self.ll + self.dl + self.dd + self.wl + self.wd + self.ww
    }

    pub fn to_probs(self) -> [f64; 5] {
        let gc = self.game_count() as f64;
        [
            self.ll as f64 / gc,
            self.dl as f64 / gc,
            (self.dd + self.wl) as f64 / gc,
            self.wd as f64 / gc,
            self.ww as f64 / gc,
        ]
    }

    pub fn points(&self) -> f64 {
        self.dl as f64 * 0.5
            + self.dd as f64 * 1.0
            + self.wl as f64 * 1.0
            + self.wd as f64 * 1.5
            + self.ww as f64 * 2.0
    }

    pub fn score(&self) -> f64 {
        score(self.to_probs())
    }

    pub fn variance(&self) -> f64 {
        variance(self.to_probs(), self.score())
    }

    pub fn logistic_elo(&self) -> (f64, f64) {
        let score = self.score();
        let variance = self.variance();
        let per_game_variance = variance / self.game_count() as f64;
        let score_lower = score - NORM_PPF_0_975 * per_game_variance.sqrt();
        let score_upper = score + NORM_PPF_0_975 * per_game_variance.sqrt();

        let elo_lower = logistic_elo(score_lower);
        let elo = logistic_elo(score);
        let elo_upper = logistic_elo(score_upper);

        (elo, (elo_upper - elo_lower) / 2.0)
    }

    pub fn dd_wl_ratio(&self) -> f64 {
        self.dd as f64 / self.wl as f64
    }
}

impl std::fmt::Display for Penta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}, {}, {}, {}, {}]",
            self.ll,
            self.dl,
            self.dd + self.wl,
            self.wd,
            self.ww
        )
    }
}

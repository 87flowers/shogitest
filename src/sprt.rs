// References:
// [1] Michel Van den Bergh, Comments on Normalized Elo, https://www.cantate.be/Fishtest/normalized_elo_practical.pdf

use std::num::FpCategory;

use crate::stats::Penta;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SprtParameters {
    lower_bound: f64,
    upper_bound: f64,
    nelo0: f64,
    nelo1: f64,
    t0: f64,
    t1: f64,
}

impl SprtParameters {
    /// Constructs parameters to use for a SPRT test.
    /// nelo0 : Represents the H0 hypothesis that the normalized elo difference is nelo0
    /// nelo1 : Represents the H1 hypothesis that the normalized elo difference is nelo1
    /// alpha : False positive rate (Type I error)
    /// beta : False negative rate (Type II error)
    pub fn new(nelo0: f64, nelo1: f64, alpha: f64, beta: f64) -> SprtParameters {
        let c_et = 800.0 / f64::ln(10.0);
        let lower_bound = (beta / (1.0 - alpha)).ln();
        let upper_bound = ((1.0 - beta) / alpha).ln();
        let t0 = nelo0 / c_et;
        let t1 = nelo1 / c_et;
        SprtParameters {
            lower_bound,
            upper_bound,
            nelo0,
            nelo1,
            t0,
            t1,
        }
    }

    /// Bounds on LLR for SPRT termination.
    /// If LLR falls below the lower bound, that demonstrates the hypothesis that elo = elo0 is more likely.
    /// If LLR falls above the upper bound, that demonstrates the hypothesis that elo = elo1 is more likely,
    /// If LLR falls within these bounds, more data is required.
    pub fn llr_bounds(&self) -> (f64, f64) {
        (self.lower_bound, self.upper_bound)
    }

    /// Returns the elo bounds provided to the constructor
    pub fn nelo_bounds(&self) -> (f64, f64) {
        (self.nelo0, self.nelo1)
    }

    /// Calculates the LLR for the given pentanomial results, given our SPRT parameters
    pub fn llr(&self, penta: Penta) -> f64 {
        let prob = regularize(penta.to_probs());
        let count = penta.pair_count();
        llr(
            count as f64,
            prob,
            [0.0, 0.25, 0.5, 0.75, 1.0],
            self.t0 * f64::sqrt(2.0),
            self.t1 * f64::sqrt(2.0),
        )
    }

    pub fn should_terminate(&self, penta: Penta) -> bool {
        if penta.pair_count() == 0 {
            return false;
        }
        let llr = self.llr(penta);
        let (lower_bound, upper_bound) = self.llr_bounds();
        llr <= lower_bound || llr >= upper_bound
    }
}

/// Compute log-likelihood ratio for t = t0 versus t = t1.
fn llr<const N: usize>(count: f64, prob: [f64; N], score: [f64; N], t0: f64, t1: f64) -> f64 {
    let p0 = mle(prob, score, 0.5, t0);
    let p1 = mle(prob, score, 0.5, t1);
    count * mean(std::array::from_fn(|i| p1[i].ln() - p0[i].ln()), prob)
}

/// Compute the maximum likelihood estimate for a discrete
/// probability distribution that has t = (mu - mu_ref) / sigma,
/// given `self` is an empirical distribution.
///
/// See section 4.1 of [1] for details.
fn mle<const N: usize>(prob: [f64; N], score: [f64; N], mu_ref: f64, t_star: f64) -> [f64; N] {
    const THETA_EPSILON: f64 = 1e-7;
    const MLE_EPSILON: f64 = 1e-4;

    // This is an iterative method, so we need to start with
    // an initial value. As suggested in [1], we start with a
    // uniform distribution.
    let mut p = [1.0 / N as f64; N];

    loop {
        // Store our current estimate away to detect convergence.
        let prev_p = p;

        // Calculate phi.
        let (mu, variance) = mean_and_variance(score, p);
        let phi: [f64; N] = std::array::from_fn(|i| {
            let a_i = score[i];
            let sigma = variance.sqrt();
            a_i - mu_ref - 0.5 * t_star * sigma * (1.0 + ((a_i - mu) / sigma).powi(2))
        });

        // We need to find a subset of the possible solutions for theta,
        // so we need to calculate our constraints for theta.
        let u = phi
            .iter()
            .min_by(|a, b| a.partial_cmp(b).expect("unexpected NaN"))
            .unwrap();
        let v = phi
            .iter()
            .max_by(|a, b| a.partial_cmp(b).expect("unexpected NaN"))
            .unwrap();
        let min_theta = -1.0 / v;
        let max_theta = -1.0 / u;

        // Solve equation 4.9 in [1] for theta.
        let theta = itp(
            |x: f64| (0..N).map(|i| prob[i] * phi[i] / (1.0 + x * phi[i])).sum(),
            (min_theta, max_theta),
            (f64::INFINITY, -f64::INFINITY),
            0.1,
            2.0,
            0.99,
            THETA_EPSILON,
        );

        // Calculate new estimate
        p = std::array::from_fn(|i| prob[i] / (1.0 + theta * phi[i]));

        // Good enough?
        if (0..N).all(|i| (prev_p[i] - p[i]).abs() < MLE_EPSILON) {
            break;
        }
    }

    p
}

fn regularize<const N: usize>(x: [f64; N]) -> [f64; N] {
    x.map(|x| x.max(1e-3))
}

fn mean<const N: usize>(x: [f64; N], p: [f64; N]) -> f64 {
    (0..N).map(|i| p[i] * x[i]).sum()
}

fn mean_and_variance<const N: usize>(x: [f64; N], p: [f64; N]) -> (f64, f64) {
    let mu = mean(x, p);
    (mu, (0..N).map(|i| p[i] * (x[i] - mu).powi(2)).sum())
}

// I. F. D. Oliveira and R. H. C. Takahashi. 2020. An Enhancement of the Bisection Method Average Performance
// Preserving Minmax Optimality. ACM Trans. Math. Softw. 47, 1, Article 5 (March 2021).
// https://doi.org/10.1145/3423597
fn itp<F>(
    f: F,
    (mut a, mut b): (f64, f64),
    (mut f_a, mut f_b): (f64, f64),
    k_1: f64,
    k_2: f64,
    n_0: f64,
    epsilon: f64,
) -> f64
where
    F: Fn(f64) -> f64,
{
    if f_a > 0.0 {
        (a, b) = (b, a);
        (f_a, f_b) = (f_b, f_a);
    }
    assert!(f_a < 0.0 && 0.0 < f_b);

    let n_half = ((b - a).abs() / (2.0 * epsilon)).log2().ceil();
    let n_max = n_half + n_0;
    let mut i = 0;
    while (b - a).abs() > 2.0 * epsilon {
        let x_half = (a + b) / 2.0;
        let r = epsilon * f64::powf(2.0, n_max - i as f64) - (b - a) / 2.0;
        let delta = k_1 * f64::powf(b - a, k_2);

        let x_f = (f_b * a - f_a * b) / (f_b - f_a);

        let sigma = (x_half - x_f) / (x_half - x_f).abs();
        let x_t = if delta <= (x_half - x_f).abs() {
            x_f + sigma * delta
        } else {
            x_half
        };

        let x_itp = if (x_t - x_half).abs() <= r {
            x_t
        } else {
            x_half - sigma * r
        };

        let f_itp = f(x_itp);
        if f_itp.classify() == FpCategory::Zero {
            a = x_itp;
            b = x_itp;
        } else if f_itp.is_sign_negative() {
            a = x_itp;
            f_a = f_itp;
        } else {
            b = x_itp;
            f_b = f_itp;
        }

        i += 1;
    }

    (a + b) / 2.0
}

//! Thread-local random number helpers used by stochastic flow metrics (Rho8 / Rho4).

use std::cell::RefCell;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

thread_local! {
    static RNG: RefCell<StdRng> = RefCell::new(StdRng::from_entropy());
}

/// Seeds the thread-local RNG. Pass `0` to reseed from OS entropy.
pub fn seed_rand(seed: u64) {
    RNG.with(|cell| {
        let mut rng = cell.borrow_mut();
        if seed == 0 {
            *rng = StdRng::from_entropy();
        } else {
            *rng = StdRng::seed_from_u64(seed);
        }
    });
}

/// Returns a uniform random floating-point number in `[from, to)`.
#[inline]
pub fn uniform_rand_real(from: f64, to: f64) -> f64 {
    RNG.with(|cell| cell.borrow_mut().gen_range(from..to))
}

/// Returns a uniform random integer in `[from, thru]`.
#[inline]
pub fn uniform_rand_int(from: i32, thru: i32) -> i32 {
    RNG.with(|cell| cell.borrow_mut().gen_range(from..=thru))
}

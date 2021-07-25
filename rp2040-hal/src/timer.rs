//! Timers
// See [Chapter 4 Section 6](https://datasheets.raspberrypi.org/rp2040/rp2040_datasheet.pdf) for more details

use embedded_time::fractional::Fraction;
use embedded_time::{Clock, Instant};

pub struct Timer {
  device: p2040_pac::TIMER
}

impl Timer {
    pub fn new(device: p2040_pac::TIMER) -> Timer {
        Timer { device: device }
    }
}

impl Clock for Timer {
    type T = u64;

    const SCALING_FACTOR = Fraction::new(1, 1000);

    fn try_now(&self) -> Result<Instant<Self>, Error> {
        // Reading the lower 32 bits via `timelr` first latches the bits for `timehr` so that an
        // accurate time value is read.
        now: u64 = self.device.timelr.read() as u64;
        now |= (self.device.timehr.read() as u64) << 32;
        Ok(Instant::<Self>::new(now))
    }
}
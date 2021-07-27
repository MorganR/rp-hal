//! Timers
// See [Chapter 4 Section 6](https://datasheets.raspberrypi.org/rp2040/rp2040_datasheet.pdf) for more details

use crate::pac;
use embedded_time::clock::Error;
use embedded_time::duration::Microseconds;
use embedded_time::fixed_point::FixedPoint;
use embedded_time::fraction::Fraction;
use embedded_time::{Clock, Instant};

pub struct Timer {
  device: pac::TIMER
}

impl Timer {
    pub fn new(device: pac::TIMER) -> Timer {
        Timer { device: device }
    }
}

impl Clock for Timer {
    type T = u64;

    // Clock operates on microsecond precision
    const SCALING_FACTOR: Fraction = Microseconds::<u64>::SCALING_FACTOR;

    fn try_now(&self) -> Result<Instant<Self>, Error> {
        // Reading the lower 32 bits via `timelr` first latches the bits for `timehr` so that an
        // accurate time value is read. However, this becomes unsafe if both cores are reading the
        // timer concurrently (see Datasheet section 4.6.4.1). Therefore we perform a more
        // complicated read over the latchless `aw*` registers instead.
        let mut high: u32 = self.device.timerawh.read().bits();
        let mut low: u32;
        loop {
            low = self.device.timerawl.read().bits();
            let next_high: u32 = self.device.timerawh.read().bits();
            if high == next_high {
                break;
            }
            high = next_high;
        }
        Ok(Instant::<Self>::new((high as u64) << 32 | (low as u64)))
    }
}
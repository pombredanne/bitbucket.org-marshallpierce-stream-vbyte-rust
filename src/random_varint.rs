extern crate rand;

use self::rand::Rng;
use self::rand::distributions::{IndependentSample, Range};

// Evenly distributed random numbers end up biased heavily towards longer encoded byte lengths:
// there are a lot more large numbers than there are small (duh), but for exercising serialization
// code paths, we'd like many at all byte lengths. This is also arguably more representative of
// real data. This should emit values whose varint lengths are uniformly distributed across the
// whole length range.
pub struct RandomVarintEncodedLengthIter<R: Rng> {
    ranges: [Range<u32>; 4],
    range_for_picking_range: Range<usize>,
    rng: R
}

impl<R: Rng> RandomVarintEncodedLengthIter<R> {
    pub fn new(rng: R) -> RandomVarintEncodedLengthIter<R> {
        RandomVarintEncodedLengthIter {
            ranges: [
                Range::new(0, 1 << 8),
                Range::new(1 << 8, 1 << 16),
                Range::new(1 << 16, 1 << 24),
                Range::new(1 << 24, u32::max_value()) // this won't ever emit the max value, sadly
            ],
            range_for_picking_range: Range::new(0, 4),
            rng
        }
    }
}

impl<R: Rng> Iterator for RandomVarintEncodedLengthIter<R> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        // pick the range we'll use
        let value_range = self.ranges[self.range_for_picking_range.ind_sample(&mut self.rng)];

        Some(value_range.ind_sample(&mut self.rng))
    }
}

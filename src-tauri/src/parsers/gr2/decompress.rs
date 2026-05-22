//! Oodle1 decompression for GR2 sections. Ported from nwn2mdk's
//! `gr2_decompress.cpp` (originally written by berenm, Boost Software License 1.0).

use super::error::{Gr2Error, Gr2Result};

struct Parameters {
    decoded_value_max: u32,
    backref_value_max: u32,
    decoded_count: u16,
    highbit_count: u16,
    sizes_count: [u8; 4],
}

impl Parameters {
    fn read(data: &[u8]) -> Gr2Result<Self> {
        if data.len() < 12 {
            return Err(Gr2Error::DecompressFailed {
                message: "Parameters block too short".into(),
            });
        }
        let dword0 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let dword1 = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

        Ok(Self {
            decoded_value_max: dword0 & 0x1FF,
            backref_value_max: (dword0 >> 9) & 0x7FFFFF,
            decoded_count: (dword1 & 0x1FF) as u16,
            highbit_count: ((dword1 >> 19) & 0x1FFF) as u16,
            sizes_count: [data[8], data[9], data[10], data[11]],
        })
    }
}

struct Decoder<'a> {
    numer: u32,
    denom: u32,
    next_denom: u32,
    stream: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    fn new(stream: &'a [u8]) -> Self {
        let numer = if stream.is_empty() {
            0
        } else {
            u32::from(stream[0] >> 1)
        };
        Self {
            numer,
            denom: 0x80,
            next_denom: 0,
            stream,
            pos: 0,
        }
    }

    fn decode(&mut self, max: u16) -> u16 {
        while self.denom <= 0x800000 {
            self.denom <<= 8;
            let b0 = self.stream.get(self.pos).copied().unwrap_or(0);
            let b1 = self.stream.get(self.pos + 1).copied().unwrap_or(0);
            self.numer =
                (self.numer << 8) | ((u32::from(b0) << 7) & 0x80) | ((u32::from(b1) >> 1) & 0x7f);
            self.pos += 1;
        }
        self.next_denom = self.denom / u32::from(max);
        std::cmp::min((self.numer / self.next_denom) as u16, max - 1)
    }

    fn commit(&mut self, max: u16, val: u16, err: u16) -> u16 {
        self.numer -= self.next_denom * u32::from(val);
        if u32::from(val) + u32::from(err) < u32::from(max) {
            self.denom = self.next_denom * u32::from(err);
        } else {
            self.denom -= self.next_denom * u32::from(val);
        }
        val
    }

    fn decode_and_commit(&mut self, max: u16) -> u16 {
        let val = self.decode(max);
        self.commit(max, val, 1)
    }
}

struct WeighWindow {
    count_cap: usize,
    ranges: Vec<u16>,
    values: Vec<u16>,
    weights: Vec<u16>,
    weight_total: u16,
    thresh_increase: u16,
    thresh_increase_cap: u16,
    thresh_range_rebuild: u16,
    thresh_weight_rebuild: u16,
}

impl WeighWindow {
    fn new(max_value: u32, count_cap: u16) -> Self {
        let count_cap = usize::from(count_cap) + 1;
        let thresh_weight_rebuild = (32 * max_value).clamp(256, 15160) as u16;
        let thresh_increase_cap = if max_value > 64 {
            std::cmp::min(2 * max_value, u32::from(thresh_weight_rebuild) / 2 - 32) as u16
        } else {
            128
        };

        Self {
            count_cap,
            ranges: vec![0, 0x4000],
            values: vec![0],
            weights: vec![4],
            weight_total: 4,
            thresh_increase: 4,
            thresh_increase_cap,
            thresh_range_rebuild: 8,
            thresh_weight_rebuild,
        }
    }

    fn rebuild_ranges(&mut self) {
        let n = self.weights.len();
        self.ranges.resize(n, 0);
        let range_weight = (8u32 * 0x4000) / u32::from(self.weight_total);
        let mut cumulative = 0u16;
        for i in 0..n {
            self.ranges[i] = cumulative;
            cumulative += ((u32::from(self.weights[i]) * range_weight) / 8) as u16;
        }
        self.ranges.push(0x4000);

        if self.thresh_increase > self.thresh_increase_cap / 2 {
            self.thresh_range_rebuild = self.weight_total.saturating_add(self.thresh_increase_cap);
        } else {
            self.thresh_increase = self.thresh_increase.saturating_mul(2);
            self.thresh_range_rebuild = self.weight_total.saturating_add(self.thresh_increase);
        }
    }

    fn rebuild_weights(&mut self) {
        for w in &mut self.weights {
            *w /= 2;
        }
        self.weight_total = self.weights.iter().sum();

        let mut i = 1;
        while i < self.weights.len() {
            if self.weights[i] == 0 {
                let last = self.weights.len() - 1;
                self.weights.swap(i, last);
                self.values.swap(i, last);
                self.weights.pop();
                self.values.pop();
            } else {
                i += 1;
            }
        }

        if self.weights.len() > 1 {
            let max_idx = (1..self.weights.len())
                .max_by_key(|&i| self.weights[i])
                .unwrap_or(1);
            let last = self.weights.len() - 1;
            self.weights.swap(max_idx, last);
            self.values.swap(max_idx, last);
        }

        if self.weights.len() < self.count_cap && self.weights[0] == 0 {
            self.weights[0] = 1;
            self.weight_total += 1;
        }
    }

    fn try_decode(&mut self, dec: &mut Decoder) -> (bool, u16) {
        if self.weight_total >= self.thresh_range_rebuild {
            if self.thresh_range_rebuild >= self.thresh_weight_rebuild {
                self.rebuild_weights();
            }
            self.rebuild_ranges();
        }

        let value = dec.decode(0x4000);

        let mut idx = 0;
        for i in 0..self.ranges.len() - 1 {
            if self.ranges[i + 1] > value {
                idx = i;
                break;
            }
        }

        let range_start = self.ranges[idx];
        let range_end = self.ranges[idx + 1];
        dec.commit(0x4000, range_start, range_end - range_start);

        self.weights[idx] += 1;
        self.weight_total += 1;

        if idx > 0 {
            return (false, self.values[idx]);
        }

        if self.weights.len() >= self.ranges.len() && dec.decode_and_commit(2) == 1 {
            let extra_count = self.weights.len() - self.ranges.len() + 1;
            let idx2 = self.ranges.len() + dec.decode_and_commit(extra_count as u16) as usize - 1;
            if idx2 < self.weights.len() {
                self.weights[idx2] += 2;
                self.weight_total += 2;
                return (false, self.values[idx2]);
            }
        }

        self.values.push(0);
        self.weights.push(2);
        self.weight_total += 2;

        if self.weights.len() == self.count_cap {
            self.weight_total -= self.weights[0];
            self.weights[0] = 0;
        }

        (true, 0)
    }
}

struct Dictionary {
    decoded_size: u32,
    backref_size: u32,
    decoded_value_max: u32,
    backref_value_max: u32,
    lowbit_value_max: u32,
    lowbit_window: WeighWindow,
    highbit_window: WeighWindow,
    midbit_windows: Vec<WeighWindow>,
    decoded_windows: Vec<WeighWindow>,
    size_windows: Vec<WeighWindow>,
}

impl Dictionary {
    fn new(params: &Parameters) -> Self {
        let decoded_value_max = params.decoded_value_max;
        let backref_value_max = params.backref_value_max;
        let lowbit_value_max = std::cmp::min(backref_value_max + 1, 4);
        let midbit_value_max = std::cmp::min(backref_value_max / 4 + 1, 256);
        let highbit_value_max = backref_value_max / 1024 + 1;

        let lowbit_window = WeighWindow::new(lowbit_value_max - 1, lowbit_value_max as u16);
        let highbit_window = WeighWindow::new(highbit_value_max - 1, params.highbit_count + 1);

        let midbit_windows = (0..highbit_value_max)
            .map(|_| WeighWindow::new(midbit_value_max - 1, midbit_value_max as u16))
            .collect();

        let decoded_windows = (0..4)
            .map(|_| WeighWindow::new(decoded_value_max - 1, params.decoded_count))
            .collect();

        let mut size_windows = Vec::with_capacity(65);
        for i in 0..4 {
            for _ in 0..16 {
                size_windows.push(WeighWindow::new(64, u16::from(params.sizes_count[3 - i])));
            }
        }
        size_windows.push(WeighWindow::new(64, u16::from(params.sizes_count[0])));

        Self {
            decoded_size: 0,
            backref_size: 0,
            decoded_value_max,
            backref_value_max,
            lowbit_value_max,
            lowbit_window,
            highbit_window,
            midbit_windows,
            decoded_windows,
            size_windows,
        }
    }

    fn decompress_block(
        &mut self,
        dec: &mut Decoder,
        dbuf: &mut [u8],
        dpos: usize,
    ) -> Result<u32, ()> {
        let sw_idx = std::cmp::min(self.backref_size as usize, self.size_windows.len() - 1);
        let (is_new, mut size_val) = self.size_windows[sw_idx].try_decode(dec);
        if is_new {
            size_val = dec.decode_and_commit(65);
            let last = self.size_windows[sw_idx].values.len() - 1;
            self.size_windows[sw_idx].values[last] = size_val;
        }
        self.backref_size = u32::from(size_val);

        if self.backref_size > 0 {
            let actual_size = if self.backref_size < 61 {
                self.backref_size + 1
            } else if self.backref_size <= 64 {
                [128, 192, 256, 512][(self.backref_size - 61) as usize]
            } else {
                return Err(());
            };

            let backref_range = std::cmp::min(self.backref_value_max, self.decoded_size);

            let (is_new_lo, mut lo) = self.lowbit_window.try_decode(dec);
            if is_new_lo {
                lo = dec.decode_and_commit(self.lowbit_value_max as u16);
                let last = self.lowbit_window.values.len() - 1;
                self.lowbit_window.values[last] = lo;
            }

            let (is_new_hi, mut hi) = self.highbit_window.try_decode(dec);
            if is_new_hi {
                hi = dec.decode_and_commit((backref_range / 1024 + 1) as u16);
                let last = self.highbit_window.values.len() - 1;
                self.highbit_window.values[last] = hi;
            }

            let mid_idx = usize::from(hi);
            let (is_new_mid, mut mid) = if mid_idx < self.midbit_windows.len() {
                self.midbit_windows[mid_idx].try_decode(dec)
            } else {
                (true, 0)
            };
            if is_new_mid {
                mid = dec.decode_and_commit(std::cmp::min(backref_range / 4 + 1, 256) as u16);
                if mid_idx < self.midbit_windows.len() {
                    let last = self.midbit_windows[mid_idx].values.len() - 1;
                    self.midbit_windows[mid_idx].values[last] = mid;
                }
            }

            let backref_offset = (u32::from(hi) << 10) + (u32::from(mid) << 2) + u32::from(lo) + 1;

            self.decoded_size += actual_size;

            let backref_offset_usize = backref_offset as usize;
            if backref_offset_usize > dpos || dpos + actual_size as usize > dbuf.len() {
                return Err(());
            }

            let src_start = dpos - backref_offset_usize;
            let copy_len = actual_size as usize;
            for i in 0..copy_len {
                dbuf[dpos + i] = dbuf[src_start + (i % backref_offset_usize)];
            }

            Ok(actual_size)
        } else {
            let alignment = dpos % 4;
            let (is_new_dec, mut val) = self.decoded_windows[alignment].try_decode(dec);
            if is_new_dec {
                val = dec.decode_and_commit(self.decoded_value_max as u16);
                let last = self.decoded_windows[alignment].values.len() - 1;
                self.decoded_windows[alignment].values[last] = val;
            }
            if dpos < dbuf.len() {
                dbuf[dpos] = (val & 0xff) as u8;
            }
            self.decoded_size += 1;
            Ok(1)
        }
    }
}

pub fn gr2_decompress(
    compressed: &[u8],
    stop0: u32,
    stop1: u32,
    decompressed_size: u32,
) -> Gr2Result<Vec<u8>> {
    if compressed.is_empty() {
        return Ok(vec![0u8; decompressed_size as usize]);
    }

    if compressed.len() < 36 {
        return Err(Gr2Error::DecompressFailed {
            message: "Compressed data too short for parameter blocks".into(),
        });
    }

    let params = [
        Parameters::read(&compressed[0..12])?,
        Parameters::read(&compressed[12..24])?,
        Parameters::read(&compressed[24..36])?,
    ];

    let mut dec = Decoder::new(&compressed[36..]);
    let mut output = vec![0u8; decompressed_size as usize];
    let steps = [stop0, stop1, decompressed_size];

    let mut dpos: usize = 0;
    for (pass, &step) in steps.iter().enumerate() {
        let mut dict = Dictionary::new(&params[pass]);
        while dpos < step as usize {
            match dict.decompress_block(&mut dec, &mut output, dpos) {
                Ok(bytes_written) => dpos += bytes_written as usize,
                Err(()) => {
                    return Err(Gr2Error::DecompressFailed {
                        message: format!(
                            "Decompression error at pass {pass}, offset {dpos}/{decompressed_size}"
                        ),
                    });
                }
            }
        }
    }

    Ok(output)
}

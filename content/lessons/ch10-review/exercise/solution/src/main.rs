mod jitter {
    pub fn delay_ms(max_ms: u32) -> u32 {
        rand::random_range(0..=max_ms)
    }
}

mod retry {
    use crate::jitter;

    pub fn plan_delays(max_ms: u32, count: usize) -> Vec<u32> {
        let mut delays = Vec::new();
        for _ in 0..count {
            delays.push(jitter::delay_ms(max_ms));
        }
        delays
    }
}

fn main() {
    let delays = retry::plan_delays(500, 5);
    println!("{delays:?}");
}

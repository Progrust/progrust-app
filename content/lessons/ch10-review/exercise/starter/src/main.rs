mod jitter {
    fn delay_ms(max_ms: u32) -> u32 {
        // ここを書く（いまは仮の値を返している）
        0
    }
}

mod retry {
    fn plan_delays(max_ms: u32, count: usize) -> Vec<u32> {
        // ここを書く（いまは仮の値を返している）
        Vec::new()
    }
}

fn main() {
    let delays = retry::plan_delays(500, 5);
    println!("{delays:?}");
}

mod dice {
    fn roll(sides: u32) -> u32 {
        // ここを書く（いまは仮の値を返している）
        0
    }
}

mod game {
    fn roll_all(sides: u32, count: usize) -> Vec<u32> {
        // ここを書く（いまは仮の値を返している）
        Vec::new()
    }
}

fn main() {
    let rolls = game::roll_all(6, 5);
    println!("{rolls:?}");
}

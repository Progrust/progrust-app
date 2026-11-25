mod dice {
    pub fn roll(sides: u32) -> u32 {
        rand::random_range(1..=sides)
    }
}

mod game {
    use crate::dice;

    pub fn roll_all(sides: u32, count: usize) -> Vec<u32> {
        let mut results = Vec::new();
        for _ in 0..count {
            results.push(dice::roll(sides));
        }
        results
    }
}

fn main() {
    let rolls = game::roll_all(6, 5);
    println!("{rolls:?}");
}

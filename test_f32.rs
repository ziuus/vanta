fn main() {
    let now = 1750000000.0_f64;
    let a = (now * 1.2) as f32;
    let a2 = ((now + 0.033) * 1.2) as f32;
    println!("{} {}", a, a2);
}

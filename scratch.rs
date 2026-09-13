fn main() {
    let levels = [0, 0x01, 0x03, 0x07, 0x47, 0x4F, 0x5F, 0x7F, 0xFF];
    for &l in &levels {
        print!("{}", std::char::from_u32(0x2800 + l as u32).unwrap());
    }
    println!();
}

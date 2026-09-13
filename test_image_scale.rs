fn main() {
    let w = 100;
    let h = 100;
    let term_w = 40;
    let term_h = 40;
    
    let scale = (w as f32 / term_w as f32)
        .max(h as f32 / term_h as f32)
        .max(1.0);
    let out_w = (w as f32 / scale) as usize;
    let out_h = (h as f32 / scale) as usize;
    
    println!("scale: {}, out_w: {}, out_h: {}", scale, out_w, out_h);
}

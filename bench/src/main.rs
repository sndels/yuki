use std::io::prelude::*;
use std::time::Instant;
use yuki::matrix::Matrix4x4;

const ITERATIONS: usize = 5000000;

fn bench_full(m: &Matrix4x4<f32>) {
    let mut m = m.clone();
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        m = m.inverted();
        if m.m[0][0].is_nan() {
            panic!("We only wanted to force the loop to be executed!")
        }
    }
    let elapsed_ns = start.elapsed().as_nanos();
    let elapsed_ms = (elapsed_ns as f64) * 1e-6;
    let us_per_invert = (elapsed_ns as f64) * 1e-3 / (ITERATIONS as f64);
    println!(
        "Full     took {:4.1} ms total, {:0.4} us per invert",
        elapsed_ms, us_per_invert
    );
}

fn bench_mul(m: &Matrix4x4<f32>) {
    let mut m = m.clone();
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        m = &m * &m;
        if m.m[0][0] == 0.0 {
            panic!("We only wanted to force the loop to be executed!")
        }
    }
    let elapsed_ns = start.elapsed().as_nanos();
    let elapsed_ms = (elapsed_ns as f64) * 1e-6;
    let us_per_invert = (elapsed_ns as f64) * 1e-3 / (ITERATIONS as f64);
    println!(
        "Full     took {:4.1} ms total, {:0.4} us per invert",
        elapsed_ms, us_per_invert
    );
}

fn main() {
    let s = Matrix4x4::new([
        [2.0, 0.0, 0.0, 0.0],
        [0.0, 3.0, 0.0, 0.0],
        [0.0, 0.0, 4.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);
    let r = Matrix4x4::new([
        [-0.6024969, 0.6975837, -0.3877816, 0.0],
        [-0.1818856, -0.5930915, -0.7843214, 0.0],
        [-0.7771198, -0.4020193, 0.4842162, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);
    let t = Matrix4x4::new([
        [1.0, 0.0, 0.0, 2.0],
        [0.0, 1.0, 0.0, 3.0],
        [0.0, 0.0, 1.0, 4.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);
    let p = Matrix4x4::new([
        [0.2 / 0.6, 0.0, 0.0, 0.0],
        [0.0, 0.2, 0.0, 0.0],
        [0.0, 0.0, 2.0, 4.0],
        [0.0, 0.0, -1.0, 0.0],
    ]);

    println!("Identity");
    bench_full(&Matrix4x4::identity());

    println!("S");
    bench_full(&s);

    println!("SR");
    let sr = &r * &s;
    bench_full(&sr);

    println!("SRT");
    let srt = &t * &(&r * &s);
    bench_full(&srt);

    println!("SRTP");
    let srtp = &p * &(&t * &(&r * &s));
    bench_full(&srtp);

    println!("Mul");
    bench_mul(&srtp);

    println!("Press enter to quit...");
    // Read a single byte and discard
    let _ = std::io::stdin().read(&mut [0u8]).unwrap();
}

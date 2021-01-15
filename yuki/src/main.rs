use std::io::prelude::*;

use yuki::math::point::Point2;

fn main() {
    let p = Point2::new(2, 3);
    println!("Hello {:?}", p);

    println!("Press enter to quit...");
    // Read a single byte and discard
    let _ = std::io::stdin().read(&mut [0u8]).unwrap();
}

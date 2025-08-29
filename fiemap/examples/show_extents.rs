extern crate fiemap;

use std::env::args;
use std::io::Error;

fn main() -> Result<(), Error> {
    for f in args().skip(1) {
        println!("{}:", f);
        for fe in fiemap::fiemap(f)? {
            println!("  {:?}", fe?);
        }
    }

    Ok(())
}

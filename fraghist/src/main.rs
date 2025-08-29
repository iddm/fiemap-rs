use std::env::args;
use std::fmt::Display;
use std::io::Error;
use std::path::Path;

use histo::Histogram;
use walkdir::WalkDir;

fn process_entry(histogram: &mut Histogram, entry: &walkdir::DirEntry) -> Result<(), Error> {
    if entry.file_type().is_file() {
        let mut count = 0;
        for fe in fiemap::fiemap(entry.path())? {
            fe?;
            count += 1;
        }
        histogram.add(count as u64);
    }
    Ok(())
}

fn process<P: AsRef<Path> + Display>(dir: P) {
    let mut histogram = Histogram::with_buckets(10);

    for entry in WalkDir::new(dir.as_ref()).same_file_system(true) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("{}: Error {:?}", dir, e);
                continue;
            }
        };
        if let Err(e) = process_entry(&mut histogram, &entry) {
            eprintln!("{}: Error {:?}", entry.path().display(), e);
        }
    }
    println!("{}:\n{}\n", dir, histogram);
}

fn main() {
    for f in args().skip(1) {
        process(f);
    }
}

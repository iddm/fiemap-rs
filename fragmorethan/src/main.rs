use std::env::args;
use std::fmt::Display;
use std::io::Error;
use std::path::Path;

use walkdir::WalkDir;

fn process_entry(entry: &walkdir::DirEntry) -> Result<usize, Error> {
    let mut count = 0;
    if entry.file_type().is_file() {
        for fe in fiemap::fiemap(entry.path())? {
            fe?;
            count += 1;
        }
    }
    Ok(count)
}

fn process<P: AsRef<Path> + Display>(dir: P, gt: usize) {
    for entry in WalkDir::new(dir.as_ref()).same_file_system(true) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!("{}: Error {:?}", dir, e);
                continue;
            }
        };

        let count = match process_entry(&entry) {
            Ok(count) => count,
            Err(e) => {
                eprintln!("{}: Error {:?}", entry.path().display(), e);
                continue;
            }
        };

        if count > gt {
            println!("{}: {}", entry.path().display(), count);
        }
    }
}

fn main() {
    let gt = args().nth(1).unwrap().parse().unwrap();
    for f in args().skip(2) {
        process(f, gt);
    }
}

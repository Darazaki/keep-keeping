#[macro_use]
extern crate clap;

use std::cmp::Ordering;
use std::path::Path;
use std::process::exit;

fn main() {
    let matches: clap::ArgMatches = clap_app!(("Keep Keeping CLI") =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: "Synchronizes directories together")
        (@arg DIRS: +required ... "Directories to synchronize")
    )
    .get_matches();

    let dirs: Vec<_> = matches.values_of("DIRS").unwrap_or_default().collect();

    match dirs.len().cmp(&2) {
        Ordering::Less => {
            eprintln!("You must precise at least 2 directories to synchronize.");
            exit(1);
        }
        Ordering::Greater => {
            eprintln!("Synchronizing more than 2 directories is not supported yet.");
            exit(1);
        }
        Ordering::Equal => synchronize_or_exit(dirs[0], dirs[1]),
    }
}

#[inline]
fn synchronize_or_exit(dir1_str: &str, dir2_str: &str) {
    let ok1 = check_dir(dir1_str);
    let ok2 = check_dir(dir2_str);

    if ok1 && ok2 {
        let dir1 = Path::new(dir1_str);
        let dir2 = Path::new(dir2_str);

        if let Err(err) = keep_keeping::synchronize(dir1, dir2) {
            eprintln!("{}", err);
            exit(1);
        }
    } else {
        exit(1);
    }
}

#[inline]
fn check_dir(dir_str: &str) -> bool {
    let dir = Path::new(dir_str);

    if !dir.exists() {
        eprintln!("Directory does not exist: '{}'", dir_str);
        false
    } else if dir.is_dir() {
        eprintln!("Path is not a directory: '{}'", dir_str);
        false
    } else {
        true
    }
}

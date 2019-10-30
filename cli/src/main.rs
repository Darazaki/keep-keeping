#[macro_use]
extern crate clap;

use keep_keeping_lib as keep_keeping;

use std::cmp::Ordering;
use std::path::Path;
use std::process::exit;

fn main() {
    let matches: clap::ArgMatches = clap_app!(("Keep Keeping CLI") =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: "Synchronizes paths together")
        (@arg PATHS: +required ... "Paths to synchronize")
    )
    .get_matches();

    let paths: Vec<_> = matches.values_of("PATHS").unwrap_or_default().collect();

    match paths.len().cmp(&2) {
        Ordering::Less => {
            eprintln!("You must precise at least 2 paths to synchronize.");
            exit(1);
        }
        Ordering::Greater => {
            eprintln!("Synchronizing more than 2 paths is not supported yet.");
            exit(1);
        }
        Ordering::Equal => synchronize_or_exit(paths[0], paths[1]),
    }
}

#[inline]
fn synchronize_or_exit(path1_str: &str, path2_str: &str) {
    let path1 = Path::new(path1_str);
    let path2 = Path::new(path2_str);

    let path1_exists = path1.exists();
    let path2_exists = path2.exists();

    let on_err = |err: &dyn std::error::Error| {
        eprintln!("Error: {}", err);

        keep_keeping::ErrorHandlingType::Fail
    };

    if path1_exists && path2_exists {
        if keep_keeping::synchronize(path1, path2, on_err).is_err() {
            exit(1);
        }
    } else {
        if !path1_exists {
            eprintln!("Path does not exist: '{}'", path1_str);
        }

        if !path2_exists {
            eprintln!("Path does not exist: '{}'", path2_str);
        }

        exit(1);
    }
}

#![forbid(unsafe_code)]

use filetime::FileTime;
use itertools::Itertools;
use std::path::Path;
use walkdir::*;

macro_rules! unused {
    ( $($x:ident), * ) => {
        {$( let _ = $x; )*}
    };
}

pub fn synchronize(dir1: &str, dir2: &str) {
    const DIR1_ID: i8 = 0;
    const DIR2_ID: i8 = 1;

    let dir_iterator = WalkDir::new(dir1)
        .into_iter()
        .map(|e| (DIR1_ID, e))
        .chain(WalkDir::new(dir2).into_iter().map(|e| (DIR2_ID, e)))
        .filter_map(|(dir_id, e)| {
            if let Ok(e) = e {
                // if e is ok
                if let Some(path) = e.path().to_str() {
                    // if path is valid UTF-8
                    if dir_id == DIR1_ID {
                        // if dir1
                        if let Some(path) = trim_base_path(dir1, path) {
                            // make path relative
                            if path != "" {
                                // if not dir1 path
                                Some((DIR1_ID, path))
                            } else {
                                // path is dir1 path => skip
                                None
                            }
                        } else {
                            // not valid UTF-8 (should never happen since it's already checked for)
                            None
                        }
                    } else if let Some(path) = trim_base_path(dir2, path) {
                        // if dir1; make path relative
                        if path != "" {
                            // if not dir1 path
                            Some((DIR2_ID, path))
                        } else {
                            // path is dir1 path => skip
                            None
                        }
                    } else {
                        // not valid UTF-8 (should never happen since it's already checked for)
                        None
                    }
                } else {
                    // path is not valid UTF-8
                    None
                }
            } else {
                // e is not ok
                None
            }
        })
        .unique_by(|tup| tup.1.clone()); // never synchronize the same path twice

    // `&str` to `String` conversion, we will need to use the `+` concatenation operator that is available
    // for `String` but not for `&str`.
    let dir1 = String::from(dir1);
    let dir2 = String::from(dir2);

    for (dir_id, relative_path) in dir_iterator {
        // `relative_path` is in the format "/relative/path",
        // `dir1` and `dir2` are in the format "/path/to/dir".
        let path_in_dir1 = dir1.clone() + &relative_path;
        let path_in_dir2 = dir2.clone() + &relative_path;

        // `String` to `Path` conversion, we will do path operation with those strings so it's more
        // efficient to convert them now.
        let path_in_dir1 = Path::new(&path_in_dir1);
        let path_in_dir2 = Path::new(&path_in_dir2);

        // `path_in_dir` is where the element is in the scanned directory,
        // `path_in_other_dir` is where the element should be in the other directory.
        let (path_in_dir, path_in_other_dir) = if dir_id == DIR1_ID {
            (path_in_dir1, path_in_dir2)
        } else {
            (path_in_dir2, path_in_dir1)
        };

        if path_in_dir.is_file() {
            if path_in_other_dir.is_file() {
                // `path_in_other_dir` exists and points to a file
                // Check timestamps, and overwrite the older with the recent one.

                let time_in_dir = FileTime::from_last_modification_time(
                    &std::fs::metadata(path_in_dir).expect("This should never happen"),
                );

                let time_in_other_dir = FileTime::from_last_modification_time(
                    &std::fs::metadata(path_in_other_dir).expect("This should never happen"),
                );

                let (source_path, target_path) = if time_in_dir > time_in_other_dir {
                    (path_in_dir, path_in_other_dir)
                } else if time_in_dir < time_in_other_dir {
                    (path_in_other_dir, path_in_dir)
                } else {
                    continue; // already synchronized => skip
                };

                if let Some(parent_path) = target_path.parent() {
                    if !parent_path.exists() {
                        // should be created before => should never happen
                        if let Err(err) = std::fs::create_dir_all(parent_path) {
                            eprintln!("Error while creating directories: {}", err);
                            continue; // failure => skip
                        }
                    }
                }

                if let Err(err) = std::fs::copy(source_path, target_path) {
                    eprintln!("Error while copying file: {}", err);
                }
            } else {
                // path does not exist in other dir
                if let Some(parent_path) = path_in_other_dir.parent() {
                    if !parent_path.exists() {
                        if let Err(err) = std::fs::create_dir_all(parent_path) {
                            eprintln!("Error while creating directories: {}", err);
                            continue; // skip
                        }
                    }
                }

                if let Err(err) = std::fs::copy(path_in_dir, path_in_other_dir) {
                    eprintln!("Error while copying file: {}", err);
                }
            }
        } else if !path_in_other_dir.exists() {
            // path in dir is a dir
            if let Err(err) = std::fs::create_dir(path_in_other_dir) {
                eprintln!("Error while creating directory: {}", err);
            }
        }
    }
}

fn trim_base_path(base_path: &str, entry_path: &str) -> Option<String> {
    let mut base_bytes = base_path.bytes();
    let entry_bytes = entry_path.bytes();

    let trimmed_path_bytes: Vec<_> = entry_bytes
        .skip_while(|b| Some(*b) == base_bytes.next())
        .collect();

    if let Ok(trimmed_path) = String::from_utf8(trimmed_path_bytes) {
        Some(trimmed_path)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn trim_base_path_test() {
        use super::trim_base_path;
        assert_eq!(
            trim_base_path("/Hi/there", "/Hi/there/you").expect("Invalid Unicode"),
            "/you"
        );
    }
}

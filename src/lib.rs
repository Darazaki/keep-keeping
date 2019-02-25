use filetime::FileTime;
use std::path::{Path, PathBuf};
use std::fs;
use walkdir::*;

type SyncResult<'t> = std::result::Result<(), &'t std::error::Error>;

#[allow(unused_macros)]
macro_rules! unused {
    ( $($x:ident), * ) => {
        {$( let _ = $x; )*}
    };
}

fn trim_base_path(base_path: &str, entry_path: &str) -> Option<PathBuf> {
    let mut base_bytes = base_path.bytes();
    let entry_bytes = entry_path.bytes();

    let trimmed_path_bytes: Vec<_> = entry_bytes
        .skip_while(|b| Some(*b) == base_bytes.next())
        .skip(1)
        .collect();

    if let Ok(trimmed_path) = String::from_utf8(trimmed_path_bytes) {
        Some(PathBuf::from(trimmed_path))
    } else {
        None
    }
}

pub fn synchronize<'r>(path1: &Path, path2: &Path) -> SyncResult<'r> {
    if path1.is_dir() {
        if path2.is_dir() {
            // path1 & path2: dir
            synchronize_dirs(path1, path2)
        } else {
            // path1: dir, path2: file
            synchronize_file_with_dir(path2, path1)
        }
    } else if path2.is_file() {
        // path1 & path2: file
        synchronize_files(path1, path2)
    } else {
        // path1: file, path2: dir
        synchronize_file_with_dir(path1, path2)
    }
}

/*
const DIR1_NOT_SYMLINK_ID: u8 = 0;
const DIR2_NOT_SYMLINK_ID: u8 = 1;
const DIR1_SYMLINK_ID: u8 = 2;
const DIR2_SYMLINK_ID: u8 = 3;
*/

macro_rules! some_or_return {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => return None,
        }
    };
}

#[inline]
fn id_and_relative_path_from_dir_entry<'r>(
    entry: &'r Result<DirEntry>,
    base_path: &'r Path,
    dir_id_no_symlink: u8,
) -> Option<(u8, PathBuf)> {
    match entry {
        Err(err) => {
            eprintln!("{}", err);
            None
        }
        Ok(entry) => {
            let dir_id = if entry.path_is_symlink() { 2 } else { 0 } + dir_id_no_symlink;
            let path: &Path = entry.path();

            let path_str = some_or_return!(path.to_str());
            let base_path_str = some_or_return!(base_path.to_str());
            let trimmed = some_or_return!(trim_base_path(base_path_str, path_str));

            Some((dir_id, trimmed))
        }
    }
}

#[inline]
fn synchronize_dirs<'r>(dir1: &Path, dir2: &Path) -> SyncResult<'r> {
    let dir_iterator = WalkDir::new(dir1)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| id_and_relative_path_from_dir_entry(&e, dir1, 0))
        .chain(
            WalkDir::new(dir2)
                .min_depth(1)
                .into_iter()
                .filter_map(|e| id_and_relative_path_from_dir_entry(&e, dir2, 1))
                // never synchronize the same path twice
                .filter(|(_, rel_path)| !dir1.join(rel_path).exists()),
        );

    for (dir_id, relative_path) in dir_iterator {
        let path_in_dir1 = dir1.join(&relative_path);
        let path_in_dir2 = dir2.join(&relative_path);

        // `path_in_dir` is where the element is in the scanned directory,
        // `path_in_other_dir` is where the element should be in the other directory.
        let (path_in_dir, path_in_other_dir) = if dir_id % 2 == 0 {
            (path_in_dir1, path_in_dir2)
        } else {
            (path_in_dir2, path_in_dir1)
        };

        if path_in_dir.is_file() {
            if path_in_other_dir.is_file() {
                // `path_in_other_dir` exists and points to a file
                // Check timestamps, and overwrite the older with the recent one.

                synchronize_files(&path_in_dir, &path_in_other_dir)?;
            } else if path_in_other_dir.is_dir() {
                synchronize_file_with_dir(&path_in_dir, &path_in_other_dir)?;
            } else {
                // path does not exist in other dir
                if let Some(parent_path) = path_in_other_dir.parent() {
                    if !parent_path.exists() {
                        if let Err(err) = std::fs::create_dir_all(parent_path) {
                            eprintln!("{}", &err);
                            continue; // skip
                        }
                    }
                }

                if let Err(err) = std::fs::copy(path_in_dir, path_in_other_dir) {
                    eprintln!("{}", &err);
                }
            }
        } else if !path_in_other_dir.exists() {
            // path in dir is a dir
            if let Err(err) = std::fs::create_dir(path_in_other_dir) {
                eprintln!("{}", &err);
            }
        } else {
            // path_in_dir: dir, path_in_other_dir: file

            synchronize_file_with_dir(&path_in_other_dir, &path_in_dir)?;
        }
    }

    Ok(())
}

#[inline]
fn synchronize_files<'r>(path1: &Path, path2: &Path) -> SyncResult<'r> {
    let time_in_dir = FileTime::from_last_modification_time(
        &std::fs::metadata(path1).expect("This should never happen"),
    );

    let time_in_other_dir = FileTime::from_last_modification_time(
        &std::fs::metadata(path2).expect("This should never happen"),
    );

    let (source_path, target_path) = if time_in_dir > time_in_other_dir {
        (path1, path2)
    } else if time_in_dir < time_in_other_dir {
        (path2, path1)
    } else {
        return Ok(()); // already synchronized => skip
    };

    if let Some(parent_path) = target_path.parent() {
        if !parent_path.exists() {
            // should be created before => should never happen
            if let Err(err) = std::fs::create_dir_all(parent_path) {
                eprintln!("{}", &err);
                return Ok(()); // failure => skip
            }
        }
    }

    if let Err(err) = std::fs::copy(source_path, target_path) {
        eprintln!("{}", &err);
    }

    Ok(())
}

#[inline]
fn synchronize_file_with_dir<'r>(file_path: &Path, dir_path: &Path) -> SyncResult<'r> {
    macro_rules! unwrap_result {
        ($e:expr) => {
            match $e {
                Err(err) => return {
                    eprintln!("{}", err);
                    Ok(())
                },
                Ok(x) => x,
            }
        };
    }

    let file_time = FileTime::from_last_modification_time(
        &unwrap_result!(file_path.metadata())
    );

    let dir_time = match dir_latest_modification_time(dir_path) {
        Ok(x) => x,
        Err(err) => return {
            eprintln!("{}", err);
            Ok(())
        },
    };

    if file_time > dir_time {
        unwrap_result!(fs::remove_dir_all(dir_path));
        unwrap_result!(fs::copy(file_path, dir_path));
    }

    Ok(())
}

#[inline]
fn dir_latest_modification_time<'r, 't>(
    path: &'t Path,
) -> std::result::Result<FileTime, &'r std::error::Error> {
    let result_err: Option<&std::error::Error> = None;

    let max = WalkDir::new(path)
        .into_iter()
        .filter_map(|e: walkdir::Result<DirEntry>| {
            macro_rules! unwrap_result {
                ($e:expr) => {
                    match $e {
                        Err(err) => {
                            eprintln!("{}", &err);
                            return None;
                        },
                        Ok(x) => x,
                    }
                };
            }

            let e = unwrap_result!(e);

            let path: &Path = e.path();
            
            Some(FileTime::from_last_modification_time(
                &unwrap_result!(path.metadata())
            ))
        })
        .max().unwrap_or_else(FileTime::zero);
    
    match result_err {
        Some(err) => Err(err),
        None => Ok(max),
    }
}

#[cfg(test)]
mod tests {
    
}

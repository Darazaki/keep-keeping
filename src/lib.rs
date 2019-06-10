#![forbid(unsafe_code)]

use filetime::FileTime;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

/// Precise how should an error be handled.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ErrorHandlingType {
    /// Stop synchronizing.
    Fail,
    /// Skip the current element and continue synchronizing.
    Skip,
    /// Continue as if no error happened.
    Ignore,
}

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

    if base_bytes.next().is_none() {
        if let Ok(trimmed_path) = String::from_utf8(trimmed_path_bytes) {
            Some(PathBuf::from(trimmed_path))
        } else {
            None
        }
    } else {
        None
    }
}

#[inline]
fn path_has_extension(path: &Path, extension: &str) -> bool {
    path.extension() == Some(std::ffi::OsStr::new(extension))
}

#[inline]
fn is_mac_app(path: &Path) -> bool {
    // A macOS app must have the 'app' extension and be a directory.
    path_has_extension(path, "app") && path.is_dir()
}

fn is_part_of_mac_app(path: &Path) -> bool {
    let mut current_path_option = path.parent();

    while let Some(current_path) = current_path_option {
        if is_mac_app(current_path) {
            return true;
        }

        current_path_option = current_path.parent();
    }

    false
}

pub fn synchronize<FErr>(path1: &Path, path2: &Path, on_err: FErr) -> Result<(), ()>
where
    FErr: Fn(&dyn std::error::Error) -> ErrorHandlingType,
{
    if path1.is_dir() {
        if path2.is_dir() {
            // path1 & path2: dir

            if path_has_extension(path1, "app") || path_has_extension(path2, "app") {
                // macOS app(s)
                synchronize_dirs_replace(path1, path2, &on_err)
            } else {
                // regular dir(s)
                synchronize_dirs(path1, path2, &on_err)
            }
        } else {
            // path1: dir, path2: file
            synchronize_file_with_dir(path2, path1, &on_err)
        }
    } else if path2.is_file() {
        // path1 & path2: file
        synchronize_files(path1, path2, &on_err)
    } else {
        // path1: file, path2: dir
        synchronize_file_with_dir(path1, path2, &on_err)
    }
}

/*
const DIR1_NOT_SYMLINK_ID: u8 = 0;
const DIR2_NOT_SYMLINK_ID: u8 = 1;
const DIR1_SYMLINK_ID: u8 = 2;
const DIR2_SYMLINK_ID: u8 = 3;
*/

fn id_and_relative_path_from_dir_entry<FErr>(
    entry: &walkdir::Result<DirEntry>,
    base_path: &Path,
    dir_id_no_symlink: u8,
    on_err: &FErr,
) -> Result<(u8, PathBuf), ErrorHandlingType>
where
    FErr: Fn(&dyn std::error::Error) -> ErrorHandlingType,
{
    match entry {
        Err(err) => Err(on_err(err)),
        Ok(entry) => {
            let dir_id = if entry.path_is_symlink() { 2 } else { 0 } + dir_id_no_symlink;
            let path: &Path = entry.path();

            macro_rules! some_or_return {
                ($e:expr) => {
                    match $e {
                        Some(x) => x,
                        None => return Err(ErrorHandlingType::Ignore),
                    }
                };
            }

            let path_str = some_or_return!(path.to_str());
            let base_path_str = some_or_return!(base_path.to_str());
            let trimmed = some_or_return!(trim_base_path(base_path_str, path_str));

            Ok((dir_id, trimmed))
        }
    }
}

fn synchronize_dirs<FErr>(dir1: &Path, dir2: &Path, on_err: &FErr) -> Result<(), ()>
where
    FErr: Fn(&dyn std::error::Error) -> ErrorHandlingType,
{
    let skip = RefCell::from(false);
    let fail = RefCell::from(false);

    macro_rules! id_and_relative_path {
        ($e:expr, $dir:expr, $id:expr, $on_err:expr) => {
            match id_and_relative_path_from_dir_entry($e, $dir, $id, $on_err) {
                Ok(x) => Some(x),
                Err(handle) => {
                    use ErrorHandlingType::*;

                    match handle {
                        Fail => *fail.borrow_mut() = true,
                        Skip => *skip.borrow_mut() = true,
                        Ignore => (),
                    };

                    None
                }
            }
        };
    }

    macro_rules! handle_error {
        ($err:expr) => {
            use ErrorHandlingType::*;

            match on_err($err) {
                Fail => return Err(()),
                Skip => return Ok(()),
                Ignore => (),
            };
        };
    }

    let dir_iterator = WalkDir::new(dir1)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| id_and_relative_path!(&e, dir1, 0, on_err))
        .chain(
            WalkDir::new(dir2)
                .min_depth(1)
                .into_iter()
                .filter_map(|e| id_and_relative_path!(&e, dir2, 1, on_err))
                // never synchronize the same path twice
                .filter(|(_, rel_path)| !dir1.join(rel_path).exists()),
        );

    if *fail.borrow() {
        return Err(());
    } else if *skip.borrow() {
        return Ok(());
    }

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

        // Paths that are part of a macOS app are already handled if they exists in both dirs => skip.
        if is_part_of_mac_app(&path_in_dir) && path_in_other_dir.exists() {
            continue;
        }

        if path_in_dir.is_file() {
            if path_in_other_dir.is_file() {
                // `path_in_other_dir` exists and points to a file
                // Check timestamps, and overwrite the older with the recent one.

                synchronize_files(&path_in_dir, &path_in_other_dir, on_err)?;
            } else if path_in_other_dir.is_dir() {
                synchronize_file_with_dir(&path_in_dir, &path_in_other_dir, on_err)?;
            } else {
                // path does not exist in other dir

                if let Err(err) = std::fs::copy(path_in_dir, path_in_other_dir) {
                    handle_error!(&err);
                }
            }
        } else if !path_in_other_dir.exists() {
            // path_in_dir: dir, path_in_other_dir: nothing

            if let Err(err) = std::fs::create_dir(path_in_other_dir) {
                handle_error!(&err);
            }
        } else if path_in_other_dir.is_file() {
            // path_in_dir: dir, path_in_other_dir: file

            synchronize_file_with_dir(&path_in_other_dir, &path_in_dir, on_err)?;
        } else if is_mac_app(&path_in_dir) {
            // path_in_dir: dir (macOS app), path_in_other_dir: dir (macOS app)

            synchronize_dirs_replace(&path_in_dir, &path_in_other_dir, on_err)?;
        } // else path_in_dir: dir, path_in_other_dir: dir => ignore.
    }

    Ok(())
}

fn synchronize_files<FErr>(path1: &Path, path2: &Path, on_err: &FErr) -> Result<(), ()>
where
    FErr: Fn(&dyn std::error::Error) -> ErrorHandlingType,
{
    macro_rules! handle_error {
        (use $on_err:ident for $err:ident) => {
            use ErrorHandlingType::*;

            let handle = $on_err(&$err);
            match handle {
                Fail => return Err(()),
                Skip | Ignore => (),
            }
        };
    }

    let time_in_dir = FileTime::from_last_modification_time(
        &std::fs::metadata(path1).expect("This should never happen"),
    );

    let time_in_other_dir = FileTime::from_last_modification_time(
        &std::fs::metadata(path2).expect("This should never happen"),
    );

    let (source_path, target_path, max_time) = if time_in_dir > time_in_other_dir {
        (path1, path2, time_in_dir)
    } else if time_in_dir < time_in_other_dir {
        (path2, path1, time_in_other_dir)
    } else {
        return Ok(()); // already synchronized => skip
    };

    if let Some(parent_path) = target_path.parent() {
        if !parent_path.exists() {
            // should be created before => should never happen
            if let Err(err) = std::fs::create_dir_all(parent_path) {
                handle_error!(use on_err for err);
            }
        }
    }

    if let Err(err) = std::fs::copy(source_path, target_path) {
        handle_error!(use on_err for err);
    }

    if let Err(err) = filetime::set_file_times(source_path, max_time, max_time) {
        handle_error!(use on_err for err);
    }

    Ok(())
}

fn synchronize_file_with_dir<FErr>(
    file_path: &Path,
    dir_path: &Path,
    on_err: &FErr,
) -> Result<(), ()>
where
    FErr: Fn(&dyn std::error::Error) -> ErrorHandlingType,
{
    macro_rules! unwrap_result {
        ($e:expr) => {
            match $e {
                Err(err) => {
                    use ErrorHandlingType::*;

                    match on_err(&err) {
                        Fail => return Err(()),
                        Skip | Ignore => return Ok(()),
                    }
                }
                Ok(x) => x,
            }
        };
    }

    let file_time = FileTime::from_last_modification_time(&unwrap_result!(file_path.metadata()));

    let dir_time = match dir_latest_modification_time(dir_path, on_err) {
        Ok(x) => x,
        Err(err) => {
            use ErrorHandlingType::*;

            return match err {
                Fail => Err(()),
                Skip => Ok(()),
                _ => unreachable!(),
            };
        }
    };

    if file_time > dir_time {
        unwrap_result!(fs::remove_dir_all(dir_path));
        unwrap_result!(fs::copy(file_path, dir_path));
        unwrap_result!(filetime::set_file_times(dir_path, file_time, file_time));
    } else {
        unwrap_result!(fs::remove_file(file_path));
        unwrap_result!(fs::create_dir(file_path));
        match copy_dir(dir_path, file_path, dir_time, on_err) {
            Ok(_) => (),
            Err(_) => return Err(()),
        }
    }

    Ok(())
}

fn copy_dir<'t1, 't2, FErr>(
    source: &'t1 Path,
    target: &'t2 Path,
    time: FileTime,
    on_err: &FErr,
) -> Result<(), ()>
where
    FErr: Fn(&dyn std::error::Error) -> ErrorHandlingType,
{
    let skip = RefCell::from(false);
    let fail = RefCell::from(false);

    let relative_path_iter = WalkDir::new(source)
        .min_depth(1)
        .into_iter()
        // Get path
        .filter_map(|e: walkdir::Result<DirEntry>| match e {
            Ok(x) => Some(x.path().to_owned()),
            Err(err) => {
                use ErrorHandlingType::*;

                match on_err(&err) {
                    Fail => *fail.borrow_mut() = true,
                    Skip => *skip.borrow_mut() = true,
                    Ignore => (),
                }

                None
            }
        })
        // Get path string representation
        .filter_map(|absolute_path: PathBuf| match absolute_path.to_str() {
            None => None,
            Some(absolute_path_str) => Some(absolute_path_str.to_owned()),
        })
        // Get relative path (returns a PathBuf)
        .filter_map(|absolute_path_str: String| match source.to_str() {
            None => None,
            Some(source_str) => trim_base_path(source_str, &absolute_path_str),
        });

    if *fail.borrow() {
        return Err(());
    } else if *skip.borrow() {
        return Ok(());
    }

    macro_rules! handle_on_error {
        ($e:expr) => {
            match $e {
                Ok(_) => (),
                Err(err) => {
                    use ErrorHandlingType::*;

                    match on_err(&err) {
                        Fail => return Err(()),
                        Skip | Ignore => return Ok(()),
                    }
                }
            }
        };
    }

    for relative_path in relative_path_iter {
        let relative_path: &Path = &relative_path;
        let path_in_dir = source.join(relative_path);
        let path_in_file = target.join(relative_path);

        if path_in_dir.is_dir() {
            handle_on_error!(fs::create_dir(&path_in_file));
        } else {
            handle_on_error!(fs::copy(&path_in_dir, &path_in_file));
        }

        handle_on_error!(filetime::set_file_times(&path_in_file, time, time));
    }

    handle_on_error!(filetime::set_file_times(target, time, time));

    Ok(())
}

/// Synchronize 2 directories, only keeps the one with the latest modification time.
fn synchronize_dirs_replace<'t1, 't2, FErr>(
    dir1_path: &'t1 Path,
    dir2_path: &'t2 Path,
    on_err: &FErr,
) -> Result<(), ()>
where
    FErr: Fn(&dyn std::error::Error) -> ErrorHandlingType,
{
    /// Unwrap or print error and return.
    macro_rules! unwrap_result {
        ($e:expr) => {
            match $e {
                Ok(x) => x,
                Err(err) => {
                    use ErrorHandlingType::*;

                    match on_err(&err) {
                        Fail => return Err(()),
                        Skip | Ignore => return Ok(()),
                    }
                }
            }
        };
    }

    let dir1_time = FileTime::from_last_modification_time(&unwrap_result!(dir1_path.metadata()));

    let dir2_time = FileTime::from_last_modification_time(&unwrap_result!(dir2_path.metadata()));

    if dir1_time > dir2_time {
        unwrap_result!(fs::remove_dir_all(dir2_path));
        copy_dir(dir1_path, dir2_path, dir1_time, on_err)?;
    } else if dir1_time != dir2_time {
        unwrap_result!(fs::remove_dir_all(dir1_path));
        copy_dir(dir2_path, dir1_path, dir2_time, on_err)?;
    }

    Ok(())
}

fn dir_latest_modification_time<'t, FErr>(
    path: &'t Path,
    on_err: &FErr,
) -> Result<FileTime, ErrorHandlingType>
where
    FErr: Fn(&dyn std::error::Error) -> ErrorHandlingType,
{
    let mut skip = false;
    let mut fail = false;

    macro_rules! unwrap_result {
        ($e:expr) => {
            match $e {
                Err(err) => {
                    use ErrorHandlingType::*;

                    match on_err(&err) {
                        Fail => fail = true,
                        Skip => skip = true,
                        Ignore => (),
                    }

                    return None;
                }
                Ok(x) => x,
            }
        };
    }

    let max = WalkDir::new(path)
        .into_iter()
        .filter_map(|e: walkdir::Result<DirEntry>| {
            let e = unwrap_result!(e);

            let path: &Path = e.path();

            Some(FileTime::from_last_modification_time(&unwrap_result!(
                path.metadata()
            )))
        })
        .max()
        .unwrap_or_else(FileTime::zero);

    if fail {
        Err(ErrorHandlingType::Fail)
    } else if skip {
        Err(ErrorHandlingType::Skip)
    } else {
        Ok(max)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn trim_base_path_unix() {
        let base = "/some/path";
        let entry = "/some/path/to/entry";
        let trimmed = super::trim_base_path(base, entry);

        assert_eq!(trimmed, Some(std::path::PathBuf::from("to/entry")))
    }

    #[test]
    fn trim_base_path_windows() {
        let base = "C:\\some\\path";
        let entry = "C:\\some\\path\\to\\entry";
        let trimmed = super::trim_base_path(base, entry);

        assert_eq!(trimmed, Some(std::path::PathBuf::from("to\\entry")))
    }

    #[test]
    fn path_has_extension_true() {
        let path = &std::path::Path::new("hello/rust.rs");
        let extension = "rs";
        let has_extension = super::path_has_extension(path, extension);

        assert!(has_extension)
    }

    #[test]
    fn path_has_extension_false() {
        let path = &std::path::Path::new("hello/rust.rs");
        let not_extension = "md";
        let has_extension = super::path_has_extension(path, not_extension);

        assert!(!has_extension)
    }

    // Not available in other OSes yet.
    #[cfg(target_os = "macos")]
    #[test]
    fn is_part_of_mac_app_true() {
        let path_inside_app = std::path::Path::new("/Applications/App Store.app/randomStuff");
        let is_part_of_mac_app = super::is_part_of_mac_app(path_inside_app);

        assert!(is_part_of_mac_app)
    }

    #[test]
    fn is_part_of_mac_app_false() {
        let path_outside_app = std::path::Path::new("hello/myAppOrNotReally/randomThingy");
        let is_part_of_mac_app = super::is_part_of_mac_app(path_outside_app);

        assert!(!is_part_of_mac_app)
    }

}

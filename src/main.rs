use atty::Stream;
use itertools::Itertools;
use sha2::{Digest, Sha256};
use std::env;
use std::fmt::Display;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use walkdir::WalkDir;

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();

    print_args(&args);
    if args.is_empty() {
        pause_with_default_message();
        return;
    }

    //Convert args to PathBuf
    let args: Vec<_> = args
        .iter()
        .map(|arg| PathBuf::from_str(arg).unwrap()) //Unwrap: infallible
        .collect();

    eprintln!("\nBuilding list of valid files...");
    let (paths_to_hash, expansion_errors) = expand_dirs(&args);
    print_dir_expansion_errors(&expansion_errors);
    if paths_to_hash.is_empty() {
        println!("\nNo valid files found (did you select folders/drives with no files?)");
        pause_with_default_message();
        return;
    }

    eprintln!("\nComputing hashes...");
    let (paths_and_hashes, hashing_errors) = pair_paths_with_hashes(&paths_to_hash);
    print_hashing_errors(&hashing_errors);
    if paths_and_hashes.is_empty() {
        println!("\nNo valid files left after skipping error-producing ones");
        pause_with_default_message();
        return;
    }

    print_individual_hashes(&paths_and_hashes);

    if paths_and_hashes.len() > 1 {
        print_hash_comparison_result(paths_and_hashes.iter().map(|(_, hash)| hash).all_equal());
        println!("Combined\n{}\n", compute_combined_hash(&paths_and_hashes));
    }

    println!("SHA-256");
    pause_with_default_message();
}

fn print_args<T: AsRef<str> + Display>(args: &[T]) {
    if !args.is_empty() {
        println!("Arguments:");
        for arg in args {
            println!("{}", arg);
        }
    } else {
        println!("No arguments passed");
    }
}

fn pause_with_default_message() {
    eprint!("\nPress enter to exit...");
    pause();
}

fn pause() {
    // Unwrap: no recourse, is end of program anyway, panic can give some info
    io::stdin().read_line(&mut String::new()).unwrap();
}

//All directories in the input are replaced with their files
fn expand_dirs<T: AsRef<Path>>(paths: &[T]) -> (Vec<PathBuf>, Vec<walkdir::Error>) {
    let mut expanded_paths = Vec::new();
    let mut errors = Vec::new();

    for path in paths.iter() {
        let (successes, failures): (Vec<_>, Vec<_>) = WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .partition_result();

        expanded_paths.extend(
            successes
                .into_iter()
                .filter(|entry| !entry.file_type().is_dir())
                .map(|entry| entry.into_path()),
        );

        errors.extend(failures);
    }

    (expanded_paths, errors)
}

fn print_dir_expansion_errors(errors: &[walkdir::Error]) {
    // for err in errors {
    //     match err.path() {
    //         Some(path) => match err.io_error() {
    //             Some(io_err) => {
    //                 eprintln!("yes path: IO error for operation on {}: {}",
    // path.display(), io_err)             }
    //             None => eprintln!("walkdir err: {}", err),
    //         },
    //         None => {
    //             eprintln!("no path: {}", err)
    //         }
    //     }
    // }

    for err in errors {
        if let (Some(path), Some(io_err)) = (err.path(), err.io_error()) {
            eprintln!("IO error for operation on {}: {}", path.display(), io_err);
        } else {
            eprintln!("{}", err)
        }
    }
}

fn pair_paths_with_hashes<T>(paths: &[T]) -> (Vec<(&Path, String)>, Vec<(&Path, io::Error)>)
where
    T: AsRef<Path>,
{
    let mut errors = Vec::new();

    let (mut open_successes, open_errors): (Vec<_>, Vec<_>) = paths
        .iter()
        .map(|path| {
            File::open(&path)
                .map(|file| (path.as_ref(), file))
                .map_err(|err| (path.as_ref(), err))
        })
        .partition_result();

    errors.extend(open_errors);

    // Only read 100 MiB at a time, enabling us to hash arbitrarily large files
    // without using much memory.
    const BUF_LEN: usize = 104_857_600;
    let mut buf = Vec::with_capacity(BUF_LEN);
    let (hash_successes, hash_errors): (Vec<_>, Vec<_>) = open_successes
        .iter_mut()
        .map(|path_file| {
            hash_in_chunks(&mut path_file.1, &mut buf, BUF_LEN)
                .map(|hash| (path_file.0, hash))
                .map_err(|err| (path_file.0, err))
        })
        .partition_result();

    errors.extend(hash_errors);

    (hash_successes, errors)
}

fn hash_in_chunks<R>(reader: &mut R, buf: &mut Vec<u8>, chunk_len: usize) -> io::Result<String>
where
    R: Read,
{
    let mut hasher = Sha256::new();
    while reader.take(chunk_len as u64).read_to_end(buf)? > 0 {
        hasher.update(&buf);
        buf.clear();
    }
    Ok(format!("{:X}", hasher.finalize()))
}

fn print_hashing_errors<T: AsRef<Path>>(errors: &[(T, io::Error)]) {
    for (path, err) in errors {
        eprintln!(
            "IO error for operation on {}: {}",
            path.as_ref().display(),
            err
        )
    }
}

fn print_individual_hashes<T: AsRef<Path>>(paths_and_hashes: &[(T, String)]) {
    println!();
    for (path, hash) in paths_and_hashes {
        println!("{}\n{}\n", path.as_ref().display(), hash);
    }
}

fn print_hash_comparison_result(are_hashes_equal: bool) {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    let mut print_with_color = |s, clr| {
        if atty::is(Stream::Stdout) {
            // Ignore result: no recourse, non-critical
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(clr)));

            // Ignore result: no recourse, non-critical
            let _ = writeln!(&mut stdout, "{}", s);
        } else {
            println!("{}", s);
        }
    };

    if are_hashes_equal {
        print_with_color("SAME\n", Color::Green);
    } else {
        print_with_color("DIFFERENT\n", Color::Red);
    }

    if atty::is(Stream::Stdout) {
        // Ignore result: no recourse, non-critical
        let _ = stdout.reset();
    }
}

fn compute_combined_hash<T: AsRef<Path>>(paths_and_hashes: &[(T, String)]) -> String {
    let mut hasher = Sha256::new();

    // Sort so it doesn't matter what order the paths were sent in
    paths_and_hashes
        .iter()
        .sorted_unstable_by_key(|(path, _)| path.as_ref())
        .for_each(|(_, hash)| hasher.update(hash));

    format!("{:X}", hasher.finalize())
}

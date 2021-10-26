#![feature(bool_to_option)]
use itertools::Itertools;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::str::FromStr;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use walkdir::WalkDir;

fn main() {
    // Only read 100 MiB at a time, enabling us to hash arbitrarily large files
    // without using much memory.
    const BUF_LEN: usize = 104_857_600;
    let mut buf = Vec::with_capacity(BUF_LEN);
    let paths_and_hashes: Vec<_> = {
        let valid_paths: Vec<_> = env::args()
            .skip(1)
            .map(|arg| PathBuf::from_str(&arg).unwrap()) // Infallible
            .filter(|path| {
                fs::metadata(path)
                    .map_err(|err| println!("{}", err))
                    .is_ok()
            })
            .collect();

        // Clippy doesn't realize we need to do `valid_paths.into_iter`, but can't if
        // it's borrowed.
        #[allow(clippy::needless_collect)]
        let paths_from_dirs: Vec<_> = valid_paths
            .iter()
            .filter(|path| path.is_dir())
            .flat_map(|path| {
                WalkDir::new(path)
                    .into_iter()
                    .filter_map(|entry| entry.map_err(|err| println!("{}", err)).ok())
                    .filter_map(|entry| entry.path().is_file().then_some(entry.into_path()))
            })
            .collect();

        paths_from_dirs
            .into_iter()
            .chain(valid_paths.into_iter().filter(|path| path.is_file()))
            .filter_map(|path| {
                File::open(&path)
                    .map(|file| (path, file))
                    .map_err(|err| println!("{}", err))
                    .ok()
            })
            .filter_map(|(path, mut file)| {
                hash_in_chunks(&mut file, &mut buf, BUF_LEN)
                    .map(|hash| (path, hash))
                    .map_err(|err| println!("{}", err))
                    .ok()
            })
            .collect()
    };

    if paths_and_hashes.is_empty() {
        return;
    }

    println!();
    for (path, hash) in &paths_and_hashes {
        println!("{}\n{}\n", path.display(), hash);
    }

    if paths_and_hashes.len() > 1 {
        let mut stdout = StandardStream::stdout(ColorChoice::Always);

        let mut print_with_color = |s, clr| {
            // Ignore result: no recourse, non-critical
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(clr)));

            // Ignore result: no recourse, non-critical
            let _ = writeln!(&mut stdout, "{}", s);
        };

        if paths_and_hashes.iter().map(|(_, hash)| hash).all_equal() {
            print_with_color("SAME\n", Color::Green);
        } else {
            print_with_color("DIFFERENT\n", Color::Red);
        }

        // Ignore result: no recourse, non-critical
        let _ = stdout.reset();

        let mut hasher = Sha256::new();

        // Sort so it doesn't matter what order the paths were sent in
        paths_and_hashes
            .iter()
            .sorted_unstable_by_key(|(path, _)| path)
            .for_each(|(_, hash)| hasher.update(hash));

        println!("Combined\n{}\n", format!("{:X}", hasher.finalize()));
    }

    println!("SHA-256\n");
    print!("Press enter to exit.");

    // Ignore result: no recourse, non-critical
    let _ = io::stdout().flush();

    // Unwrap: no recourse, is end of program anyway, panic can give some info
    io::stdin().read_line(&mut String::new()).unwrap();
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

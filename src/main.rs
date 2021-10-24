use itertools::Itertools;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use walkdir::WalkDir;

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();

    if args.is_empty() {
        return;
    }

    let mut paths = Vec::new();
    let mut hashes = Vec::new();

    //Only read 100 MiB at a time, enabling us to hash arbitrarily large files without
    //using much memory.
    const BUF_LEN: usize = 104_857_600;
    let mut buf = Vec::with_capacity(BUF_LEN);
    for path in args.iter().map(|arg| Path::new(arg)).filter(|p| p.exists()) {
        let mut push_hash_and_path = |path| {
            hashes.push(hash_in_chunks(
                &mut File::open(&path).unwrap(),
                &mut buf,
                BUF_LEN,
            ));
            paths.push(path);
        };

        if path.is_dir() {
            WalkDir::new(path)
                .into_iter()
                .map(|e| e.unwrap())
                .filter(|e| e.path().is_file())
                .for_each(|e| push_hash_and_path(e.path().to_owned()));
        } else if path.is_file() {
            push_hash_and_path(path.to_owned());
        }
    }

    println!("SHA-256\n");

    if paths.len() > 1 {
        //When creating the combined hash, sort lexicographically by path, that way it doesn't
        //matter what order the files happened to get sent in; the same set of files will always
        //produce the same combined hash.

        let mut combined_hasher = Sha256::new();

        paths
            .iter()
            .zip(hashes.iter())
            .sorted_unstable_by_key(|(path, _)| *path)
            .for_each(|(_, hash)| combined_hasher.update(hash));

        println!("Combined");
        println!("{}\n", format!("{:X}", combined_hasher.finalize()));

        //Eye-catching green/red for match/no match.
        let mut stdout = StandardStream::stdout(ColorChoice::Always);

        let mut print_with_color = |s, clr| {
            stdout
                .set_color(ColorSpec::new().set_fg(Some(clr)))
                .unwrap();
            writeln!(&mut stdout, "{}", s).unwrap();
        };

        if hashes.iter().all_equal() {
            print_with_color("SAME\n", Color::Green);
        } else {
            print_with_color("DIFFERENT\n", Color::Red);
        }

        stdout.reset().unwrap();
    }

    for (path, hash) in paths.iter().zip(hashes.iter()) {
        println!("{}\n{}\n", path.display(), hash);
    }

    print!("Press enter to exit.");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut String::new()).unwrap();
}

fn hash_in_chunks<R: Read>(reader: &mut R, mut buf: &mut Vec<u8>, chunk_len: usize) -> String {
    let mut hasher = Sha256::new();
    while reader.take(chunk_len as u64).read_to_end(&mut buf).unwrap() > 0 {
        hasher.update(&buf);
        buf.clear();
    }
    format!("{:X}", hasher.finalize())
}

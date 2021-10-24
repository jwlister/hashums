use itertools::Itertools;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use walkdir::WalkDir;

fn main() {
    //Only read 100 MiB at a time, enabling us to hash arbitrarily large files without
    //using much memory.
    const BUF_LEN: usize = 104_857_600;
    let mut buf = Vec::with_capacity(BUF_LEN);

    let path_iter = || {
        env::args()
            .skip(1)
            .map(|arg| Path::new(&arg).to_owned())
            .filter(|p| p.exists())
    };

    let paths_and_hashes: Vec<_> = path_iter()
        .filter(|p| p.is_dir())
        .flat_map(|p| {
            WalkDir::new(p)
                .into_iter()
                .map(|e| e.unwrap())
                .filter(|e| e.path().is_file())
                .map(|e| e.path().to_owned())
        })
        .chain(path_iter().filter(|p| p.is_file()))
        .map(|p| {
            let mut file = File::open(&p).unwrap();
            (p, hash_in_chunks(&mut file, &mut buf, BUF_LEN))
        })
        .collect();

    if paths_and_hashes.is_empty() {
        return;
    }

    println!("SHA-256\n");

    if paths_and_hashes.len() > 1 {
        //When creating the combined hash, sort lexicographically by path, that way it doesn't
        //matter what order the files happened to get sent in; the same set of files will always
        //produce the same combined hash.

        let mut hasher = Sha256::new();

        paths_and_hashes
            .iter()
            .sorted_unstable_by_key(|(path, _)| path)
            .for_each(|(_, hash)| hasher.update(hash));

        println!("Combined");
        println!("{}\n", format!("{:X}", hasher.finalize()));

        //Eye-catching green/red for match/no match.
        let mut stdout = StandardStream::stdout(ColorChoice::Always);

        let mut print_with_color = |s, clr| {
            stdout
                .set_color(ColorSpec::new().set_fg(Some(clr)))
                .unwrap();
            writeln!(&mut stdout, "{}", s).unwrap();
        };

        if paths_and_hashes.iter().map(|(_, hash)| hash).all_equal() {
            print_with_color("SAME\n", Color::Green);
        } else {
            print_with_color("DIFFERENT\n", Color::Red);
        }

        stdout.reset().unwrap();
    }

    for (path, hash) in &paths_and_hashes {
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

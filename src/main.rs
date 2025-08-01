use std::{path::{Path, PathBuf}, process::exit};

use clap::{arg, value_parser, Command};
use glob::glob;
use ygopac::{pack, unpack};

fn main() {
    let matches = Command::new("ygopac")
        .name("ygopac")
        .version("0.1")
        .about("Unpack and repack .pac files from the NDS Yugioh games")
        .subcommand_required(true)
        .arg(arg!(-v --verbose "Show additional info"))
        .subcommands([
            Command::new("unpack")
                .arg(arg!(<input> ".pac file to unpack")
                    .value_parser(value_parser!(PathBuf)))
                .arg(arg!(<output> "Directory to extract the files into")
                    .value_parser(value_parser!(PathBuf))),
            Command::new("unpack-dir")
                .arg(arg!(<input> "Directory of .pac files to unpack")
                    .value_parser(value_parser!(PathBuf)))
                .arg(arg!(<output> "Directory to extract the files into")
                    .value_parser(value_parser!(PathBuf))),
            Command::new("pack")
                .arg(arg!(<input> ".pacman file to load")
                    .value_parser(value_parser!(PathBuf)))
                .arg(arg!(<output> "Directory to write the .pac file into")
                    .value_parser(value_parser!(PathBuf))),
            Command::new("pack-dir")
                .arg(arg!(<input> "Directory of .pacman files to load")
                    .value_parser(value_parser!(PathBuf)))
                .arg(arg!(<output> "Directory to write the .pac files into")
                    .value_parser(value_parser!(PathBuf))),
        ])
        .get_matches();

    match matches.subcommand() {
        Some(("unpack", sub_matches)) => {
            let input = sub_matches.get_one::<PathBuf>("input").unwrap();
            let output = sub_matches.get_one::<PathBuf>("output").unwrap();

            if !input.is_file() {
                eprintln!("Error: '{}' is not a file", input.display());
                exit(1);
            }

            if input.extension().and_then(|s| s.to_str()) != Some("pac") {
                eprintln!("Error: '{}' is not a .pac file", input.display());
                exit(1);
            }

            if let Err(err) = unpack(input, output, matches.get_flag("verbose")) {
                eprintln!("{err}");
                exit(1);
            }
        }
        Some(("unpack-dir", sub_matches)) => {
            let input = sub_matches.get_one::<PathBuf>("input").unwrap();
            let output = sub_matches.get_one::<PathBuf>("output").unwrap();

            if !input.is_dir() {
                eprintln!("Error: '{}' is not a directory", input.display());
                exit(1);
            }

            let verbose = matches.get_flag("verbose");
            for entry in glob(input.join("**/*.pac").to_str().unwrap()).unwrap() {
                match entry {
                    Ok(path) => {
                        if verbose {
                            println!("Unpacking {}", path.display());
                        }
                        let relative_path = path.strip_prefix(input).unwrap();
                        let outdir = output.join(format!("{}.d", relative_path.display()));

                        if let Err(err) = unpack(&path , &outdir, verbose) {
                            eprintln!("{err}");
                            exit(1);
                        }
                        if verbose {
                            println!();
                        }
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        exit(1);
                    }
                }
            }
        }
        Some(("pack", sub_matches)) => {
            let input = sub_matches.get_one::<PathBuf>("input").unwrap();
            let output = sub_matches.get_one::<PathBuf>("output").unwrap();

            if !input.is_file() {
                eprintln!("Error: '{}' is not a file", input.display());
                exit(1);
            }

            if input.extension().and_then(|s| s.to_str()) != Some("pacman") {
                eprintln!("Error: '{}' is not a .pacman file", input.display());
                exit(1);
            }

            if let Err(err) = pack(input, output, matches.get_flag("verbose")) {
                eprintln!("{err}");
                exit(1);
            }
        }
        Some(("pack-dir", sub_matches)) => {
            let input = sub_matches.get_one::<PathBuf>("input").unwrap();
            let output = sub_matches.get_one::<PathBuf>("output").unwrap();

            if !input.is_dir() {
                eprintln!("Error: '{}' is not a directory", input.display());
                exit(1);
            }

            let verbose = matches.get_flag("verbose");
            for entry in glob(input.join("**/*.pac.d").to_str().unwrap()).unwrap() {
                match entry {
                    Ok(path) => {
                        if !path.is_dir() {
                            eprintln!("Error: '{}' is not a directory", path.display());
                        }

                        let dir_name = path.file_name().unwrap().to_str().unwrap();
                        let pacman_name = format!("{}man", &dir_name[..dir_name.len()-2]);
                        let infile = path.join(pacman_name);

                        if !infile.is_file() {
                            eprintln!("Error: '{}' is not a file", infile.display());
                            exit(1);
                        }

                        if verbose {
                            println!("Packing {}", input.display());
                        }

                        let relative_path = path.strip_prefix(input).unwrap();
                        let parent_dir = relative_path.parent().unwrap_or(Path::new(""));
                        let outdir = output.join(parent_dir);

                        if let Err(err) = pack(&infile, &outdir, verbose) {
                            eprintln!("{err}");
                            exit(1);
                        }

                        if verbose {
                            println!();
                        }
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        exit(1);
                    }
                }
            }
        }
        _ => unreachable!(),
    }
}

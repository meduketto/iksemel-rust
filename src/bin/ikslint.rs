/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::env;
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_version() {
    println!("ikslint (iksemel) v{}", VERSION);
}

fn print_usage() {
    println!(concat!(
        "Usage: ikslint [OPTIONS] FILE.xml...\n",
        "This tool checks the well-formedness of XML documents.\n",
        "Options:\n",
        "  -s, --stat       Display statistics\n",
        "  -t, --histogram  Display tag histogram\n",
        "  -h, --help       Display this help message and exit\n",
        "  -v, --version    Display the version and exit\n",
        "Report issues at https://github.com/meduketto/iksemel-rust/issues"
    ));
}

struct Linter {
    do_state: bool,
    do_histogram: bool,
    nr_tags: u32,
}

impl Linter {
    fn new(do_state: bool, do_histogram: bool) -> Self {
        Linter {
            do_state,
            do_histogram,
            nr_tags: 0,
        }
    }

    fn lint_file(&mut self, file: &str) -> bool {
        return true;
    }
}

fn main() -> ExitCode {
    let mut args = env::args();

    let mut files = Vec::new();
    let mut do_state = false;
    let mut do_histogram = false;

    // Skip the first argument (program name)
    args.next();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-s" | "--stat" => {
                do_state = true;
            }
            "-t" | "--histogram" => {
                do_histogram = true;
            }
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            "-v" | "--version" => {
                print_version();
                return ExitCode::SUCCESS;
            }
            _ => {
                files.push(arg);
            }
        }
    }

    let mut linter = Linter::new(do_state, do_histogram);
    for file in files {
        if !linter.lint_file(&file) {
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

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
use std::fs::File;
use std::io::Read;
use std::process::ExitCode;

use iksemel::{Document, DocumentError};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_version() {
    println!("ikspath (iksemel) v{}", VERSION);
}

fn print_usage() {
    println!(concat!(
        "Usage: ikspath [OPTIONS] [FILE.xml] [XPATH expression]\n",
        "This tool applies XPATH expression to an XML document.\n",
        "Options:\n",
        "  -h, --help           Display this help message and exit\n",
        "  -v, --version        Display the version and exit\n",
        "Report issues at https://github.com/meduketto/iksemel-rust/issues"
    ));
}

fn load_file(file: &str) -> Result<Document, DocumentError> {
    let mut parser = iksemel::DocumentParser::new();
    let mut f = File::open(file).unwrap();
    let mut buffer = vec![0u8; 256 * 1024];
    loop {
        let bytes_read = f.read(&mut buffer).unwrap();
        if bytes_read == 0 {
            break;
        }
        parser.parse_bytes(&buffer[..bytes_read])?;
    }
    Ok(parser.into_document()?)
}

fn main() -> ExitCode {
    let mut args = env::args();

    let mut file: Option<String> = None;

    // Skip the first argument (program name)
    args.next();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            "-v" | "--version" => {
                print_version();
                return ExitCode::SUCCESS;
            }
            _ => match file {
                Some(_) => {}
                None => {
                    file = Some(arg.to_string());
                }
            },
        }
    }

    let doc = load_file(&file.unwrap()).unwrap();
    println!("{:?}", doc.arena_stats());

    ExitCode::SUCCESS
}

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
        "Usage: ikspath [OPTIONS] [XPATH expression]\n",
        "This tool applies XPATH expression to an XML document.\n",
        "Options:\n",
        "  -f, --file <FILE.xml>  Specify the XML file to process\n",
        "  -h, --help             Display this help message and exit\n",
        "  -v, --version          Display the version and exit\n",
        "Report issues at https://github.com/meduketto/iksemel-rust/issues"
    ));
}

enum IkspathError {
    IoError(std::io::Error),
    NoMemory,
    DocumentError(DocumentError),
}

impl From<std::io::Error> for IkspathError {
    fn from(err: std::io::Error) -> Self {
        IkspathError::IoError(err)
    }
}

impl From<DocumentError> for IkspathError {
    fn from(err: DocumentError) -> Self {
        IkspathError::DocumentError(err)
    }
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

fn process_file(file: Option<String>, expression: Option<String>) -> Result<(), IkspathError> {
    let doc = load_file(&file.unwrap()).unwrap();
    println!("{:?}", doc.arena_stats());
    Ok(())
}

fn main() -> ExitCode {
    let mut args = env::args();

    let mut file: Option<String> = None;
    let mut expression: Option<String> = None;

    // Skip the first argument (program name)
    args.next();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-f" | "--file" => {
                if let Some(value) = args.next() {
                    file = Some(value);
                } else {
                    eprintln!("Error: file name expected after -f/--file");
                    return ExitCode::FAILURE;
                }
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
                if expression.is_none() {
                    expression = Some(arg.to_string());
                } else {
                    eprintln!("Error: only one expression can be specified");
                    return ExitCode::FAILURE;
                }
            }
        }
    }

    match process_file(file, expression) {
        Ok(_) => {}
        Err(IkspathError::IoError(err)) => {
            eprintln!("IO Error: {}", err);
            return ExitCode::FAILURE;
        }
        Err(IkspathError::NoMemory) => {
            eprintln!("Error: not enough memory");
            return ExitCode::FAILURE;
        }
        Err(IkspathError::DocumentError(DocumentError::NoMemory)) => {
            eprintln!("Error: not enough memory");
            return ExitCode::FAILURE;
        }
        Err(IkspathError::DocumentError(DocumentError::BadXml(err))) => {
            eprintln!("Error: Syntax error: {}", err);
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

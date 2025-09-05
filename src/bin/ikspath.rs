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
use std::io::stdin;
use std::process::ExitCode;

use iksemel::{Document, DocumentError, DocumentParser, XPath};

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
    BadXml(&'static str),
}

impl From<std::io::Error> for IkspathError {
    fn from(err: std::io::Error) -> Self {
        IkspathError::IoError(err)
    }
}

impl From<DocumentError> for IkspathError {
    fn from(err: DocumentError) -> Self {
        match err {
            DocumentError::NoMemory => IkspathError::NoMemory,
            DocumentError::BadXml(msg) => IkspathError::BadXml(msg),
        }
    }
}

fn load_xml_file(
    parser: &mut DocumentParser,
    file: Option<String>,
) -> Result<Document, IkspathError> {
    let mut f: Box<dyn Read> = match file {
        None => Box::new(stdin()),
        Some(file_name) => Box::new(File::open(file_name)?),
    };
    let mut buffer = vec![0u8; 256 * 1024];
    loop {
        let bytes_read = f.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        parser.parse_bytes(&buffer[..bytes_read])?;
    }
    Ok(parser.take_document()?)
}

fn main() -> ExitCode {
    let mut args = env::args();

    let mut file: Option<String> = None;
    let mut expression: Option<XPath> = None;

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
                    match XPath::new(&arg) {
                        Ok(xpath) => expression = Some(xpath),
                        Err(err) => {
                            eprintln!("Error: invalid XPath expression: {}", err);
                            return ExitCode::FAILURE;
                        }
                    }
                } else {
                    eprintln!("Error: only one expression can be specified");
                    return ExitCode::FAILURE;
                }
            }
        }
    }

    let mut parser = DocumentParser::new();
    let file_desc = match file.as_deref() {
        None => "input stream".to_string(),
        Some(file_name) => format!("file '{}'", file_name),
    };
    let document = match load_xml_file(&mut parser, file) {
        Ok(doc) => doc,
        Err(IkspathError::IoError(err)) => {
            eprintln!("Error: io error in {}: {}", file_desc, err);
            return ExitCode::FAILURE;
        }
        Err(IkspathError::NoMemory) => {
            eprintln!("Error: not enough memory");
            return ExitCode::FAILURE;
        }
        Err(IkspathError::BadXml(err)) => {
            eprintln!(
                "Error: syntax error in {} at {}: {}",
                file_desc,
                parser.location(),
                err
            );
            return ExitCode::FAILURE;
        }
    };

    println!("{:?}", document.arena_stats());

    if let Some(xpath) = expression {
        let sequence = xpath.apply(&document).unwrap();
        println!("{}", sequence);
    }

    ExitCode::SUCCESS
}

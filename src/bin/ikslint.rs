/*
** This file is a part of Iksemel (XML parser for Jabber/XMPP)
** Copyright (C) 2000-2025 Gurer Ozen
**
** Iksemel is free software: you can redistribute it and/or modify it
** under the terms of the GNU Lesser General Public License as
** published by the Free Software Foundation, either version 3 of
** the License, or (at your option) any later version.
*/

use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::Read;
use std::io::stdin;
use std::process::ExitCode;
use std::vec::Vec;

use iksemel::SaxElement;
use iksemel::SaxError;
use iksemel::SaxHandler;
use iksemel::SaxHandlerError;
use iksemel::SaxParser;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;

fn print_version() {
    println!("ikslint (iksemel) v{}", VERSION);
}

fn print_usage() {
    println!(
        concat!(
            "Usage: ikslint [OPTIONS] [FILE.xml...]\n",
            "This tool checks the well-formedness of XML documents.\n",
            "Options:\n",
            "  -s, --stat           Overall statistics\n",
            "  -c, --count          Tag counts\n",
            "  -b, --buffer <SIZE>  File read buffer size in bytes (default: {})\n",
            "  -h, --help           Display this help message and exit\n",
            "  -v, --version        Display the version and exit\n",
            "Report issues at https://github.com/meduketto/iksemel-rust/issues"
        ),
        DEFAULT_BUFFER_SIZE
    );
}

struct Handler {
    do_stats: bool,
    do_tag_count: bool,
    level: usize,
    max_depth: usize,
    nr_tags: usize,
    nr_empty_tags: usize,
    nr_cdata_size: usize,
    tag_stack: Vec<String>,
    tag_map: HashMap<String, usize>,
    attribute_map: HashSet<String>,
    error: Option<String>,
}

impl Handler {
    fn new(do_stats: bool, do_tag_count: bool) -> Self {
        Handler {
            do_stats,
            do_tag_count,
            level: 0,
            max_depth: 0,
            nr_tags: 0,
            nr_empty_tags: 0,
            nr_cdata_size: 0,
            tag_stack: Vec::new(),
            tag_map: HashMap::new(),
            attribute_map: HashSet::new(),
            error: None,
        }
    }

    fn report(&mut self) {
        if self.do_stats {
            println!(
                "Tags pairs: {}, empty element tags: {}, max depth: {}",
                self.nr_tags, self.nr_empty_tags, self.max_depth
            );
            print!(
                "Total size of character data: {} bytes.\n",
                self.nr_cdata_size
            );
        }
        if self.do_tag_count {
            println!("Tag counts:");
            for (tag, count) in self.tag_map.iter() {
                println!("  {}: {}", tag, count);
            }
        }
        self.level = 0;
        self.max_depth = 0;
        self.nr_tags = 0;
        self.nr_empty_tags = 0;
        self.nr_cdata_size = 0;
        self.tag_stack.clear();
        self.tag_map.clear();
        self.attribute_map.clear();
    }
}

impl SaxHandler for Handler {
    fn handle_element(&mut self, element: &SaxElement) -> Result<(), SaxHandlerError> {
        match element {
            SaxElement::StartTag(name) => {
                self.nr_tags += 1;
                self.level += 1;
                self.max_depth = self.max_depth.max(self.level);
                if self.do_tag_count {
                    *self.tag_map.entry(name.to_string()).or_insert(0) += 1;
                }
                self.tag_stack.push(name.to_string());
                self.attribute_map.clear();
            }
            SaxElement::Attribute(name, _value) => {
                if self.attribute_map.contains(*name) {
                    self.error = Some(format!("duplicate attribute: '{}'", name));
                    return Err(SaxHandlerError::Abort);
                }
                self.attribute_map.insert(name.to_string());
            }
            SaxElement::EmptyElementTag => {
                self.nr_empty_tags += 1;
                self.tag_stack.pop();
            }
            SaxElement::CData(cdata) => {
                self.nr_cdata_size += cdata.len();
            }
            SaxElement::EndTag(name) => {
                self.level -= 1;
                let start_name = self.tag_stack.pop().unwrap();
                if &start_name != name {
                    self.error = Some(format!(
                        "end tag mismatch: expected '{}', got '{}'",
                        start_name, name
                    ));
                    return Err(SaxHandlerError::Abort);
                }
            }
        }
        Ok(())
    }
}

enum LinterError {
    IoError(std::io::Error),
    SaxError(SaxError),
}

impl From<std::io::Error> for LinterError {
    fn from(err: std::io::Error) -> Self {
        LinterError::IoError(err)
    }
}

impl From<SaxError> for LinterError {
    fn from(err: SaxError) -> Self {
        LinterError::SaxError(err)
    }
}

struct Linter {
    handler: Handler,
    parser: SaxParser,
    buffer_size: usize,
}

impl Linter {
    fn new(do_stats: bool, do_tag_count: bool, buffer_size: usize) -> Self {
        Linter {
            handler: Handler::new(do_stats, do_tag_count),
            parser: SaxParser::new(),
            buffer_size,
        }
    }

    fn parse_file(&mut self, file: &str, is_stream: bool) -> Result<(), LinterError> {
        let mut f: Box<dyn Read> = if is_stream {
            Box::new(stdin())
        } else {
            Box::new(File::open(file)?)
        };
        let mut buffer = vec![0u8; self.buffer_size];
        loop {
            let bytes_read = f.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            self.parser
                .parse_bytes(&mut self.handler, &buffer[..bytes_read])?;
        }
        Ok(self.parser.parse_finish()?)
    }

    fn lint_file(&mut self, file: &str, is_stream: bool) -> bool {
        self.parser.reset();
        match self.parse_file(file, is_stream) {
            Ok(()) => {
                self.handler.report();
                true
            }
            Err(LinterError::IoError(e)) => {
                eprintln!("Error reading file '{}': {}", file, e);
                false
            }
            Err(LinterError::SaxError(SaxError::NoMemory)) => {
                eprintln!("Memory allocation failed while parsing '{}'", file);
                false
            }
            Err(LinterError::SaxError(SaxError::BadXml(msg))) => {
                eprintln!(
                    "Syntax error in file '{}' at line {} column {}: {}",
                    file,
                    self.parser.nr_lines(),
                    self.parser.nr_column(),
                    msg
                );
                false
            }
            Err(LinterError::SaxError(SaxError::HandlerAbort)) => {
                eprintln!(
                    "Well-formedness error in file '{}' at line {} column {}: {}",
                    file,
                    self.parser.nr_lines(),
                    self.parser.nr_column(),
                    self.handler.error.as_ref().unwrap()
                );
                false
            }
        }
    }
}

fn main() -> ExitCode {
    let mut args = env::args();

    let mut files = Vec::new();
    let mut do_stats = false;
    let mut do_tag_count = false;
    let mut buffer_size = DEFAULT_BUFFER_SIZE;

    // Skip the first argument (program name)
    args.next();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-s" | "--stat" => {
                do_stats = true;
            }
            "-c" | "--count" => {
                do_tag_count = true;
            }
            "-cs" | "-sc" => {
                do_stats = true;
                do_tag_count = true;
            }
            "-b" | "--buffer" => {
                if let Some(size) = args.next() {
                    if let Ok(size) = size.parse::<usize>() {
                        buffer_size = size;
                    } else {
                        eprintln!("Invalid buffer size");
                        return ExitCode::FAILURE;
                    }
                } else {
                    eprintln!("Missing buffer size");
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
                files.push(arg);
            }
        }
    }

    let mut linter = Linter::new(do_stats, do_tag_count, buffer_size);
    if files.is_empty() {
        if !linter.lint_file("stdin", true) {
            return ExitCode::FAILURE;
        }
    } else {
        for file in files {
            if !linter.lint_file(&file, false) {
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}

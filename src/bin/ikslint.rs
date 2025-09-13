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

use iks::ParseError;
use iks::SaxElement;
use iks::SaxElements;
use iks::SaxParser;

const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;

fn print_version() {
    println!("ikslint (iksemel) v{}", iks::VERSION);
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

    fn process_element(&mut self, element: &SaxElement) -> Result<(), ParseError> {
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
                    return Err(ParseError::BadXml("duplicate attribute"));
                }
                self.attribute_map.insert(name.to_string());
            }
            SaxElement::StartTagContent => {}
            SaxElement::StartTagEmpty => {
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
                    return Err(ParseError::BadXml("end tag mismatch"));
                }
            }
        }
        Ok(())
    }

    fn report(&mut self) {
        if self.do_stats {
            println!(
                "Tags pairs: {}, empty element tags: {}, max depth: {}",
                self.nr_tags, self.nr_empty_tags, self.max_depth
            );
            println!(
                "Total size of character data: {} bytes.",
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

enum LinterError {
    IoError(std::io::Error),
    ParseError(ParseError),
}

impl From<std::io::Error> for LinterError {
    fn from(err: std::io::Error) -> Self {
        LinterError::IoError(err)
    }
}

impl From<ParseError> for LinterError {
    fn from(err: ParseError) -> Self {
        LinterError::ParseError(err)
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
            let mut elements = SaxElements::new(&mut self.parser, &buffer[..bytes_read]);
            loop {
                match elements.next() {
                    Some(Ok(element)) => {
                        self.handler.process_element(&element)?;
                    }
                    Some(Err(err)) => return Err(err.into()),
                    None => {
                        break;
                    }
                }
            }
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
            Err(LinterError::ParseError(ParseError::NoMemory)) => {
                eprintln!("Memory allocation failed while parsing '{}'", file);
                false
            }
            Err(LinterError::ParseError(ParseError::BadXml(msg))) => {
                eprintln!(
                    "Syntax error in file '{}' at {}: {}",
                    file,
                    self.parser.location(),
                    msg
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

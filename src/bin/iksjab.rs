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

use iks::Jid;

fn print_version() {
    println!("iksjab (iksemel) v{}", iks::VERSION);
}

fn print_usage() {
    println!(concat!(
        "Usage: iksjab [OPTIONS]\n",
        "This tool can communicate over XMPP.\n",
        "Options:\n",
        "  -j, --jid <JID>        Jabber ID\n",
        "  -h, --help             Display this help message and exit\n",
        "  -v, --version          Display the version and exit\n",
        "Report issues at https://github.com/meduketto/iksemel-rust/issues"
    ));
}

fn main() -> ExitCode {
    let mut args = env::args();
    let mut _jid: Option<Jid> = None;

    // Skip the first argument (program name)
    args.next();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-j" | "--jid" => {
                if let Some(value) = args.next() {
                    _jid = match Jid::new(&value) {
                        Ok(jid) => Some(jid),
                        Err(err) => {
                            eprintln!("Error: {}", err);
                            return ExitCode::FAILURE;
                        }
                    };
                } else {
                    eprintln!("Error: Jabber ID expected after {arg}");
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
            _ => {}
        }
    }

    ExitCode::SUCCESS
}

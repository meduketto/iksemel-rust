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
use iks::XMPP_CLIENT_PORT;
use iks::XmppClient;

fn print_version() {
    println!("iksjab (iksemel) v{}", iks::VERSION);
}

fn print_usage() {
    println!(concat!(
        "Usage: iksjab [OPTIONS]\n",
        "This tool can communicate over XMPP.\n",
        "Options:\n",
        "  -j, --jid <JID>        Jabber ID\n",
        "  -s, --server <SERVER>  XMPP server override\n",
        "  -d, --debug            Print XMPP traffic\n",
        "  -h, --help             Display this help message and exit\n",
        "  -v, --version          Display the version and exit\n",
        "Report issues at https://github.com/meduketto/iksemel-rust/issues"
    ));
}

fn run(jid: Jid, server: Option<String>, debug: bool) {
    println!("lala");
    let mut client = XmppClient::build(jid)
        .server(server)
        .debug(debug)
        .connect()
        .unwrap();
    loop {
        let _stanza = client.wait_for_stanza();
    }
}

fn main() -> ExitCode {
    let mut args = env::args();
    let mut jid: Option<Jid> = None;
    let mut server: Option<String> = None;
    let mut debug = false;

    // Skip the first argument (program name)
    args.next();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-j" | "--jid" => {
                if let Some(value) = args.next() {
                    jid = match Jid::new(&value) {
                        Ok(jid) => {
                            if jid.is_bare() {
                                Some(jid.with_resource("iksjab").unwrap())
                            } else {
                                Some(jid)
                            }
                        }
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
            "-s" | "--server" => {
                if let Some(value) = args.next() {
                    if value.contains(':') {
                        server = Some(value);
                    } else {
                        server = Some(format!("{}:{}", value, XMPP_CLIENT_PORT));
                    }
                } else {
                    eprintln!("Error: Server address expected after {arg}");
                    return ExitCode::FAILURE;
                }
            }
            "-d" | "--debug" => {
                debug = true;
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

    if let Some(jid) = jid {
        run(jid, server, debug);
    } else {
        eprintln!("Error: Jabber ID not provided");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

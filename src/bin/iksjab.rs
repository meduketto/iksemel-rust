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
use std::io::Write;
use std::process::ExitCode;

use rpassword::prompt_password;

use iks::Jid;
use iks::XMPP_CLIENT_PORT;
use iks::XmppClient;
use iks::XmppClientError;

fn print_version() {
    println!("iksjab (iksemel) v{}", iks::VERSION);
}

fn print_usage() {
    println!(concat!(
        "Usage: iksjab [OPTIONS]\n",
        "This tool can communicate over XMPP.\n",
        "Options:\n",
        "  -j, --jid <JID>             Jabber ID\n",
        "  -s, --server <SERVER>       XMPP server override\n",
        "  -p, --password <ENVVARNAME> Environment variable with the password\n",
        "  -b, --backup <FILENAME>     Backup roster to the given file\n",
        "  -m, --message <JID> <BODY>  Send a message\n",
        "  -w, --watch                 Listen and print stanzas forever\n",
        "  -d, --debug                 Print XMPP traffic\n",
        "  -h, --help                  Display this help message and exit\n",
        "  -v, --version               Display the version and exit\n",
        "Report issues at https://github.com/meduketto/iksemel-rust/issues"
    ));
}

struct LoginOptions {
    jid: Jid,
    server: Option<String>,
    password: String,
    debug: bool,
}

struct MessageOptions {
    jid: Jid,
    body: String,
}

fn login(options: LoginOptions) -> Result<XmppClient, XmppClientError> {
    let mut client = XmppClient::build(options.jid, options.password)
        .server(options.server)
        .debug(options.debug)
        .connect()?;
    let _iq_bind_stanza = client.wait_for_stanza()?;
    Ok(client)
}

fn run(
    options: LoginOptions,
    backup_file: Option<String>,
    messages: Vec<MessageOptions>,
    watch_mode: bool,
) -> Result<(), XmppClientError> {
    let mut client = login(options)?;

    if let Some(file) = backup_file {
        client.request_roster()?;
        loop {
            let stanza = client.wait_for_stanza()?;
            if stanza.root().attribute("id") == Some("roster") {
                let mut f = File::create(file)?;
                f.write_all(stanza.to_string().as_bytes())?;
                break;
            }
        }
    }

    for message in messages {
        client.send_message(message.jid, &message.body)?
    }

    if watch_mode {
        loop {
            let stanza = client.wait_for_stanza()?;
            println!("Stanza:{}", stanza);
        }
    }

    Ok(())
}

fn get_password(var_name: Option<String>) -> Result<String, String> {
    if let Some(name) = &var_name {
        return match env::var(name) {
            Ok(value) => Ok(value),
            Err(err) => Err(format!(
                "Failed to get password from environment variable {}: {}",
                name, err
            )),
        };
    }
    if let Ok(password) = prompt_password("Jabber password: ") {
        Ok(password)
    } else {
        Err("Password not provided".to_string())
    }
}

fn main() -> ExitCode {
    let mut args = env::args();
    let mut jid: Option<Jid> = None;
    let mut server: Option<String> = None;
    let mut password_var: Option<String> = None;
    let mut messages: Vec<MessageOptions> = Vec::new();
    let mut backup_file: Option<String> = None;
    let mut debug = false;
    let mut watch_mode = false;

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
            "-p" | "--password" => {
                if let Some(value) = args.next() {
                    password_var = Some(value);
                } else {
                    eprintln!("Error: Password environment variable expected after {arg}");
                    return ExitCode::FAILURE;
                }
            }
            "-b" | "--backup" => {
                if let Some(value) = args.next() {
                    backup_file = Some(value);
                } else {
                    eprintln!("Error: Backup file expected after {arg}");
                    return ExitCode::FAILURE;
                }
            }
            "-m" | "--message" => {
                if let Some(jid_str) = args.next() {
                    let jid = match Jid::new(&jid_str) {
                        Ok(jid) => jid,
                        Err(err) => {
                            eprintln!("Error: Invalid JID: {}", err);
                            return ExitCode::FAILURE;
                        }
                    };
                    if let Some(body) = args.next() {
                        messages.push(MessageOptions { jid, body });
                    } else {
                        eprintln!("Error: Message body expected after {arg} <JID>");
                        return ExitCode::FAILURE;
                    }
                } else {
                    eprintln!("Error: Jid expected after {arg}");
                    return ExitCode::FAILURE;
                }
            }
            "-w" | "--watch" => {
                watch_mode = true;
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

    let options: LoginOptions;
    if let Some(jid) = jid {
        match get_password(password_var) {
            Ok(password) => {
                options = LoginOptions {
                    jid,
                    server,
                    password,
                    debug,
                };
            }
            Err(err) => {
                eprintln!("Error: {}", err);
                return ExitCode::FAILURE;
            }
        }
    } else {
        eprintln!("Error: Jabber ID not provided");
        return ExitCode::FAILURE;
    }

    if let Err(err) = run(options, backup_file, messages, watch_mode) {
        eprintln!("Error: {}", err);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

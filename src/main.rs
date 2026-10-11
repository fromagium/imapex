use std::fs::{self, write};

use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long)]
    username: String,
    #[arg(short, long)]
    password: String,
    #[arg(short, long)]
    server: String,
    #[arg(short, long)]
    debug: bool,
    #[arg(long)]
    destination: String,
    #[arg(short, long)]
    limit: Option<u64>,
}

const DEFAULT_IMAPS_PORT: u16 = 993;

fn main() -> Result<(), String> {
    let args = Args::parse();
    if args.debug {
        println!("opening session on {} for {}", args.server, args.username);
    }
    let tls = native_tls::TlsConnector::builder().build().unwrap();
    match imap::connect(
        (args.server.clone(), DEFAULT_IMAPS_PORT),
        args.server.clone(),
        &tls,
    ) {
        Ok(client) => {
            if args.debug {
                println!("connected to {}", args.server);
            }
            match client.login(&args.username, &args.password) {
                Ok(mut session) => {
                    if args.debug {
                        println!("session opened");
                    }
                    // Select the inbox
                    session
                        .select("INBOX")
                        .map_err(|error| format!("failed to select INBOX: {error}"))?;

                    fs::create_dir_all(&args.destination)
                        .map_err(|error| format!("failed to create destination dir: {error}"))?;

                    // Fetch all messages in the inbox and save them locally until the inbox
                    // is empty.
                    let mut position = 1;
                    loop {
                        let messages = session.fetch(position.to_string(), "RFC822");
                        if let Some(m) = messages.iter().next() {
                            match m.first() {
                                Some(message) => {
                                    // Found a new message
                                    if args.debug {
                                        println!("found message at position: {position}");
                                    }

                                    // Save the message in the destination directory
                                    match message.body() {
                                        Some(body) => match str::from_utf8(body) {
                                            Ok(body) => {
                                                // Save message
                                                if let Err(error) = write(
                                                    format!("{}/{position}.eml", args.destination),
                                                    body,
                                                ) {
                                                    eprintln!("{error}");
                                                } else {
                                                    // Mark message as deleted
                                                    if let Err(error) = session.store(
                                                        position.to_string(),
                                                        "+FLAGS (\\Deleted)",
                                                    ) {
                                                        eprintln!(
                                                            "failed to mark message at posistin {position} as deleted: {error}"
                                                        );
                                                    }
                                                }
                                            }
                                            Err(error) => eprintln!(
                                                "couldn't parse body for message at position {position}: {error}"
                                            ),
                                        },
                                        None => eprintln!(
                                            "no body found for message at position: {position}"
                                        ),
                                    }

                                    if let Some(limit) = args.limit
                                        && position == limit
                                    {
                                        break;
                                    }

                                    position += 1;
                                }
                                None => {
                                    if args.debug {
                                        println!("no message in fetched content");
                                    }
                                }
                            }
                        } else {
                            if args.debug {
                                println!("no more messages");
                            }
                            break;
                        };
                    }

                    session
                        .expunge()
                        .map_err(|error| format!("failed to expunge mailbox: {error}"))?;
                }
                Err(error) => eprintln!("failed to open session: {error:?}"),
            }
        }
        Err(error) => eprintln!("failed to open connection: {error}"),
    }

    Ok(())
}

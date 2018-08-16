extern crate file_protocol;
#[macro_use]
extern crate log;
extern crate simplelog;

use simplelog::*;

use file_protocol::CborProtocol;
use file_protocol::FileProtocol;
use std::env;
use std::path::Path;

// Receive callback function
// Called after a message is received over UDP and then validated by the CBOR `recv_start` function?
fn on_message(message: &str) {
    // TODO: Get a pointer to `f_protocol` and call its `on_message` function
    unimplemented!();
}

fn upload(source_path: &str, target_path: &str) -> Result<(), String> {
    // Get a local UDP socket (Bind)
    let c_protocol = CborProtocol::new(0, on_message);

    // Set up the full connection info
    let f_protocol = FileProtocol {
        cbor_proto: c_protocol,
        // Remote IP?
        host: String::from("127.0.0.1"),
        dest_port: 7000,
    };

    info!(
        "Uploading local:{} to remote:{}",
        &source_path, &target_path
    );
    // Copy file to upload to temp storage. Calculate the hash and chunk info
    // Q: What's `mode` for? `local_import` always returns 0. Looks like it should be file permissions
    let (hash, num_chunks, mode) = f_protocol.local_import(&source_path)?;
    // Tell our destination the hash and number of chunks to expect
    f_protocol.send_sync(&hash, num_chunks)?;
    // Send the actual file
    f_protocol.send_export(&hash, &target_path, mode)?;

    Ok(())
}

fn download(source_path: &str, target_path: &str) -> Result<(), String> {
    // Get a local UDP socket (Bind)
    let c_protocol = CborProtocol::new(0, on_message);

    // Set up the full connection info
    let f_protocol = FileProtocol {
        cbor_proto: c_protocol,
        // Remote IP?
        host: String::from("127.0.0.1"),
        dest_port: 7000,
    };

    info!(
        "Downloading remote: {} to remote: {}",
        source_path, target_path
    );

    // Send our file request to the remote addr and get the returned data
    let (hash, num_chunks, mode) = f_protocol.send_import(source_path)?;

    // Calculate our received hash and number of chunks and send back to the remote addr to verify
    // successful transfer
    f_protocol.sync_and_send(&hash, Some(num_chunks))?;

    // If good, save received data to the requested path
    f_protocol.local_export(&hash, target_path, Some(mode))?;

    Ok(())
}

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, Config::default()).unwrap(),
    ]).unwrap();

    info!("Starting file transfer client");

    let mut args = env::args();
    // Skip the first command arg
    args.next();

    // get upload vs download (required)
    let command = match args.next() {
        Some(ref cmd) if cmd == "upload" || cmd == "download" => cmd.clone(),
        _ => {
            error!("Missing first arg: 'upload' or 'download' must be specified");
            return;
        }
    };

    // get source file (required)
    let source_path = match args.next() {
        Some(path) => path,
        None => {
            error!("Missing source file path");
            return;
        }
    };

    // get target file. If not present, just copy the filename from the source path
    let target_path = match args.next() {
        Some(path) => path,
        None => Path::new(&source_path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
    };

    let result = match command.as_ref() {
        "upload" => upload(&source_path, &target_path),
        "download" => download(&source_path, &target_path),
        // This shouldn't be possible, since we checked the string earlier
        _ => {
            error!("Unknown command given");
            return;
        }
    };

    if let Err(err) = result {
        error!("Operation failed: {}", err);
    } else {
        info!("Operation successful");
    }
}

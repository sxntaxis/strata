#![cfg(unix)]

use std::{
    io::{BufRead, BufReader, BufWriter, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

use crate::{command::CommandIntent, profile};

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    id: u128,
    command: CommandIntent,
}

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    id: u128,
    ok: bool,
    message: String,
}

pub(crate) struct CommandServer {
    listener: UnixListener,
    socket_path: PathBuf,
}

impl CommandServer {
    pub(crate) fn bind() -> Result<Self, String> {
        let socket_path = socket_path();
        if socket_path.exists() {
            match UnixStream::connect(&socket_path) {
                Ok(_) => {
                    return Err("another Strata runtime is already active for this profile".into());
                }
                Err(_) => std::fs::remove_file(&socket_path)
                    .map_err(|error| format!("cannot remove stale control socket: {error}"))?,
            }
        }
        let listener = UnixListener::bind(&socket_path)
            .map_err(|error| format!("cannot bind profile control socket: {error}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| format!("cannot configure profile control socket: {error}"))?;
        Ok(Self {
            listener,
            socket_path,
        })
    }

    pub(crate) fn process_pending<F>(&self, mut handler: F) -> Result<(), String>
    where
        F: FnMut(CommandIntent) -> Result<String, String>,
    {
        loop {
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_read_timeout(Some(Duration::from_millis(600)));
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(600)));
                    let response = match read_request(&stream) {
                        Ok(request) => response_for(request, &mut handler),
                        Err(error) => Response {
                            id: 0,
                            ok: false,
                            message: error,
                        },
                    };
                    write_response(&mut stream, &response)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(())
    }
}

impl Drop for CommandServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

pub(crate) fn send(command: &CommandIntent) -> Result<Option<String>, String> {
    let path = socket_path();
    let mut stream = match UnixStream::connect(path) {
        Ok(stream) => stream,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound
                    | std::io::ErrorKind::ConnectionRefused
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::AddrNotAvailable
            ) =>
        {
            return Ok(None);
        }
        Err(error) => return Err(error.to_string()),
    };
    stream
        .set_read_timeout(Some(Duration::from_millis(800)))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_millis(800)))
        .map_err(|error| error.to_string())?;
    let request = Request {
        id: request_id(),
        command: command.clone(),
    };
    let payload = serde_json::to_string(&request).map_err(|error| error.to_string())?;
    let mut writer = BufWriter::new(&mut stream);
    writer
        .write_all(payload.as_bytes())
        .map_err(|error| error.to_string())?;
    writer.write_all(b"\n").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())?;
    drop(writer);
    let response = read_response(&stream)?;
    if response.ok {
        Ok(Some(response.message))
    } else {
        Err(response.message)
    }
}

fn socket_path() -> PathBuf {
    profile::state_dir().join("runtime.sock")
}

fn request_id() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn response_for<F>(request: Request, handler: &mut F) -> Response
where
    F: FnMut(CommandIntent) -> Result<String, String>,
{
    let result = handler(request.command);
    match result {
        Ok(message) => Response {
            id: request.id,
            ok: true,
            message,
        },
        Err(message) => Response {
            id: request.id,
            ok: false,
            message,
        },
    }
}

fn read_request(stream: &UnixStream) -> Result<Request, String> {
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| error.to_string())?;
    serde_json::from_str(line.trim()).map_err(|error| error.to_string())
}

fn read_response(stream: &UnixStream) -> Result<Response, String> {
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| error.to_string())?;
    serde_json::from_str(line.trim()).map_err(|error| error.to_string())
}

fn write_response(stream: &mut UnixStream, response: &Response) -> Result<(), String> {
    let payload = serde_json::to_string(response).map_err(|error| error.to_string())?;
    let mut writer = BufWriter::new(stream);
    writer
        .write_all(payload.as_bytes())
        .map_err(|error| error.to_string())?;
    writer.write_all(b"\n").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

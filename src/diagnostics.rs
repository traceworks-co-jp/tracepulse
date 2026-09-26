use base64::Engine;
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::Duration;
#[cfg(not(windows))]
use std::time::Instant;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum BasicKind {
    Ping,
    Traceroute,
    Port,
}

#[derive(Debug, Deserialize)]
struct Request {
    kind: BasicKind,
    target: String,
    count: Option<u8>,
    port: Option<u16>,
}

pub struct Connection {
    stream: TcpStream,
}

impl Connection {
    pub fn accept(mut stream: TcpStream, key: &str) -> Result<Self, String> {
        let mut digest = Sha1::new();
        digest.update(key.as_bytes());
        digest.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
        let accept = base64::engine::general_purpose::STANDARD.encode(digest.finalize());
        let response = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
        );
        stream
            .write_all(response.as_bytes())
            .map_err(|error| error.to_string())?;
        stream.flush().map_err(|error| error.to_string())?;
        Ok(Self { stream })
    }

    fn read_text(&mut self) -> Result<Option<String>, String> {
        let mut header = [0_u8; 2];
        match self.stream.read_exact(&mut header) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(error) => return Err(error.to_string()),
        }
        let opcode = header[0] & 0x0f;
        let masked = header[1] & 0x80 != 0;
        let mut length = u64::from(header[1] & 0x7f);
        if length == 126 {
            let mut bytes = [0_u8; 2];
            self.stream
                .read_exact(&mut bytes)
                .map_err(|error| error.to_string())?;
            length = u64::from(u16::from_be_bytes(bytes));
        }
        if length == 127 {
            let mut bytes = [0_u8; 8];
            self.stream
                .read_exact(&mut bytes)
                .map_err(|error| error.to_string())?;
            length = u64::from_be_bytes(bytes);
        }
        if length > 1_048_576 {
            return Err("websocket frame is too large".to_string());
        }
        let mut mask = [0_u8; 4];
        if masked {
            self.stream
                .read_exact(&mut mask)
                .map_err(|error| error.to_string())?;
        }
        let mut payload = vec![0_u8; length as usize];
        self.stream
            .read_exact(&mut payload)
            .map_err(|error| error.to_string())?;
        if masked {
            for (index, byte) in payload.iter_mut().enumerate() {
                *byte ^= mask[index % 4];
            }
        }
        if opcode == 0x8 {
            return Ok(None);
        }
        String::from_utf8(payload)
            .map(Some)
            .map_err(|error| error.to_string())
    }

    fn send(&mut self, text: &str) -> Result<(), String> {
        let payload = text.as_bytes();
        let mut frame = vec![0x81];
        match payload.len() {
            0..=125 => frame.push(payload.len() as u8),
            126..=65_535 => {
                frame.push(126);
                frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
            }
            _ => {
                frame.push(127);
                frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
            }
        }
        frame.extend_from_slice(payload);
        self.stream
            .write_all(&frame)
            .map_err(|error| error.to_string())?;
        self.stream.flush().map_err(|error| error.to_string())
    }
}

pub fn handle(mut connection: Connection) -> Result<(), String> {
    let result = handle_request(&mut connection);
    if let Err(error) = &result {
        let _ = connection.send(&serde_json::json!({"error":error,"done":true}).to_string());
    }
    result
}

fn handle_request(connection: &mut Connection) -> Result<(), String> {
    let request: Request = serde_json::from_str(
        &connection
            .read_text()?
            .ok_or_else(|| "diagnostic request is missing".to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let target: IpAddr = request
        .target
        .parse()
        .map_err(|_| "target must be an IP address".to_string())?;
    let count = request.count.unwrap_or(5).clamp(1, 20);
    match request.kind {
        BasicKind::Port => {
            let port = request.port.ok_or_else(|| "port is required".to_string())?;
            let address = SocketAddr::new(target, port);
            let result = TcpStream::connect_timeout(&address, Duration::from_secs(3));
            let line = match result {
                Ok(_) => format!("PORT {port}: Success - OPEN"),
                Err(error) => format!("PORT {port}: Failed - {error}"),
            };
            connection.send(
                &serde_json::json!({"event":"diagnostic","line":line,"done":true}).to_string(),
            )
        }
        BasicKind::Ping | BasicKind::Traceroute => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            runtime.block_on(async {
                for sequence in 1..=count {
                    let line = if matches!(request.kind, BasicKind::Ping) { probe(target, sequence, None).await } else { probe(target, sequence, Some(sequence)).await };
                    connection.send(&serde_json::json!({"event":"diagnostic","line":line,"done":sequence == count}).to_string())?;
                    if sequence != count { tokio::time::sleep(Duration::from_secs(1)).await; }
                }
                Ok::<(), String>(())
            })
        }
    }
}

async fn probe(target: IpAddr, sequence: u8, ttl: Option<u8>) -> String {
    #[cfg(windows)]
    {
        return windows_probe(target, sequence, ttl);
    }
    #[cfg(not(windows))]
    {
        use surge_ping::{Client, Config, ICMP, IcmpPacket, PingIdentifier, PingSequence};
        let mut builder =
            Config::builder().kind(if target.is_ipv4() { ICMP::V4 } else { ICMP::V6 });
        if let Some(ttl) = ttl {
            builder = builder.ttl(u32::from(ttl));
        }
        let Ok(client) = Client::new(&builder.build()) else {
            return format!(
                "{} {sequence}: Failed - ICMP socket unavailable",
                if ttl.is_some() { "HOP" } else { "PING" }
            );
        };
        let mut pinger = client.pinger(target, PingIdentifier(0x5450)).await;
        let started = Instant::now();
        match pinger
            .ping(PingSequence(sequence as u16), &[sequence; 8])
            .await
        {
            Ok((IcmpPacket::V4(_), elapsed)) | Ok((IcmpPacket::V6(_), elapsed)) => format!(
                "{} {sequence}: Success - responder unavailable ({:.2}ms)",
                if ttl.is_some() { "HOP" } else { "PING" },
                elapsed.as_secs_f64() * 1000.0
            ),
            Err(error) => format!(
                "{} {sequence}: Failed - {error} ({:.2}ms)",
                if ttl.is_some() { "HOP" } else { "PING" },
                started.elapsed().as_secs_f64() * 1000.0
            ),
        }
    }
}

#[cfg(windows)]
fn windows_probe(target: IpAddr, sequence: u8, ttl: Option<u8>) -> String {
    use std::net::Ipv4Addr;
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        ICMP_ECHO_REPLY, IP_OPTION_INFORMATION, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho2,
    };
    let IpAddr::V4(address) = target else {
        return format!(
            "{} {sequence}: Failed - IPv6 responder lookup unavailable",
            if ttl.is_some() { "HOP" } else { "PING" }
        );
    };
    let handle = unsafe { IcmpCreateFile() };
    if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
        return format!(
            "{} {sequence}: Failed - ICMP socket unavailable",
            if ttl.is_some() { "HOP" } else { "PING" }
        );
    }
    let options = IP_OPTION_INFORMATION {
        Ttl: ttl.unwrap_or(64),
        Tos: 0,
        Flags: 0,
        OptionsSize: 0,
        OptionsData: std::ptr::null_mut(),
    };
    let payload = [sequence; 8];
    #[repr(C)]
    struct ReplyBuffer {
        echo: ICMP_ECHO_REPLY,
        data: [u8; 512],
    }
    let mut reply = std::mem::MaybeUninit::<ReplyBuffer>::zeroed();
    let result = unsafe {
        IcmpSendEcho2(
            handle,
            std::ptr::null_mut(),
            None,
            std::ptr::null_mut(),
            u32::from(Ipv4Addr::from(address)).to_be(),
            payload.as_ptr() as _,
            payload.len() as u16,
            &options,
            reply.as_mut_ptr() as _,
            std::mem::size_of::<ReplyBuffer>() as u32,
            1_000,
        )
    };
    unsafe {
        IcmpCloseHandle(handle);
    }
    if result == 0 {
        return format!(
            "{} {sequence}: Failed - timeout",
            if ttl.is_some() { "HOP" } else { "PING" }
        );
    }
    let echo = unsafe { &(*reply.as_ptr()).echo };
    let responder = Ipv4Addr::from(u32::from_be(echo.Address));
    format!(
        "{} {sequence}: Success - {responder} ({:.2}ms)",
        if ttl.is_some() { "HOP" } else { "PING" },
        echo.RoundTripTime
    )
}

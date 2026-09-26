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
    let result = (|| {
        let request = connection
            .read_text()?
            .ok_or_else(|| "diagnostic request is missing".to_string())?;
        run_basic(&request, |message| connection.send(message))
    })();
    if let Err(error) = &result {
        let _ = connection.send(&serde_json::json!({"error":error,"done":true}).to_string());
    }
    result
}

pub fn run_basic(
    request: &str,
    mut send: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    let request: Request = serde_json::from_str(request).map_err(|error| error.to_string())?;
    let target: IpAddr = request
        .target
        .parse()
        .map_err(|_| "target must be an IP address".to_string())?;
    let count = if matches!(request.kind, BasicKind::Traceroute) {
        30
    } else {
        request.count.unwrap_or(5).clamp(1, 20)
    };
    match request.kind {
        BasicKind::Port => {
            let port = request.port.ok_or_else(|| "port is required".to_string())?;
            let address = SocketAddr::new(target, port);
            let result = TcpStream::connect_timeout(&address, Duration::from_secs(3));
            let line = match &result {
                Ok(_) => format!("PORT {port}: Success - OPEN"),
                Err(error) => format!("PORT {port}: Failed - {error}"),
            };
            send(
                &serde_json::json!({"event":"diagnostic","line":line,"done":true,
                "message_type":"port_result","data":{
                    "target":address.to_string(),
                    "status":if result.is_ok() { "OPEN" } else { "UNREACHABLE" },
                    "message":line
                }})
                .to_string(),
            )
        }
        BasicKind::Ping | BasicKind::Traceroute => {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            runtime.block_on(async {
                let mut responses = 0_u8;
                let mut rtt_total_ms = 0.0_f64;
                let mut min_rtt_ms = f64::INFINITY;
                let mut max_rtt_ms = 0.0_f64;
                for sequence in 1..=count {
                    let (line, rtt_ms, reached_target) = if matches!(request.kind, BasicKind::Ping) { probe(target, sequence, None).await } else { probe(target, sequence, Some(sequence)).await };
                    if let Some(rtt_ms) = rtt_ms {
                        responses += 1;
                        rtt_total_ms += rtt_ms;
                        min_rtt_ms = min_rtt_ms.min(rtt_ms);
                        max_rtt_ms = max_rtt_ms.max(rtt_ms);
                    }
                    let done = sequence == count || matches!(request.kind, BasicKind::Traceroute) && reached_target;
                    let result = if matches!(request.kind, BasicKind::Ping) {
                        serde_json::json!({"message_type":"ping_result","data":{
                            "target":target.to_string(),"sent":count,"received":responses,
                            "lost":count - responses,"packet_loss_percent":format!("{:.0}%", 100.0 * f64::from(count - responses) / f64::from(count)),
                            "min_rtt_ms":if responses > 0 { Some(min_rtt_ms) } else { None },
                            "average_rtt_ms":if responses > 0 { Some(rtt_total_ms / f64::from(responses)) } else { None },
                            "max_rtt_ms":if responses > 0 { Some(max_rtt_ms) } else { None },
                            "verdict":if responses == count { "REACHABLE" } else if responses > 0 { "PARTIAL" } else { "NO RESPONSE" }
                        }})
                    } else {
                        serde_json::json!({"message_type":"traceroute_result","data":{
                            "target":target.to_string(),"hops_probed":sequence,"responding_hops":responses,
                            "verdict":if reached_target { "DESTINATION REACHED" } else if responses > 0 { "HOP LIMIT REACHED" } else { "NO RESPONSE" }
                        }})
                    };
                    let mut message = serde_json::json!({"event":"diagnostic","line":line,"done":done});
                    if done {
                        message["message_type"] = result["message_type"].clone();
                        message["data"] = result["data"].clone();
                    }
                    send(&message.to_string())?;
                    if done { break; }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Ok::<(), String>(())
            })
        }
    }
}

async fn probe(target: IpAddr, sequence: u8, ttl: Option<u8>) -> (String, Option<f64>, bool) {
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
            return (
                format!(
                    "{} {sequence}: Failed - ICMP socket unavailable",
                    if ttl.is_some() { "HOP" } else { "PING" }
                ),
                None,
                false,
            );
        };
        let mut pinger = client.pinger(target, PingIdentifier(0x5450)).await;
        let started = Instant::now();
        match pinger
            .ping(PingSequence(sequence as u16), &[sequence; 8])
            .await
        {
            Ok((IcmpPacket::V4(_), elapsed)) | Ok((IcmpPacket::V6(_), elapsed)) => (
                format!(
                    "{} {sequence}: Success - responder unavailable ({:.2}ms)",
                    if ttl.is_some() { "HOP" } else { "PING" },
                    elapsed.as_secs_f64() * 1000.0
                ),
                Some(elapsed.as_secs_f64() * 1000.0),
                true,
            ),
            Err(error) => (
                format!(
                    "{} {sequence}: Failed - {error} ({:.2}ms)",
                    if ttl.is_some() { "HOP" } else { "PING" },
                    started.elapsed().as_secs_f64() * 1000.0
                ),
                None,
                false,
            ),
        }
    }
}

#[cfg(windows)]
fn windows_probe(target: IpAddr, sequence: u8, ttl: Option<u8>) -> (String, Option<f64>, bool) {
    use std::net::Ipv4Addr;
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        ICMP_ECHO_REPLY, IP_OPTION_INFORMATION, IP_SUCCESS, IcmpCloseHandle, IcmpCreateFile,
        IcmpSendEcho2,
    };
    let IpAddr::V4(address) = target else {
        return (
            format!(
                "{} {sequence}: Failed - IPv6 responder lookup unavailable",
                if ttl.is_some() { "HOP" } else { "PING" }
            ),
            None,
            false,
        );
    };
    let handle = unsafe { IcmpCreateFile() };
    if handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
        return (
            format!(
                "{} {sequence}: Failed - ICMP socket unavailable",
                if ttl.is_some() { "HOP" } else { "PING" }
            ),
            None,
            false,
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
        return (
            format!(
                "{} {sequence}: Failed - timeout",
                if ttl.is_some() { "HOP" } else { "PING" }
            ),
            None,
            false,
        );
    }
    let echo = unsafe { &(*reply.as_ptr()).echo };
    let responder = Ipv4Addr::from(u32::from_be(echo.Address));
    let reached_target = echo.Status == IP_SUCCESS && IpAddr::V4(responder) == target;
    (
        format!(
            "{} {sequence}: Success - {responder} ({:.2}ms)",
            if ttl.is_some() { "HOP" } else { "PING" },
            echo.RoundTripTime
        ),
        Some(f64::from(echo.RoundTripTime)),
        reached_target,
    )
}

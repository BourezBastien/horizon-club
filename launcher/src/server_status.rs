//! A server's players, by the status ping every client sends to show a server in its
//! Multiplayer list (vanilla's `ServerStatusPinger`): a handshake asking for the status state, a
//! status request, and the server's JSON answer. Nothing about the asker is sent.
//!
//! The launcher's Multiplayer tab asks the servers on its list, as the game's own Multiplayer
//! screen does when it opens, to show who is on each: the club's own, and the hosted ones, which
//! [`crate::online`] admits only at an address of the local network. It goes to each by its
//! address as written, through no SRV record, as the managed game connects.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// The port an address names without one.
pub const DEFAULT_PORT: u16 = 25565;

/// How long to wait for one server to answer.
pub const TIMEOUT: Duration = Duration::from_secs(3);

/// The longest status answer read: room for a 64x64 icon in base64 and a long description.
const MAX_ANSWER: usize = 256 * 1024;

/// The players part of a status answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Players {
    pub online: u64,
    pub max: u64,
}

/// `host`, `host:port` or `[v6]:port` as host and port. SRV records are not looked up.
pub fn split_address(address: &str) -> Option<(String, u16)> {
    let address = address.trim();
    if address.is_empty() {
        return None;
    }
    if let Some(rest) = address.strip_prefix('[') {
        let (host, after) = rest.split_once(']')?;
        let port = match after.strip_prefix(':') {
            Some(p) => p.parse().ok()?,
            None if after.is_empty() => DEFAULT_PORT,
            None => return None,
        };
        return Some((host.to_owned(), port));
    }
    match address.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => Some((host.to_owned(), port.parse().ok()?)),
        _ => Some((address.to_owned(), DEFAULT_PORT)),
    }
}

/// Asks `address` for its status within `timeout` and returns its player count. Err: why it
/// didn't answer.
pub fn players(address: &str, timeout: Duration) -> Result<Players, String> {
    let (host, port) = split_address(address).ok_or("not a server address")?;
    let addr = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or("the name has no address")?;
    let mut stream = TcpStream::connect_timeout(&addr, timeout).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(timeout))
        .and_then(|()| stream.set_write_timeout(Some(timeout)))
        .map_err(|e| e.to_string())?;
    stream
        .write_all(&request(&host, port))
        .map_err(|e| e.to_string())?;
    let json = read_answer(&mut stream)?;
    parse_players(&json)
}

/// The handshake (next state: status) and the status request, each framed with its length. The
/// protocol version is the game's (`horizon_platform::version`); a server answers any.
pub fn request(host: &str, port: u16) -> Vec<u8> {
    let mut handshake = Vec::new();
    write_varint(&mut handshake, 0x00);
    write_varint(&mut handshake, horizon_platform::version::PROTOCOL_VERSION);
    write_varint(&mut handshake, host.len() as i32);
    handshake.extend_from_slice(host.as_bytes());
    handshake.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut handshake, 1);
    let mut out = Vec::new();
    write_varint(&mut out, handshake.len() as i32);
    out.extend(handshake);
    // The status request: length 1, packet 0x00.
    out.extend([1, 0x00]);
    out
}

fn write_varint(out: &mut Vec<u8>, value: i32) {
    let mut v = value as u32;
    loop {
        if v & !0x7f == 0 {
            out.push(v as u8);
            return;
        }
        out.push((v & 0x7f) as u8 | 0x80);
        v >>= 7;
    }
}

fn read_varint(r: &mut impl Read) -> Result<i32, String> {
    let mut value = 0u32;
    for i in 0..5 {
        let mut byte = [0];
        r.read_exact(&mut byte).map_err(|e| e.to_string())?;
        value |= u32::from(byte[0] & 0x7f) << (7 * i);
        if byte[0] & 0x80 == 0 {
            return Ok(value as i32);
        }
    }
    Err("a varint longer than 5 bytes".into())
}

/// The status response packet's JSON.
pub fn read_answer(r: &mut impl Read) -> Result<String, String> {
    let length = read_varint(r)?;
    if length <= 0 || length as usize > MAX_ANSWER {
        return Err(format!("a status answer of {length} bytes"));
    }
    let mut packet = vec![0; length as usize];
    r.read_exact(&mut packet).map_err(|e| e.to_string())?;
    let mut body = packet.as_slice();
    if read_varint(&mut body)? != 0x00 {
        return Err("not a status answer".into());
    }
    let text = read_varint(&mut body)?;
    if text < 0 || text as usize != body.len() {
        return Err("a status answer of the wrong length".into());
    }
    String::from_utf8(body.to_vec()).map_err(|e| e.to_string())
}

/// `players.online` and `players.max` from a status answer.
pub fn parse_players(json: &str) -> Result<Players, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let players = value.get("players").ok_or("no players in the answer")?;
    let field = |k: &str| players.get(k).and_then(serde_json::Value::as_u64);
    Ok(Players {
        online: field("online").ok_or("no players.online")?,
        max: field("max").unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn answer(json: &str) -> Vec<u8> {
        let mut packet = Vec::new();
        write_varint(&mut packet, 0x00);
        write_varint(&mut packet, json.len() as i32);
        packet.extend_from_slice(json.as_bytes());
        let mut out = Vec::new();
        write_varint(&mut out, packet.len() as i32);
        out.extend(packet);
        out
    }

    #[test]
    fn addresses_split_into_host_and_port() {
        let split = |a| split_address(a);
        assert_eq!(
            split("mc-club.example.edu"),
            Some(("mc-club.example.edu".into(), 25565))
        );
        assert_eq!(
            split("mc-club.example.edu:25570"),
            Some(("mc-club.example.edu".into(), 25570))
        );
        assert_eq!(split("[::1]:25566"), Some(("::1".into(), 25566)));
        assert_eq!(split("[::1]"), Some(("::1".into(), 25565)));
        assert_eq!(split("example.org:http"), None);
        assert_eq!(split(" "), None);
    }

    /// By hand from the protocol: a frame of 16 bytes, the handshake (id 0, protocol 777 as
    /// `0x89 0x06`, the host's length 9 and the host, port 25565 as `0x63 0xdd`, state 1), then
    /// the status request.
    #[test]
    fn the_request_is_the_status_handshake_then_the_status_request() {
        assert_eq!(
            request("127.0.0.1", 25565),
            [
                0x10, 0x00, 0x89, 0x06, 0x09, b'1', b'2', b'7', b'.', b'0', b'.', b'0', b'.', b'1',
                0x63, 0xdd, 0x01, 0x01, 0x00
            ]
        );
    }

    #[test]
    fn varints_are_vanillas() {
        for (value, bytes) in [
            (0, vec![0x00]),
            (127, vec![0x7f]),
            (128, vec![0x80, 0x01]),
            (25565, vec![0xdd, 0xc7, 0x01]),
            (-1, vec![0xff, 0xff, 0xff, 0xff, 0x0f]),
        ] {
            let mut out = Vec::new();
            write_varint(&mut out, value);
            assert_eq!(out, bytes, "{value}");
            assert_eq!(read_varint(&mut bytes.as_slice()).unwrap(), value);
        }
    }

    #[test]
    fn bad_answers_are_refused() {
        assert!(read_answer(&mut [0x00].as_slice()).is_err());
        assert!(read_answer(&mut &answer("{}")[..2]).is_err());
        let mut wrong_id = answer("{}");
        wrong_id[1] = 0x01;
        assert!(read_answer(&mut wrong_id.as_slice()).is_err());
        assert!(parse_players(r#"{"players":{"max":5}}"#).is_err());
        assert!(parse_players("not json").is_err());
        assert_eq!(
            read_answer(&mut answer(r#"{"players":{"online":3}}"#).as_slice()).unwrap(),
            r#"{"players":{"online":3}}"#
        );
        assert_eq!(
            parse_players(r#"{"players":{"online":3}}"#).unwrap(),
            Players { online: 3, max: 0 }
        );
    }

    /// A server on this computer answers its player count.
    #[test]
    #[ignore = "daily"]
    fn a_server_answers_its_player_count() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let expected = request("127.0.0.1", port);
            let mut got = vec![0; expected.len()];
            socket.read_exact(&mut got).unwrap();
            assert_eq!(got, expected);
            let json = r#"{"version":{"name":"26.3","protocol":777},"players":{"max":100,"online":12,"sample":[]},"description":{"text":"Horizon"}}"#;
            socket.write_all(&answer(json)).unwrap();
        });
        let players = players(&format!("127.0.0.1:{port}"), Duration::from_secs(2)).unwrap();
        assert_eq!(
            players,
            Players {
                online: 12,
                max: 100
            }
        );
        server.join().unwrap();
    }
}

//! A managed build: the `managed` feature, for a school or a club whose supervisor decides where
//! the game may connect (docs/school-edition). The launcher hands the game a rule file with
//! `--managed FILE`, one `key:value` a line, and the game keeps to it:
//!
//! ```text
//! # A `#` starts a comment.
//! lan:true                 # a world may be opened to the LAN (vanilla's Open to LAN)
//! server:mc.example.org    # an address outside the local network the players may join:
//!                          # `host` or `host:port`, one line each
//! ```
//!
//! Everywhere else the rule is the local network: the game connects to a server only when its
//! name resolves to a private address (10/8, 172.16/12, 192.168/16, loopback, link-local, and
//! IPv6's equivalents). The check reads the resolved addresses, and the connection goes to the
//! one it checked by its number: no SRV redirect, no second lookup the name could answer
//! differently. A managed build started without its rule plays single player only: nothing
//! dials. A build without the feature reads no rule and keeps vanilla's behaviour. A player's
//! game of a managed build also opens no agent-control socket, whatever it was started with; a
//! dedicated server keeps the one `horizon-server` drives it with.
use horizon_protocol::connection::Connection;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

/// Whether this build is managed.
pub fn on() -> bool {
    cfg!(feature = "managed")
}

/// The rule of the run: `None` until `--managed` loaded it.
static RULE: OnceLock<Option<Rule>> = OnceLock::new();

struct Rule {
    /// `lan:true`: a world may be opened to the LAN.
    lan: bool,
    /// `server:` lines, outside the local network, that the players may join.
    public: Vec<(String, u16)>,
}

/// Load the rule file the `--managed` option named. A second call changes nothing.
pub fn load(path: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    RULE.set(Some(parse(&text)?)).map_err(|_| "the rule is already loaded".to_owned())
}

fn parse(text: &str) -> Result<Rule, String> {
    let mut rule = Rule {
        lan: false,
        public: Vec::new(),
    };
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let bad = |what: &str| format!("line {}: {what}", n + 1);
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| bad("expected `key:value`"))?;
        let value = value.trim();
        match key.trim() {
            "lan" => {
                rule.lan = match value {
                    "true" => true,
                    "false" => false,
                    _ => return Err(bad("`lan` takes `true` or `false`")),
                }
            }
            "server" => {
                let Some((host, port)) = horizon_protocol::servers::parse_address(value) else {
                    return Err(bad("`server` takes `host` or `host:port`"));
                };
                rule.public.push((host, port));
            }
            _ => return Err(bad("unknown key")),
        }
    }
    Ok(rule)
}

/// Whether the game may play multiplayer. A managed build may only with its rule.
pub fn multiplayer() -> bool {
    !on() || RULE.get().is_some_and(Option::is_some)
}

/// Whether a world may be opened to the LAN.
pub fn lan() -> bool {
    !on() || RULE.get().and_then(Option::as_ref).is_some_and(|r| r.lan)
}

/// Where a connection to `host:port` may go: to that address by its number, or as the game
/// connects without a rule. Err: the key of the refusal, a `horizon.managed.*` string.
pub fn pin(host: &str, port: u16) -> Result<Option<SocketAddr>, &'static str> {
    if !on() {
        return Ok(None);
    }
    let Some(rule) = RULE.get().and_then(Option::as_ref) else {
        return Err(NO_RULE);
    };
    if rule.public.iter().any(|(h, p)| same_host(h, host) && *p == port) {
        return Ok(None);
    }
    let addrs: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|_| UNRESOLVED)?
        .collect();
    verdict(&addrs).map(Some)
}

/// [`pin`] on addresses already resolved, so tests need no name server: the one to connect to.
fn verdict(addrs: &[SocketAddr]) -> Result<SocketAddr, &'static str> {
    let Some(&first) = addrs.first() else {
        return Err(UNRESOLVED);
    };
    // One public address in the set is a name that could leave the network.
    if addrs.iter().any(|a| !is_local(a.ip())) {
        return Err(REFUSED);
    }
    Ok(first)
}

/// Connect to an address a rule checked.
pub(crate) fn connect(addr: SocketAddr, timeout: Duration) -> horizon_protocol::Result<Connection> {
    Connection::connect(&addr.ip().to_string(), addr.port(), timeout)
}

/// A status request's socket: to the address a rule checked, else as the game connects without
/// one.
pub fn status_connection(host: &str, port: u16, timeout: Duration) -> Result<Connection, String> {
    let connection = match pin(host, port).map_err(str::to_owned)? {
        Some(addr) => connect(addr, timeout),
        None => Connection::connect_server(host, port, timeout),
    };
    connection.map_err(|e| e.to_string())
}

/// One host, however it is spelled: names ignore their case and their last dot.
fn same_host(a: &str, b: &str) -> bool {
    a.trim_end_matches('.')
        .eq_ignore_ascii_case(b.trim_end_matches('.'))
}

/// An address of the local network: RFC 1918, RFC 4193, loopback, link-local, unspecified.
fn is_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private() || v4.is_loopback() || v4.is_link_local() || v4.is_unspecified()
        }
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_local(IpAddr::V4(v4)),
            None => {
                v6.is_unique_local()
                    || v6.is_loopback()
                    || v6.is_unicast_link_local()
                    || v6.is_unspecified()
            }
        },
    }
}

pub const NO_RULE: &str = "horizon.managed.no_rule";
pub const REFUSED: &str = "horizon.managed.refused";
const UNRESOLVED: &str = "horizon.managed.unresolved";
/// Why a player's game of a managed build opens no control socket (a dedicated server keeps
/// the one `horizon-server` drives it with).
pub const CONTROL_REFUSAL: &str =
    "a managed build opens no control socket for a player's game";

#[cfg(test)]
mod tests {
    use super::*;

    fn local(ips: &[&str]) -> Vec<SocketAddr> {
        ips.iter()
            .map(|ip| SocketAddr::new(ip.parse().unwrap(), 25565))
            .collect()
    }

    #[test]
    fn a_rule_reads_its_lines() {
        let rule = parse("# the club\nlan: true\n\nserver:mc.example.org:25570\nserver:10.0.0.1\n").unwrap();
        assert!(rule.lan);
        assert_eq!(
            rule.public,
            [("mc.example.org".to_owned(), 25570), ("10.0.0.1".to_owned(), 25565)]
        );
        assert!(!parse("").unwrap().lan);
        assert!(parse("lan: maybe").is_err());
        assert!(parse("server:").is_err());
        assert!(parse("colour: blue").is_err());
        assert!(parse("lan true").is_err());
    }

    #[test]
    fn local_addresses_are_the_private_ranges() {
        for ip in [
            "10.255.0.1",
            "172.16.0.1",
            "172.31.255.254",
            "192.168.1.2",
            "127.0.0.1",
            "169.254.1.1",
            "0.0.0.0",
            "::1",
            "fc00::1",
            "fd12:3456:789a::1",
            "fe80::1",
            "::ffff:10.1.2.3",
            "::ffff:192.168.0.1",
        ] {
            assert!(is_local(ip.parse().unwrap()), "{ip}");
        }
        for ip in [
            "8.8.8.8",
            "172.32.0.1",
            "172.15.255.255",
            "192.169.0.1",
            "100.64.0.1",
            "192.0.2.1",
            "224.0.0.1",
            "2001:4860:4860::8888",
            "2001:db8::1",
            "::ffff:8.8.8.8",
            "64:ff9b::808:808",
        ] {
            assert!(!is_local(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn a_name_stays_on_the_local_network() {
        let addrs = local(&["10.1.2.3", "192.168.5.6"]);
        assert_eq!(
            verdict(&addrs),
            Ok(SocketAddr::new("10.1.2.3".parse().unwrap(), 25565))
        );
        assert_eq!(verdict(&local(&["8.8.8.8"])), Err(REFUSED));
        // One public address among private ones refuses the whole name.
        assert_eq!(verdict(&local(&["10.1.2.3", "1.1.1.1"])), Err(REFUSED));
        assert_eq!(verdict(&[]), Err(UNRESOLVED));
    }

    #[test]
    fn a_listed_host_is_the_same_in_any_case() {
        assert!(same_host("MC.Example.org", "mc.example.org."));
        assert!(same_host("10.0.0.1", "10.0.0.1"));
        assert!(!same_host("mc.example.org", "mc.example.org.evil.net"));
    }
}

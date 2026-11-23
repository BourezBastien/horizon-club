//! The text of `managed.txt`, the rule of the game (`sync`): what `club.toml` says, as the
//! game's `managed.rs` reads it. The window never writes this file; only the supervisor's tool
//! does, so what a student's game is allowed to do is never a student's to change.

use horizon_launcher::config::Config;
use horizon_launcher::online;
use std::net::IpAddr;

/// `managed.txt`'s text from the club's configuration: whether a world may be opened to the LAN,
/// and one `server:` line for each club server outside the local network (the managed game
/// connects to local addresses anyway; a `server:` line is the club's say-so for the others).
/// `resolve` is how a server's name is looked up: an address a name resolves to decides whether
/// it is on the local network, and a name that resolves to nothing gets its line, so the game,
/// not the tool, answers for it at run time.
pub fn text(config: &Config, resolve: &dyn Fn(&str) -> Vec<IpAddr>) -> String {
    let mut lines = vec![format!("lan:{}", config.lan.open)];
    for server in &config.servers {
        if !on_the_local_network(&server.address, resolve) {
            lines.push(format!("server:{}", server.address));
        }
    }
    lines.join("\n") + "\n"
}

/// Whether an address as written in a `[[servers]]` entry names a host on the local network: a
/// literal local address does; a name does when every address it resolves to is local.
fn on_the_local_network(address: &str, resolve: &dyn Fn(&str) -> Vec<IpAddr>) -> bool {
    let host = host_of(address);
    if let Ok(ip) = host.parse::<IpAddr>() {
        return online::local(&ip.to_string());
    }
    let resolved = resolve(host);
    !resolved.is_empty() && resolved.iter().all(|ip| online::local(&ip.to_string()))
}

/// The host of an address as the game's list writes it: `host`, `host:port`, or `[v6]:port`.
pub(crate) fn host_of(address: &str) -> &str {
    let address = address.trim();
    if let Some(rest) = address.strip_prefix('[') {
        return rest.split(']').next().unwrap_or(rest);
    }
    match address.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') && port.chars().all(|c| c.is_ascii_digit()) => {
            host
        }
        _ => address,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_launcher::config::Server;

    fn config(lan: bool, servers: &[(&str, &str)]) -> Config {
        Config {
            lan: horizon_launcher::config::Lan { open: lan },
            servers: servers
                .iter()
                .map(|(name, address)| Server {
                    name: (*name).into(),
                    address: (*address).into(),
                })
                .collect(),
            ..Config::default()
        }
    }

    fn no_names(_: &str) -> Vec<IpAddr> {
        Vec::new()
    }

    #[test]
    fn the_rule_says_whether_the_lan_is_open() {
        assert_eq!(text(&config(true, &[]), &no_names), "lan:true\n");
        assert_eq!(text(&config(false, &[]), &no_names), "lan:false\n");
    }

    /// A server outside the local network gets its line, one for each; a local one needs none:
    /// the managed game reaches local addresses by its own rule.
    #[test]
    fn only_servers_outside_the_local_network_get_a_line() {
        let club = config(
            true,
            &[
                ("Serveur du club", "mc-club.example.edu:25565"),
                ("Salle info", "192.168.10.5"),
                ("Deuxième", "10.0.0.9:25570"),
            ],
        );
        assert_eq!(
            text(&club, &no_names),
            "lan:true\nserver:mc-club.example.edu:25565\n",
            "a name is not a local address written as one, so it keeps its line"
        );
        // A name the club's network answers for is local, by any address it resolves to.
        let resolve = |host: &str| match host {
            "local.school.example.edu" => vec!["192.168.1.5".parse::<IpAddr>().unwrap()],
            "both.example.edu" => vec![
                "192.168.1.5".parse::<IpAddr>().unwrap(),
                "51.10.2.3".parse::<IpAddr>().unwrap(),
            ],
            _ => Vec::new(),
        };
        let club = config(
            false,
            &[
                ("Dans le club", "local.school.example.edu"),
                ("Dehors", "both.example.edu:25565"),
                ("Inconnu", "unknown.example.edu"),
            ],
        );
        assert_eq!(
            text(&club, &resolve),
            "lan:false\nserver:both.example.edu:25565\nserver:unknown.example.edu\n",
            "one local address among others is still outside; a name that resolves to nothing \
             keeps its line, for the game to answer at run time"
        );
    }

    /// An address as the game's list writes it: a bare host, a port after it, or brackets for
    /// IPv6.
    #[test]
    fn the_host_of_an_address_is_read_as_the_game_reads_it() {
        assert_eq!(host_of("mc.example.edu"), "mc.example.edu");
        assert_eq!(host_of("mc.example.edu:25565"), "mc.example.edu");
        assert_eq!(host_of("192.168.1.5"), "192.168.1.5");
        assert_eq!(host_of("192.168.1.5:25565"), "192.168.1.5");
        assert_eq!(host_of("[fd12:3456::1]:25565"), "fd12:3456::1");
        assert_eq!(host_of("fd12:3456::1"), "fd12:3456::1");
        assert_eq!(host_of("  mc.example.edu:25565  "), "mc.example.edu");
    }
}

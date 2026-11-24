//! The game's server list, `servers.dat` in the game home: the game's Multiplayer screen reads
//! it. The club decides what it holds (docs/school-edition/launcher-target.md, section 6): before
//! every Play, and whenever the launcher's Multiplayer tab refreshes, the launcher rewrites the
//! list with the servers of `club.toml`, then the hosted servers announced in the club's
//! `online\` folder. A student adds none and removes none.
//!
//! Its format is vanilla's (`ServerList.load`/`save`, `ServerData.read`/`write`): uncompressed
//! NBT, `servers` a list of compounds with `name`, `ip`, an optional `icon` (a PNG in base64),
//! `acceptTextures`, `acceptedCodeOfConduct` and `hidden`. An entry the launcher writes takes no
//! server resource pack: `acceptTextures` set and false, where the game would otherwise ask.

use crate::{config::Config, online::Announcement};
use base64::Engine;
use horizon_platform::world::nbt::{self, Compound, JavaString, Limits, NamedTag, Tag};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const FILE: &str = "servers.dat";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Server {
    pub name: String,
    /// Where to find it: `host` or `host:port`.
    pub ip: String,
    /// The server's picture, as the game last saw it (PNG). None on a server the launcher
    /// lists; [`replace`] sets it on what it wrote.
    pub icon: Option<Vec<u8>>,
}

impl Server {
    pub fn new(name: impl Into<String>, ip: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ip: ip.into(),
            icon: None,
        }
    }
}

/// The servers the game's list holds: those of `club.toml`, in its order, then the hosted servers
/// `announced` in the club's `online\` folder ([`crate::online::list`]), then those `remembered`
/// that aren't announced now ([`crate::invited`]): a friend's server that is off is still listed,
/// to be joined from the game when it comes back.
pub fn club_servers(
    config: &Config,
    announced: &[Announcement],
    remembered: &[crate::invited::Invited],
) -> Vec<Server> {
    config
        .servers
        .iter()
        .map(|server| Server::new(&server.name, &server.address))
        .chain(announced.iter().map(|a| Server::new(&a.name, a.address())))
        .chain(
            remembered
                .iter()
                .filter(|i| {
                    !announced
                        .iter()
                        .any(|a| crate::invited::Invited::of(a).same(i))
                })
                .map(|i| Server::new(&i.name, i.address())),
        )
        .collect()
}

fn key(k: &str) -> JavaString {
    JavaString::from(k)
}

fn text(entry: &Compound, k: &str) -> String {
    match entry.get(&key(k)) {
        Some(Tag::String(s)) => s.to_string_lossy(),
        _ => String::new(),
    }
}

/// The entries of the list in `home`, as the file holds them; empty when there is no file or it
/// can't be read, as the game reads it.
fn entries_in(home: &Path) -> Vec<Compound> {
    let bytes = match fs::read(home.join(FILE)) {
        Ok(bytes) => bytes,
        Err(_) => return Vec::new(),
    };
    let Ok((root, _)) = nbt::read_named(&bytes, Limits::default()) else {
        return Vec::new();
    };
    let Some(Tag::List(list)) = root.tag.get("servers") else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|t| match t {
            Tag::Compound(c) => Some(c.clone()),
            _ => None,
        })
        .collect()
}

/// The pictures the list in `home` holds, by address: what the game last saw of each server.
fn pictures_in(home: &Path) -> HashMap<String, String> {
    entries_in(home)
        .into_iter()
        .filter_map(|entry| {
            let icon = text(&entry, "icon");
            (!icon.is_empty()).then(|| (text(&entry, "ip"), icon))
        })
        .collect()
}

fn entry(server: &Server, icon: Option<&str>) -> Compound {
    let mut fields = vec![
        (key("name"), Tag::String(server.name.as_str().into())),
        (key("ip"), Tag::String(server.ip.as_str().into())),
    ];
    if let Some(icon) = icon {
        fields.push((key("icon"), Tag::String(icon.into())));
    }
    // The game reads a set `acceptTextures` as the pack's status, true or false; unset, it asks.
    fields.push((key("acceptTextures"), Tag::Byte(0)));
    fields.push((key("hidden"), Tag::Byte(0)));
    fields.into_iter().collect()
}

/// Rewrites the game's server list in `home` with the club's servers and the hosted servers
/// `announced` ([`club_servers`]): before every Play, and whenever the launcher's Multiplayer tab
/// refreshes. The servers as written; Err: why the list could not be saved.
pub fn write_club_list(
    home: &Path,
    config: &Config,
    announced: &[Announcement],
    remembered: &[crate::invited::Invited],
) -> Result<Vec<Server>, String> {
    replace(home, &club_servers(config, announced, remembered))
}

/// Now, in seconds since the Unix epoch: what an announcement's age is read against.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// Rewrites the game's server list in `home` with `servers`, in this order and no other (the
/// entries joined through Direct Connection go too): the game's own backup of what was there is
/// kept as `servers.dat_old`. A server that was already in the list keeps the picture the game
/// last saw of it. The servers as written, with those pictures; Err: why the list could not be
/// saved.
pub fn replace(home: &Path, servers: &[Server]) -> Result<Vec<Server>, String> {
    let pictures = pictures_in(home);
    let kept: Vec<(Server, Option<String>)> = servers
        .iter()
        .map(|server| {
            let icon = pictures.get(&server.ip).cloned();
            (server.clone(), icon)
        })
        .collect();
    let root = NamedTag {
        name: "".into(),
        tag: Tag::compound([(
            "servers",
            Tag::List(
                kept.iter()
                    .map(|(server, icon)| Tag::Compound(entry(server, icon.as_deref())))
                    .collect(),
            ),
        )]),
    };
    let bytes = nbt::write_named(&root, Limits::default()).map_err(|e| e.to_string())?;
    let (new, current) = (home.join("servers.dat_new"), home.join(FILE));
    fs::create_dir_all(home)
        .and_then(|()| fs::write(&new, bytes))
        .and_then(|()| match current.exists() {
            true => fs::copy(&current, home.join("servers.dat_old")).map(|_| ()),
            false => Ok(()),
        })
        .and_then(|()| fs::rename(&new, &current))
        .map_err(|e| {
            crate::i18n::trf(
                "The game's server list could not be saved: {error}",
                &[("error", &e.to_string())],
            )
        })?;
    Ok(kept
        .into_iter()
        .map(|(server, icon)| Server {
            icon: icon.and_then(|icon| base64::engine::general_purpose::STANDARD.decode(icon).ok()),
            ..server
        })
        .collect())
}

#[cfg(test)]
#[path = "server_list/tests.rs"]
mod tests;

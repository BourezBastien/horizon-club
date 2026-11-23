# Launcher trim map — Horizon Club (school edition)

Working map for deleting modules from `launcher/` (own workspace, crate `horizon-launcher`).
Every reference below was verified with `rg` or by reading the file at the time of writing
(2026-11-22). Line numbers drift; re-rg before each step. `src/` = `launcher/src/`.
"cost" = reference sites in that kept file that must change (module paths + methods + fields).

## 1. Module inventory

### 1a. lib.rs modules (`launcher/src/lib.rs:2-83`), ~110k of the ~150k lines

| Module | File(s) | LOC | Verdict |
|---|---|---|---|
| account | account.rs + account/{session,tests}.rs | 2585+2999 | DELETE |
| agents | agents.rs | 214 | DELETE |
| backend | backend.rs + backend/{messages,settings,release_*,update_selection,installation,play_log,look_settings,developer_play,*_tests}.rs | 8127+1490 | KEEP (trim: §2) |
| browser | browser.rs + browser/ (incl. horizon/, installed/, java_instance/) | 510+4987 | DELETE |
| build_import | build_import.rs | 643 | DELETE |
| cache_cleanup | cache_cleanup.rs + cache_cleanup/tests.rs | 499+371 | UNDECIDED → recommend KEEP (backend.rs:1378 uses `delete_unleased_game_unit`) |
| data_location | data_location.rs + tests | 1131+1325 | KEEP |
| datapacks | datapacks.rs + tests | 289+139 | KEEP (uses packs, §3) |
| desktop | desktop.rs | 356 | KEEP |
| desktop_seams | desktop_seams.rs | 110 | KEEP (not on your list; used by datapacks.rs:152, instances.rs:463) |
| developer_install | developer_install.rs | 393 | DELETE |
| developer_offline | developer_offline.rs | 292 | DELETE |
| download_retry | download_retry.rs (pub(crate)) | 98 | KEEP (not on your list; backend.rs:508) |
| downloads | downloads.rs + tests | 1009+832 | KEEP |
| effects | effects.rs | 104 | KEEP |
| export | export.rs + tests | 346+328 | DELETE |
| game_app | game_app.rs (unix) | 481 | KEEP (not on your list; backend.rs:3118) |
| game_log | game_log.rs + tests | 158+114 | KEEP (has mclo.gs upload + account names, §2) |
| game_shader_api | game_shader_api.rs | 642 | DELETE |
| github | github.rs + tests | 294+444 | DELETE (backend uses it, §3) |
| github_mods | github_mods.rs + tests | 621+385 | DELETE |
| heads | heads.rs | 386 | KEEP, strip network part (§3) |
| i18n | i18n.rs + tests | 664+254 | KEEP |
| import | import.rs + tests | 334+408 | DELETE |
| installer | installer.rs (windows) | 739 | DELETE |
| instances | instances.rs + instances/{switch,tests}.rs | 814+848 | KEEP, reduce to one fixed instance |
| instances_mrpack | instances_mrpack.rs + tests | 73+113 | DELETE |
| java | java.rs + tests | 286+151 | DELETE (backend uses it, §3) |
| java_launch | java_launch.rs + tests | 490+413 | DELETE (backend uses it, §3) |
| jars | jars.rs + jars/launch.rs + jars_tests.rs | 213+91+467 | DELETE (servers.rs uses it, §3) |
| launch | launch.rs + launch/{hook,tests}.rs | 572+1023 | UNDECIDED → recommend KEEP: game-start wrapper (Launch, Vars, wrap, wrap_stoppable, unless_stopped, run_after_exit, instance_vars); used by instances.rs:11,228-230, launcher_play.rs:329,355, servers.rs:728, windows_runtime.rs:262, backend/messages.rs:77, main.rs:517 |
| loaders | loaders.rs + loaders/ | 306+1303 | DELETE |
| mac_update | mac_update.rs + tests (macos) | 168+357 | KEEP-adapt (not on your list; backend.rs:3928,3936 self-update; delete with self-update) |
| min_launcher | min_launcher.rs | 184 | DELETE (launcher_setup.rs:574-586) |
| minecraft_versions | minecraft_versions.rs | 170 | DELETE (backend.rs:2200) |
| modpacks | modpacks.rs | 862 | DELETE (instances.rs uses it, §3) |
| mojang | mojang.rs + mojang_tests.rs | 524+609 | KEEP |
| native_cache | native_cache.rs + tests (mod, private) | 303+460 | DELETE (listed under "every other native_*") — **⚠ recommend KEEP**: it is the install-cache/lease layer (Error, linked, checked, locked, LockMode, PROFILES) used by backend.rs:1375,4418-4426,4616,4666, refusals.rs:139,428, process/leases.rs:3, windows_runtime.rs:76, cache_cleanup |
| native_fit | native_fit.rs | 147 | DELETE (no kept refs) |
| native_mods | native_mods.rs + native_mods/ (27 files) | 5695+12732 | DELETE, but small version-metadata part is shared (§3) |
| native_status | native_status.rs | 1194 | DELETE (glass/blocks_versions + launcher_setup use it, §2) |
| nightly | nightly.rs | 740 | DELETE, but tag-formatting half is shared (§3) |
| other_launchers | other_launchers.rs + tests | 594+431 | DELETE |
| packs | packs.rs + tests | 472+295 | DELETE (datapacks + glass tabs use it, §3) |
| pictures | pictures.rs | 529 | KEEP |
| private_window | private_window.rs | 401 | DELETE (launcher_play.rs:586) |
| process | process.rs + process/{leases,supervisor}+tests | 32+408+~900 | KEEP (not on your list; backend.rs:2096,2670,2678,3747,4476) |
| proxy | proxy.rs | 154 | DELETE (listed) — **⚠ recommend KEEP or inline**: downloads.rs:312, backend.rs:471, heads.rs:102 build all HTTP through it |
| refusals | refusals.rs | 477 | KEEP |
| relocation | relocation.rs + tests | 379+245 | DELETE (move_window.rs:17 + startup.rs:78 use it, §2) |
| run_leases | run_leases.rs + run_leases/{control,tests}.rs (mod, private) | 627+605 | UNDECIDED → **KEEP**: `is_horizon/mark/snapshot/shared/spawn/publish/installation/game_units`, `Claim`, `GAME`, `SOURCE`, `control::startup_cleanup` used by backend, servers, refusals, windows_runtime, process/leases, instances.rs:652; tells Horizon's game from Java Edition |
| rust_toolchain | rust_toolchain.rs | 1964 | DELETE (blocks_settings.rs:343,353; process.rs:20 comment) |
| screenshots | screenshots.rs | 103 | DELETE |
| server_list | server_list.rs + tests | 282+400 | KEEP (§3 telemetry consts) |
| servers | servers.rs + servers/{remote,*tests}.rs | 1468+785 | KEEP (hosting; uses jars/run_leases/native_mods::cache_error, §2) |
| setup_args | setup_args.rs | 154 | KEEP-adapt (not on your list; startup.rs:107 names `setup_args::Mode`, installer.rs:32-383 uses it) |
| shop | shop.rs + shop/tests.rs | 273+240 | DELETE |
| shortcut | shortcut.rs + tests | 473+225 | KEEP (cli/installer use it too; game_app.rs:65 needs `shortcut::xml`) |
| signed | signed.rs | 266 | DELETE (backend release verification, §3) |
| sources | sources.rs + sources_tests.rs | 666+691 | KEEP |
| storage | storage.rs + storage/{eviction,inventory,listing,policy,*tests}.rs | 494+2603 | DELETE (backend/instances/glass use parts, §3) |
| tailscale | tailscale.rs + tests | 507+270 | DELETE (no kept refs) |
| telemetry | telemetry.rs | 492 | DELETE (many kept callers, §2) |
| themes | themes.rs + tests | 845+219 | KEEP |
| uninstall_aside | uninstall_aside.rs | 108 | DELETE (only installer.rs:268,287) |
| update_handoff | update_handoff.rs | 729 | DELETE (startup/launcher_setup/blocks_settings/backend/main, §2) |
| update_recovery | update_recovery.rs | 37 | KEEP-adapt (not on your list; backend.rs:4304 self-update advice) |
| wardrobe | wardrobe.rs + tests | 727+655 | DELETE |
| windows_runtime | windows_runtime.rs (mod, windows) | 414 | KEEP (uses run_leases, §2) |
| worlds | worlds.rs + tests | 339+246 | KEEP |
| test_support | test_support.rs (cfg test) | 13 | KEEP while run_leases tests stay (run_leases/tests.rs:173) |
| (nested) backend::messages | backend/messages.rs | 79 | KEEP, trim Msg variants (§5) |
| (nested) backend::settings | backend/settings.rs | 415 | KEEP, trim fields (§2) |

### 1b. main.rs modules (`main.rs:7-103`), GUI

KEEP: blocks_chrome, blocks_play, blocks_settings, blocks_versions, launcher_messages, launcher_play, launcher_render, launcher_setup, widgets, blocks, console_page, cull, datapacks_page, developer_play, frame, glass_home, glass_home_lists, glass_home_tabs, glass_palette, glass_settings, glass_toast, glass_ui, glass_versions, glide, home, home_tabs, language_page, line_edit, motion, move_window, multiplayer_page, nav_strip, perf, picture_cache, plate, scrollbar, servers_page, shell, theme, themes_page, titlebar, while_playing, window_root, worlds_page, startup (mod at main.rs:692).
Not on your lists, classified by usage: console.rs (KEEP-adapt: startup.rs:62 `attach_for_cli`, only CLI needs it), markdown.rs (KEEP: notes_cache.rs:4), notes_cache.rs (KEEP: main.rs:391), specimen.rs (KEEP: launcher_render.rs:7,14; debug-only design page), glass_shot_blur.rs (KEEP: picture_cache.rs:136 `Look::Blurred`), sticky.rs (DELETE: only main.rs:279/launcher_setup.rs:93 `mods_sticky`).

| Module | File | LOC | Verdict |
|---|---|---|---|
| agents_page | agents_page.rs | 265 | DELETE (glass_settings.rs:672-674 `no_ai_row`/`agents_section`) |
| build_mode | build_mode.rs | 1177 | DELETE (glass_home.rs:462) |
| cli | cli.rs + cli/{progress,developer_builds,settings_output,storage_tests}.rs + cli_catalog_tests.rs + cli_native_permission.rs | 4129+351+80 | DELETE (only startup.rs:109 `cli::run()`) |
| cli_tools | cli_tools.rs | 514 | DELETE (no kept refs) |
| collection | collection.rs | 484 | DELETE (data-consent overlay, §5) |
| export_page | export_page.rs | 236 | DELETE (main.rs:613) |
| game_tools | game_tools.rs | 489 | DELETE (no kept refs; MCP `game_*` tools) |
| github_mods_page | github_mods_page.rs | 597 | DELETE (main.rs:599, launcher_setup.rs:280, launcher_render tick) |
| glass_account | glass_account.rs | 205 | DELETE (glass_settings.rs:622 `self.glass_account(cx)`) |
| glass_home_mods | glass_home_mods.rs | 668 | DELETE (glass_home_tabs.rs:89 `glass_instance_mods`) |
| glass_home_packs | glass_home_packs.rs | 340 | DELETE (glass_home_tabs.rs:94-104 `glass_packs`/`glass_screenshots`) |
| glass_instance | glass_instance.rs | 1494 | DELETE (frame.rs:181 COLOURS, glass_server_dialog.rs:7 helpers, §5 dialog) |
| glass_instance_java | glass_instance_java.rs | 299 | DELETE (only glass_instance) |
| glass_mod_states | glass_mod_states.rs | 437 | DELETE (main.rs:566, launcher_setup.rs:365 `mod_preview_from_env`) |
| glass_sheet | glass_sheet.rs | 1098 | DELETE (frame.rs:1352-1389, glass_home.rs:77+264+913+1064, launcher_setup.rs:353, main.rs:413 `SheetTab`) |
| glass_storage | glass_storage.rs | 729 | DELETE (glass_settings.rs:200,724; main.rs:323 `Trims`) |
| import_page | import_page.rs | 160 | DELETE (launcher_render.rs:330-331 drop target, §5) |
| instance_icons | instance_icons.rs | 211 | DELETE (frame.rs:157,186) |
| instance_mods | instance_mods.rs | 1252 | DELETE (home_tabs.rs:51,109 + glass tab, §7) |
| instances_page | instances_page.rs | 1183 | DELETE — **⚠ core Home helpers live here** (§3) |
| internet | internet.rs | 842 | DELETE (servers_page.rs:510,1941; glass_servers.rs:806; §2) |
| jar_launch | jar_launch.rs | 136 | DELETE (developer_play.rs:113, launcher_messages.rs:154, servers_page.rs:31) |
| jars_tests | (in jars, above) | 467 | DELETE |
| launch_page | launch_page.rs | 893 | DELETE (main.rs:614,616 `MemorySlider`) |
| location_page | location_page.rs | 314 | DELETE (launcher_setup.rs:182-183, glass_settings.rs:684 `location_row`, refusals.rs:107) |
| mcp | mcp.rs | 378 | DELETE (no kept refs) |
| microsoft | microsoft.rs | 110 | DELETE (blocks_settings.rs:765-877, blocks_play.rs:198,286, frame.rs:1093) |
| mod_icons | mod_icons.rs | 28 | DELETE (instance_mods.rs:182,214) |
| modpacks_page | modpacks_page.rs | 792 | DELETE (main.rs:581, launcher_messages.rs:397-398) |
| mods_page | mods_page.rs | 2625 | DELETE (glass_palette.rs:159, launcher_play.rs:448-475, main.rs:598, launcher_setup.rs:279) |
| mods_project | mods_project.rs | 1353 | DELETE (no kept refs; uses markdown) |
| native_build | native_build.rs | 292 | DELETE (launcher_messages.rs:276,361 `native_look_again`/`native_failed`) |
| native_consent | native_consent.rs | 136 | DELETE (`PendingConsent` type used by main.rs:120,372 + blocks_settings.rs:321,368) |
| native_log_page | native_log_page.rs | 439 | DELETE (main.rs:386, launcher_render.rs:235,338, launcher_messages.rs:554) |
| native_registry_consent | native_registry_consent.rs | 30 | DELETE (launcher_messages.rs:317) |
| new_instance | new_instance.rs | 586 | DELETE (glass_home.rs:225-227, launcher_setup.rs:343-345 `NewKind`) |
| owner_page | owner_page.rs | 558 | DELETE (`asking_owner`/`owner_screen`, §5; main.rs:149 comment) |
| packs_page | packs_page.rs | 336 | DELETE (main.rs:609, launcher_setup.rs:287; home_tabs `pack_counts`/`packs_section`) |
| partner | partner.rs | 362 | DELETE (frame.rs:686-693, blocks_chrome.rs:319-326, servers_page.rs:1416-1423, glass_servers.rs:824-846, launcher_setup.rs:34-39, backend/settings.rs:129) |
| profile_heads | profile_heads.rs | 154 | DELETE (main.rs:475, launcher_setup.rs:215, launcher_play.rs:498 `load_profile_heads`) |
| screenshots_page | screenshots_page.rs | 356 | DELETE (home_tabs screenshot sections, §5 viewer) |
| setup_page | setup_page.rs (windows) | 481 | DELETE (startup.rs:135) |
| shortcut_page | shortcut_page.rs | 73 | DELETE (main.rs:622) |
| sticky | sticky.rs | 122 | DELETE (§1b note) |
| switch_page | switch_page.rs | 113 | DELETE (`who_plays`/`players`/`play_as`/`account_menu` used by blocks_chrome.rs:84, frame.rs:1013-1268) |
| welcome | welcome.rs | 229 | DELETE (`welcoming`/`welcome_escape`/`welcome_screen`, §5) |
| wardrobe_page | wardrobe_page.rs + wardrobe_page/{marks,purchases,shop,viewer}+shop/{buy,thumbs} | 1615+1680 | DELETE (not named in your lists; it *is* the Wardrobe/Shop page: launcher_render.rs:73-74,79, main.rs:420-426, launcher_setup.rs:172-177,332,356-363) |

Counts: **DELETE ≈ 82 modules** (56 lib-file groups + 26 GUI groups incl. wardrobe_page), **KEEP ≈ 63**
(46 lib + 17 extra), **UNDECIDED 3** (launch, run_leases, cache_cleanup — all recommended KEEP).

## 2. Reverse reference index (references from KEEP files into DELETE/UNDECIDED)

Grouped by the kept file that must change. Format `file:line — what`. Counts include the
method/field references listed; re-verify before editing.

### backend.rs — cost ≈ 101 refs (highest; most become dead once their GUI dies)
Self-update: :514,668-670,679,788,1126 `crate::github::*` (release list fetch); :700,1175,1306,1315 `crate::signed::*`; :871,927 `crate::min_launcher::version_in`; :4274 `crate::installer::after_update`; :4348,4376,7050 `crate::update_handoff::MIN_RESTARTED_ENV`; :3928,3936 `crate::mac_update`. Accounts/session: :2141-2146 `account::{Accounts,launch,Endpoints,client_id,store}`; :2187 `account::java_player`; :2271-2278 `account::Launch/NoOwner`; :4887-4942+4981-4982 (tests). Java: :2212-2213,2244,2249,2259,2604 `crate::{java,java_launch}`; :2200 `crate::minecraft_versions::load`; :2232 `crate::loaders::install`; :2333-2370 `prepare::java::*` (platform, kept). Native/versions: :980,1374-1379,1421-1424,1816,3294,3361,4422-4433,4449,5107-5114 `crate::native_mods::{resource_dependent(s),about,copy_base?(comment),cache_error}`; :1375,4418,4426,4616,4666 `crate::native_cache::Error` (→ §3). Leases/cleanup: :3089,3130 `crate::run_leases::mark`; :4398-4463 `run_leases::{Claim,shared,is_horizon,spawn,installation}`; :4405 `pub use run_leases::control::startup_cleanup`; :4603,4615 tests; :1378,4609,4613,4665 `crate::cache_cleanup::*`. Storage: :3186 `crate::storage::eviction::request_cleanup`. Telemetry: :3091 `crate::telemetry::ping_only_if_allowed`. Proxy: :471 `crate::proxy::agent`. Developer: :3086,3127 `crate::developer_install::uses_developer`; :3118-3120 `crate::game_app` (kept). Nightly: :587,738,772,1121-1141,3229,3652,3665,6910-6916 `crate::nightly::*`; :2174 comment `launch::wrap`.

### main.rs — cost ≈ 54 refs + struct fields (§4)
:111 `use horizon_launcher::account::{self,Accounts,SignIn,SignInState}`; :113 `use build_import`; :116-117 `use native_mods/native_status`; :118 `use storage`; :119 `use telemetry`; :120 `use native_consent::PendingConsent`; :131-143 `Page` variants Mods/Wardrobe/Shop; :153-156 `Page::from_env` mods/wardrobe/shop + `horizon_cosmetics::forced()`; :171-179 `Page::ALL` entries; :183-189 `in_top_bar` wardrobe filter; :194-205 `SelfUpdate`; :473-489 account/owner fields; :502-504,511 friend join; :517 `launch::Vars`; :527-533 collection/agents/playing/official_players; :558-617 mods/packs/native/storage/screenshots fields; :644-650 `shut_down`: `servers_page::stop_on_quit()` (keep), `telemetry::send_before_exit()` (remove), `update_handoff::on_close()` (remove); :890 test constructs `account::Account`.

### launcher_setup.rs — cost ≈ 35
:7 `game_shader_api::draws_here`; :8 `storage::eviction::cleanup_after_start`; :28-50 partner-seen timer (`partner::SEEN_BEFORE_HIDE`); :93 `mods_sticky`; :108-111 `update_handoff::take_failure/min_restarted`; :166 account_menu; :172-177 wardrobe/shop fields; :182-183 `location_page::startup_error/note`; :208 `internet::Internet::new`; :215 `profile_heads`; :239-240 collection/agents; :279-281 mods_page/github_mods_page/instances_page; :284 screenshots_page; :287 packs_page; :290-292 export_page/launch_page/memory_slider; :293 import_page; :296 shortcut_page; :317-319 `account::store/no_keychain` thread; :327-331 `load_profile_heads`, `mods_auto_update`, `watch_shop`; :342-355 new_instance/glass_sheet env previews; :356-363 `wardrobe_preview`; :365 `mod_preview_from_env`; :366-368 `telemetry::send_in_background/watch_playing/watch_official`; :419-465 `go()`: :434-440 storage_revoke/counting/visit, :441 `wardrobe_reload`, :456-458 `mods_sticky.reset` + `mods.reset_lists`, :463 `mods.browse.sort_menu`; :492,558-560 `update_handoff::installs_on_close/start_now`; :574-586 `min_launcher`; :606-615 `native_status::Build` / `native_mods::is_native`.

### instances.rs — cost ≈ 27
:10 `browser::horizon::pack` (registry packs); :11 `crate::launch`; :12 `crate::modpacks`; :90,123 `browser::images::{fit_png,fetch}` (instance pictures); :219 `loaders::Loader` field; :236-237 `browser::java_instance::Target`; :326,340 `nightly::stable_latest` (channel selection); :388-389,538,622,784 `modpacks::{listed_in,profile_home,slug,apply_profile}` (Port/Pack instance kinds); :414,183,289 `native_mods::Profile`; :463 `desktop_seams` (kept); :472 `storage::eviction::request_cleanup`; :652 `run_leases::publish` + `native_mods::cache_error`; :796-798 `horizon_telemetry::consent::Consent::path` (moves consent file with instance). instances/switch.rs: :7 loaders::Loader, :121 `browser::java::MODS_DIR`, :122 `native_mods::INSTALLED`. instances/tests.rs: 12 refs (72,211,223,467-468,515,525,532-539,661).

### launcher_messages.rs — cost ≈ 25 (+ §5 handlers)
:9 `game_shader_api::answers`; :19 `Page::Wardrobe|Page::Shop` filter in bench; :39,47,203 `nightly::note_published/date_of`; :64 `telemetry::set_official_server`; :66 `follow_min_launcher`; :70-113 Keychain/Owner/Account*/Session msgs; :87 `account::session::State::Ready`; :114-115 Playing/OfficialPlayers (telemetry watchers); :120,159 `launch::unless_stopped`; :130,134-137,170,177,478 `account::{take_unsaved,NoOwner,discard_unread_sessions}`; :154 `jar_launch::consent`; :203 nightly; :276 `native_look_again`; :291-299 Msg::Imported; :316-371 NativeRegistryConsent/NativeProgress/Native msgs (`native_mods::{Failed,CANCELLED,Caller,AtStake}`); :384 `update_handoff::set_pending`; :389 `follow_min_launcher`; :391-402 PackLine/PackDone/ImportedPacks/FoundPacks; :483 `self.screenshots.stale()`; :492 `launch::run_after_exit`; :502 `refusals::crash_report` (kept); :552-554 `mods_tick`/`github_tick`/`native_log_read`.

### blocks_settings.rs — cost ≈ 22
:76 `update_handoff::installs_on_close`; :207 "Accounts, updates…" caption; :310 `native_status::enabled`; :321,368 `native_consent::resolve`; :334-339 native consent dialog (keep only if dialog stays); :343,353 `rust_toolchain::host/installed`; :345 `storage::format_bytes`; :379 `browser::java::set_enabled`; :511,555,588 `proxy::refuse/announce` (proxy recommended kept); :593-704 `account_rows` (account::session states, `account::store`, has_owner); :610-877 `microsoft_rows` (`microsoft::has_private_window/use_different_account/open_private/sign_in_button`); :623,694 `Accounts::load`; :648-813 `SignInState::*`.

### launcher_play.rs — cost ≈ 20
:11 `switched_after_start_crash` (kept, backend); :87 `nightly::install_stable`; :171 `build_import::import`; :329,355 `launch::wrap_stoppable` + `launch::Launch`; :448-475 `mods_page::{AutoTarget,auto_update,Event}`; :479-482 `developer_offline::enabled` + `account::session_for_game`; :511-558 sign-in refresh (`account::session::{Phase,refresh_with_progress,State}`); :565 `Accounts::load().has_owner(account::store())`; :582 `begin_sign_in(account::Account)`; :586 `private_window::detected`; :498 `load_profile_heads`.

### glass_versions.rs — cost ≈ 15
:16,39,65,68,327,689,703 `nightly::{day,label,published,same_day_time}`; :238 `native_mods::about`; :263,700 `native_mods::is_native`; :245,264,688,704 `build_import::{is_imported,about,label}`.

### backend/messages.rs — cost ≈ 14 Msg variants (§5)
:6 Storage; :12 StableRetry; :26-27 LauncherProgress/LauncherUpdated; :32-42 Keychain/Owner/Account*/Session (`crate::account`); :43-46 JarConsent; :49-52 JarClient; :54-60 PackLine/PackDone/ImportedPacks/FoundPacks (`crate::modpacks::Imported`, `crate::other_launchers::Found`); :62-68 NativeProgress/NativeRegistryConsent/Native (`crate::native_mods`); :70 Imported; :74-78 JavaGame (`crate::launch::Vars` — kept).

### frame.rs — cost ≈ 14
:7 `instances_page::java_line`; :157,186 `instance_icons::centred`; :181 `glass_instance::COLOURS`; :209 `instances_page::initial`; :280 `Page::Mods` + `self.mods.on_modpacks()`; :682-693 partner card; :707-771 sidebar links (discord/sponsor/roadmap/docs/website); :870-975 `rail_update_card` (self_update); :996 `Page::in_top_bar(self.wardrobe_shown())`; :1013-1268 account chip + `glass_account_menu` (`Accounts::load`:1068, `microsoft::logo`:1093, switch_page helpers); :1352-1389 row menu `open_instance_sheet` + `glass_sheet::SheetTab::{General,Storage,Launch}`.

### startup.rs — cost ≈ 9
:57 `update_handoff::take_min_restarted_env`; :59 `native_mods::internals_rustc_wrapper`; :62 `console::attach_for_cli` (drop with cli); :75 `installer::run_quiet`; :78 `relocation::Worker::from_args` (+ move_window window); :92 `update_handoff::run`; :100 `proxy::announce` (keep if proxy stays); :105-107 `installer::window_mode` / `setup_args::Mode`; :109 `cli::run()`; :135 `setup_page::open`.

### refusals.rs — cost ≈ 9
:13-19 `run_leases::{is_horizon,game_snapshot}` (keep with run_leases); :107 comment `location_page::environment_blocked`; :139 `native_cache::linked`; :298-313 tests `run_leases::{mark,snapshot}`; :428 `native_cache::Error`.

### home.rs — cost ≈ 9
:40,363 `nightly::label`/`build_import::label`; :52 `telemetry::stats_url` (playing count); :122 `instances_page::played_line`; :190-195 telemetry PING labels; :265 `developer_offline::selected`; :313 `go(Page::Mods)`; :362 nightly label; :434 `horizon_friends::invite::parse` (friend code).

### developer_play.rs (main) — cost ≈ 9
:7-75 `account::{GameSession,NoOwner,take_unsaved,discard_unread_sessions}`; :41-49 `developer_offline::{require_capability,mark,NOTICE}`; :101 `modpacks::apply_profile`; :113 `jar_launch::client`. backend/developer_play.rs — cost ≈ 9: :11-47 `developer_offline`; :16-42 `account::{session_for_game,session_for_game_as}`; :32 `browser::horizon::before_play`.

### windows_runtime.rs — cost ≈ 8
:7 `run_leases::SOURCE`; :76,81 `run_leases::{Claim,game_units}` + `native_cache::Error`; :260,338,375 `run_leases::mark`; :262 `launch::wrap`.

### glass_home.rs — cost ≈ 8
:77,264,913,1064 `open_instance_sheet` (instances_page) + `glass_sheet::SheetTab`; :223 `open_instance_dialog(None)`; :225-227 `new_instance::NewKind`; :315,636 `developer_offline::selected`; :335 `telemetry::stats_url`; :363 `developer_install::instance_status`; :431 `native_status::State`; :462 `build_mode::percent`; :1140 `horizon_friends::invite::parse`.

### servers.rs — cost ≈ 6
:712,728 `jars::{Permission,prepare_launch}` (§3); :742-755 `run_leases::{Claim,game_units_in}` + `native_mods::cache_error` (:755).

### heads.rs — cost ≈ 6 (strip network)
:94,174,262,333,365 `crate::account::Account` param; :102-105 `crate::proxy::agent` + ureq fetch; :129-171 `session_skin` → sessionserver.mojang.com. Offline part kept: `default_face` (servers_page.rs:1299), `face`, `cached_skin`, `default_skin_file`.

### blocks_chrome.rs — cost ≈ 6
:73-84 `account_menu` toggle + `switch_page::account_menu` + `Accounts::load`; :305 accounts button (blocks_play); :319-326 partner card; :427-428 `nightly::label`/`build_import::label`.

### backend/settings.rs — cost ≈ 6
:110-129 `ping_off`/PING doc (telemetry) + `partner_seen_secs/partner_hidden`; :165 `browser::horizon::auto` comment; :249,289 `nightly::date_of` (Nightly channel).

### blocks_versions.rs — cost ≈ 5
:18 `horizon_friends::invite::parse`; :86-90 `native_mods::about`/`build_import::about`/`nightly::label`; :290 `native_mods::is_native`; :297 `build_import::is_imported`.

### backend/update_selection.rs — cost ≈ 5
:13-19 `nightly::date_of`, `build_import::is_imported`, `developer_install::uses_developer`. backend/release_selection.rs — cost ≈ 3: :18,20 `github::{json_optional,API}`, :126 `signed::title_version`. backend/release_loading_tests.rs:282 `signed::asset_url`. backend/installation.rs — cost ≈ 4: :12,55 `run_leases::{installation,publish}` + `native_mods::cache_error`. backend/play_log.rs:4 `run_leases::spawn`.

### widgets.rs — cost ≈ 4
:176-177 `nightly::label`/`build_import::label`; :495-499 `account::session::Status/State` badge.

### servers_page.rs — cost ≈ 4 + 3 method sites
:31 `jar_launch::with_consent`; :397 `own_name(&Accounts)`; :510 `self.internet_tick(overview)`; :1416-1423 partner banner; :1941 `self.internet_panel(port, cx)`; :3029 `create_dialog` (kept).

### glass_servers.rs — cost ≈ 2 + 1
:806 `self.internet_invites(...)`; :824,846 partner banner (`partner_seen_secs`, `partner::banner_image`).

### glass_palette.rs — cost ≈ 2 + 5
:12 `browser::Project`; :89 `wardrobe_shown`; :153 `open_instance_dialog`; :159 `mods_page::InstallRequest` + `mods_install`; :112,162 `mods_has`/`mods_remove` (mods_page).

### glass_home_lists.rs — cost ≈ 2
:6 `packs::Kind as PackKind`; :157 `storage::format_bytes`.

### glass_settings.rs — cost ≈ 1 + 3
:200 `glass_storage::count_now`; :622 `self.glass_account(cx)` (glass_account.rs:12); :672-674 `no_ai_row`/`agents_section` (agents_page.rs:22,48); :724 `self.glass_storage(cx)` (glass_storage.rs:163); :521 `mods_auto_update_row` (calls native auto-update settings).

### glass_home_tabs.rs — cost ≈ 1 + 3
:6 `packs::Kind`; :89 `glass_instance_mods` (glass_home_mods.rs:16); :94,102 `glass_packs`/`glass_screenshots` (glass_home_packs.rs:14,208).

### home_tabs.rs — cost ≈ 5 method sites
:50-51 `has_mods_tab`/`instance_mods_count` + :108-110 `instance_mods_tab` (instance_mods.rs:286,418,476); :59-61,118 `screenshots_count`/`screenshots_section` (screenshots_page.rs:110,120); :55-57,117 `pack_counts`/`packs_section` (packs_page.rs:100,109).

### blocks_play.rs — cost ≈ 4
:198,286 `microsoft::sign_in_button`; :282 `account::NEEDS_OWNER`; :295 `account::client_id`; :305 "Accounts…" button.

### other kept files, cost 1 each
game_log.rs:35 `account::Accounts::load` (name scrubbing; :127 ureq mclo.gs upload is its own network feature); multiplayer_page.rs:57 `telemetry::server_players`; move_window.rs:17 `relocation::{Progress,Step,Worker}`; downloads.rs:312 `proxy::spec_for`; datapacks.rs:5 `packs::{entry,plain}` + :73,260 `horizon_port_pack::zip_intake`; glass_server_dialog.rs:7 `glass_instance::{choice,choice_text,list_box,list_choice,section,tick}`; process/leases.rs:3-4 `native_cache` + `run_leases` types (kept with those); process.rs:20 comment rust_toolchain; data_location/tests.rs:1136 `native_mods::log_path()`; instances/switch.rs (above); mojang_tests.rs:357 platform `prepare::java` (fine, kept).

## 3. Shared symbols that must survive (live in DELETE modules, used by KEEP code)

| Symbol | Lives in | Used by (kept) | Suggested home |
|---|---|---|---|
| `jars::Permission<'_>`, `jars::prepare_launch` | jars.rs:712 area (servers.rs:712,728) | servers.rs run_managed (hosting) | Move the two into servers.rs (or keep a 50-line jars_core); hosting needs jar consent |
| `run_leases::{is_horizon, mark, snapshot, game_snapshot, shared, spawn, publish, installation, game_units, game_units_in, Claim, GAME, SOURCE}` + `control::startup_cleanup` | run_leases.rs (private mod) | backend.rs:3089-4463, servers.rs:742-754, refusals.rs:13-19, windows_runtime.rs:7-375, process/leases.rs:4, instances.rs:652, backend/{installation,play_log} | **Keep the whole module** (make it `pub`); it distinguishes Horizon's game from Java Edition and guards cache deletion |
| `nightly::{label, day, published, published_at, date_of, same_day_time, version_of}` (tag formatting) | nightly.rs | glass_versions.rs:16,39,65,68,327,689,703; blocks_versions.rs:90; blocks_chrome.rs:427; home.rs:39,362; widgets.rs:176; instances.rs:326,340; backend.rs many | Move formatting fns into backend.rs (or a new kept `release_tags.rs`); delete install/update half (`install_stable`, `after_update`, `note_published` needs a home in backend) |
| `build_import::{about, label, is_imported}` | build_import.rs | glass_versions.rs:245-264,688,704; blocks_versions.rs:89,297; blocks_chrome.rs:428; home.rs:40,363; widgets.rs:177; launcher_play.rs:171; backend/update_selection.rs:18 | School edition has no imported builds: delete callers' label branches, not move |
| `native_mods::{is_native, about/About, Profile, log_path, INSTALLED, cache_error}` | native_mods.rs:80,317,326,479,511,839-851 | backend.rs (12 sites), instances.rs:183-414,652, blocks_versions.rs:86,290, glass_versions.rs:238-700, launcher_setup.rs:606-615, servers.rs:755, backend/installation.rs, data_location/tests.rs:1136, instances/switch.rs:122 | This is *game version* metadata, not modding: extract into a small kept module (e.g. `native_meta.rs`); `cache_error` formats storage errors → move to native_cache; or hardcode `is_native == true` (school = only native builds) |
| `native_cache::{Error, linked, checked, locked, LockMode, PROFILES}` | native_cache.rs | backend.rs:1375,4418-4426,4616,4666; refusals.rs:139,428; process/leases.rs:3; windows_runtime.rs:76; cache_cleanup | **Keep native_cache** despite its name (it is the runtime-lease/cache layer); rename if desired |
| `storage::{format_bytes, Report, Category, eviction::{request_cleanup, cleanup_after_start}}` | storage.rs, storage/eviction.rs | format_bytes: blocks_settings.rs:345, glass_home_lists.rs:157; eviction: backend.rs:3186, instances.rs:472, launcher_setup.rs:8; Report/Category: main.rs:316-327, backend/messages.rs:6 | Move `format_bytes` to backend.rs (or worlds.rs); decide on eviction: either keep `storage/eviction.rs`+`storage/inventory.rs` (≈600 lines) or stub both calls to no-ops |
| `packs::{Kind, entry, plain}` + `horizon_port_pack::zip_intake` | packs.rs:23… | datapacks.rs:5,73,260; glass_home_tabs.rs:6; glass_home_lists packs rows | Keep `packs.rs` reduced to Kind + the zip-listing helpers (~80 lines) or move them into datapacks.rs; keeps horizon-port-pack dep (zip_intake) |
| `instances_page::{reload_instances, selected_instance, select_instance, open_instance_sheet, open_instance_dialog, instance_dialog, instance_escape, initial, java_line, Instances}` | instances_page.rs:93-620 | glass_home.rs, frame.rs, home.rs, launcher_setup.rs, glass_palette.rs, launcher_render.rs | The instance-selection core must move into a kept file (instances.rs or a new home.rs block) before instances_page dies; the dialog/sheet parts just die |
| `new_instance::NewKind` | new_instance.rs | glass_home.rs:225-227, launcher_setup.rs:343-345 | Dies with the New instance dialog; delete the env-preview code |
| `switch_page::{who_plays, players, play_as}` + `microsoft::sign_in_button/logo` | switch_page.rs, microsoft.rs | blocks_chrome.rs:84, frame.rs:1013-1268, blocks_play.rs:198,286 | School is account-free: remove the account chip + sign-in buttons (rewrite `who_plays` as a fixed label if the chip stays) |
| `servers_page::Field` | servers_page.rs:48 (KEPT module) | multiplayer_page.rs:8,122-127,152,169-170; console_page.rs:4,247,282,525 | Nothing to do — survives with servers_page |
| `jar_launch::{client, consent, with_consent}` | jar_launch.rs | developer_play.rs:113, launcher_messages.rs:154, servers_page.rs:31 | Java Edition client start dies; `with_consent` wraps the servers-page jar consent — replace with a direct `jars` call once jars moves into servers.rs |
| `account::{GameSession, session_for_game, take_unsaved, discard_unread_sessions, Launch, NoOwner, Accounts, session}` | account.rs | backend.rs:2141-2278, launcher_play.rs:479-582, launcher_messages.rs, developer_play.rs, game_log.rs:35, heads.rs | School plays offline: replace with a stub `GameSession` (offline name/env, `dev:` style) or keep account reduced to its offline session-file writer; backend's `start_client` needs *something* |
| `telemetry::{stats_url, PING, PING_NAME, set_official_server, server_players, send_*}` | telemetry.rs | home.rs:52,190-195, glass_home.rs:335, multiplayer_page.rs:57, launcher_setup.rs:366-368, launcher_messages.rs:64, backend.rs:3091 | Delete all call sites (Home "N playing now" row, ping setting, multiplayer count) |
| `storage::eviction` + `cache_cleanup::{delete_unleased_game_unit, delete_game_unit}` | cache_cleanup.rs | backend.rs:1378 (version delete path), backend tests :4609-4665 | Keep cache_cleanup (recommended) — it guards deleting a version a Java game still uses; else inline a simplified delete into backend |
| `horizon_telemetry::{server_status::OFFICIAL_ADDRESS/PREVIOUS, consent::Consent::path}` | crate horizon-telemetry | server_list.rs:17-19 (official SMP entry), instances.rs:796-798 (consent file move) | Inline the two constants into server_list.rs; for consent, keep the dep or drop the move |
| `horizon_friends::invite::parse` | crate horizon-friends | backend.rs:3402-3405, glass_home.rs:1140, home.rs:434, blocks_versions.rs:18 | Product decision: if Join-a-friend dies, remove the four sites + the friend_code field; else keep dep |
| `glass_instance::{choice, choice_text, list_box, list_choice, section, tick, COLOURS}` | glass_instance.rs | glass_server_dialog.rs:7 (kept), frame.rs:181 | Move these small widget helpers into glass_ui.rs (or glass_server_dialog.rs); COLOURS only if instance rows keep colours |
| `proxy::{agent, spec_for, announce, refuse, spec}` | proxy.rs | downloads.rs:312, backend.rs:471, heads.rs:102, blocks_settings.rs:511-588, startup.rs:100 | **Keep proxy** (it is generic HTTP proxy config, not update machinery); if deleted, inline a plain ureq agent in backend.rs |
| `relocation::{Worker, Progress, Step}` | relocation.rs | move_window.rs:17 (kept window), startup.rs:78 | Keep relocation + move_window (data-folder move is part of data_location), or delete both with startup.rs:78-85 and the Settings > Data row |
| `storage::eviction` note | — | glass_settings.rs:200 `glass_storage::count_now` | Replace the storage dialog with a plain folder-size row or delete the Settings section |

## 4. Launcher struct fields (main.rs:254-638), grouped

- **Club-essential (keep)**: theme 255, page 256, behind 257, switches 258, content_scroll 261, nav_strip 263, settings_pane 265, settings_query* 267-269, whats_new_open 271, settings_marked 273, long_list 281, opening 282, releases 306, installed 307, installing 308, removing 312, disk 315, bar 305, disk_for 341, install_error 342, rooms 380, notes_open 388, notes 391, technical_open 393, picker 394, accent_strip 396, look_sliders 398, hue_area 401, hue_dragging 403, language_menu 405, row_menu 409, rail 411, look 415, menu_closed_at 418, restyle 434, background_error 436, settings 441, settings_save_error 443, links 445, preparing 446, prepare_log 447, log_revision 450, prepare_error 451, prepare_fix 453, client 454, client_error 455, client_started 457, client_home 459, crash 460, servers 462, palette 467, toast_seen 469, root_focus 470, starting_client 492, starting_instance 494, pre_launch_stop 497, play_when_prepared 499, window_width 506, window_long_side 509, scale_factor 510, quick_play 513, client_run 515, client_vars 517 (launch::Vars), args_* 519-521, tx/rx 534-535, _appearance 536, _activation 538, perf 540, install_started 541, prepare_started 542, prepare_tag 544, download_seen 546, home_tab 548, home_menu 550, folder_hovered 552, list_hover 555, list_menu 557, home_strip 560, client_since 562, play_pressed 569, play_step 571, bench 573, worlds 603, multiplayer 604, pictures 608, world_packs 612, console 618, aside 620, side_servers 624, sky 626, scene 629, rain 631, backdrop 633, pushed 635, scale 637.
- **Hosting**: servers 462, side_servers 624 (kept) — plus **internet 464 (DELETE)**.
- **Accounts/identity (DELETE)**: account_menu 407, accounts 472, auth_status 473, auth_progress 474, profile_heads 475, auth_checked 477, sign_in_target 478, auth_busy 479, auth_next 480, sign_in 481, sign_in_state 482, keychain 484, owner 487, owner_screen_open 489.
- **Mods/native (DELETE)**: version_menu 275, sheet_size 277, mods_sticky 279, native_consent_build 340, importing_build 345, import_error 346, native_step 349, native_stopping 352, native_prepare_first 355, native_progress 357, first_build 359, play_after_build 362, native_done 364, native_error 365, native_failed_mods 368, native_worlds 371, pending_consent 372, consent_error 374, native_for 376, native_built_mods 378, native_builds 383, native_log 386, instance_mods 564, mod_preview 566, project_icons 568, mods 598, github 599, game_packs 609 (packs page — DELETE), shader_answers 611.
- **Modpacks/packs (DELETE)**: packs 576, registry_packs 578, packs_loaded 579, packs_reading 581, pack_open 582, pack_filter 583, pack_busy 584, pack_log 585, pack_error 586, found_packs 589, found_scanning 590, starting_pack 592, starting_work 594 (shared with instance start; keep if instances start), porting 596.
- **Telemetry/shop/partner (DELETE)**: wardrobe 420, wardrobe_marks 422, wardrobe_reload 424, shop 426, shop_status 429, shop_kept 432, collection 527, agents 529, playing 531, official_players 533.
- **Self-update (DELETE)**: updates 283, checked 284, update_error 285, self_update 286, self_update_hidden 287, min_restart_tried 290, update_install 292, instance_install 294 (instance-driven installs may survive in reduced form), install_next 297, stable_install 300, stable_failed 303.
- **Storage UI (DELETE)**: storage_report 316 … storage_revoke_error 337 (11 fields, main.rs:316-337) + storage_trims 323 (glass_storage::Trims).
- **Instance-mgmt UI (DELETE)**: instances 601, sheet_tab 413.
- **Location (DELETE)**: location_target 438, location_error 439, location_note 440.
- **Screenshots (DELETE)**: screenshots 605, shot_blur 606 (keep if picture_cache keeps Look::Blurred).
- **Import/export/launch pages (DELETE)**: export 613, launch 614, memory_slider 616, importing 617, shortcuts 622.
- **Friend join (decide)**: friend_code 502, friend_focused 503, friend_focus 504, joining 511.
- **Proxy settings UI (keep with proxy)**: proxy_input 523, proxy_focused 524, proxy_focus 525.

## 5. Overlays and message handling (launcher_render.rs)

Render-time overlays, with their gate, methods and Msg handlers; GOES = removed in school edition.

| Overlay | render site | Methods (file) | Msg/keys | Verdict |
|---|---|---|---|---|
| Import drop target/note | :330-331 | `import_drop_target`, `import_note` (import_page.rs:108,120) | — | GOES |
| Instance dialog | :332-334 | `instance_dialog`/`instance_escape` (instances_page.rs:620,401; glass_instance.rs draws), `pick_kind` (new_instance.rs:169) | — | GOES (with instance mgmt) |
| Server create dialog | :325-329 | `create_dialog` (servers_page.rs:3029) | — | KEEP |
| Settings dialog (glass) | :335-337 | `settings_dialog` (glass_settings.rs:418), `settings_escape` | — | KEEP (trim rows, §2) |
| Native build log panel | :338-340 | `native_log_panel/escape/read` (native_log_page.rs:125-180) | — | GOES |
| Screenshot viewer | :341-345 | `screenshot_viewer/escape/step` (screenshots_page.rs:57-195); keys at :239-241 | — | GOES |
| Toast float | :346-348 | `toast_float` (glass_toast) | — | KEEP |
| Palette (Ctrl/Cmd+K) | :349-351, capture at :248-260 | `palette_toggle/overlay` (glass_palette.rs:50,182) | — | KEEP, trim commands (§7) |
| Native consent dialog | :352-354, Escape at :216-220 | `native_consent`/`cancel_native_consent`/`request_native_build_consent` (blocks_settings.rs:307-334, KEPT) but `PendingConsent` from native_consent.rs; Msg::NativeRegistryConsent handler launcher_messages.rs:316-318 (`native_registry_consent::show`) | Msg::NativeRegistryConsent | GOES if native mods are pre-allowed; else move `PendingConsent` into blocks_settings |
| Welcome | :355-356, Escape :233 | `welcoming/welcome_escape/welcome_screen` (welcome.rs:12-26) | — | GOES |
| Data consent | :357-358, Escape :234 | `asking_consent/consent_escape/consent_screen` (collection.rs:138-177) | — | GOES (telemetry) |
| Owner gate | :360-362 | `asking_owner/owner_screen` (owner_page.rs:10,39) | Msg::Owner, Msg::Session, Msg::Keychain | GOES (accounts) |
| Opening logo | :363-416 | `opening_phase/skip_opening` (launcher_render.rs:445-486) | — | KEEP |
| Row menu (glass) | :164-168, :324 | `row_menu` + `frame::RowMenu`; arms open instance sheet (frame.rs:1352-1389) | — | KEEP, drop sheet arms |

Msg variants (backend/messages.rs) that GOES: Storage :6; StableRetry :12; LauncherProgress :26 / LauncherUpdated :27 (self-update); Keychain :32, Owner :34, AccountStatus :36, AccountProgress :37, AccountHead :38, AccountRemoved :39, Session :42 (accounts); JarConsent :43-46, JarClient :49-52 (Java client; keep JarConsent if hosting keeps jar consent); Playing :4, OfficialPlayers :5 (telemetry watchers, declared :4-5); PackLine :54, PackDone :56, ImportedPacks :58, FoundPacks :60; NativeProgress :62, NativeRegistryConsent :63, Native :68; Imported :70 (build import). Handlers to remove in launcher_messages.rs: :9-14 (shader answers), :19 (wardrobe), :39-47/:203 (nightly notes — keep `note_published` data part if Versions shows dates), :53-60 StableRetry, :64 (telemetry), :70-113 (accounts), :114-115, :116-180 (JavaGame/JarClient), :154, :276, :291-299, :305-314 update install (keep game-version updates? decide), :316-371, :384-390, :391-402, :483, :552-554, :556-575 (sign-in watcher).

## 6. Startup and shutdown hooks

startup.rs `run_launcher`: :57 update_handoff::take_min_restarted_env; :58 platform holder retain (keep); :59 native_mods::internals_rustc_wrapper (GOES); :62-71 console::attach_for_cli (GOES with cli); :75-77 installer::run_quiet (GOES); :78-85 relocation::Worker + move_window (decide, §3); :89 data_location::startup (keep); :92-94 update_handoff::run (GOES); :96-97 remove_replaced_binaries/cleanup_launcher_updates (backend, self-update — GOES); :100 proxy::announce (keep if proxy stays); :105-107 installer::window_mode / setup_args::Mode (GOES; drop the type too); :108-112 cli::run (GOES); :114 pictures::probe_gpu_limit (keep); :133-137 setup_page::open (GOES).

launcher_setup.rs `Launcher::new`: listed in §2 (game_shader_api :7, storage eviction :8, partner timer :28-50, update_handoff :108-111, location_page :182-183, internet :208, profile_heads :215, collection/agents :239-240, deleted pages :279-297, account keychain thread :316-320, load_profile_heads :327, mods_auto_update :330, watch_shop :332, new-instance/glass_sheet previews :340-355, wardrobe_preview :356-363, mod_preview_from_env :365, telemetry :366-368).

shut_down (main.rs:644-650): :645 servers_page::stop_on_quit (KEEP); :646 telemetry::send_before_exit (GOES); :648 update_handoff::on_close (GOES).

## 7. Pages and tabs wiring

- `enum Page` main.rs:131-143 (drop Mods/Wardrobe/Shop variants); `Page::from_env` :151-167 (mods/wardrobe/shop arms + horizon_cosmetics::forced); `Page::ALL` :171-179 (7 → 4 entries); `in_top_bar` :183-189 (drop wardrobe param).
- Render dispatch: launcher_render.rs:68-76 (drop Mods/Wardrobe/Shop arms); :73-79 wardrobe/shop pages + free_replaced; :182-189 Page::Mods scroll sticky.
- `home.rs:313` go(Page::Mods) (Mods button on Home).
- `frame.rs:280` Page::Mods && mods.on_modpacks; :996 in_top_bar(self.wardrobe_shown()).
- Bench pages list: launcher_messages.rs:17-21 (Page::ALL filter).
- Palette commands: glass_palette.rs:88-96 (pages), :97-101 NewInstance, :109-127 + :158-163 mod install/remove; `Run` enum :32-40.
- HomeTab: home_tabs.rs:7-18 (drop Mods/Screenshots; Packs/Shaders if packs die too), from_env :24-34, `home_tab_list` :42-67, section dispatch :106-120; glass strip: glass_home_tabs.rs:19-40 (icons), :88-104 content dispatch.
- Rail (glass sidebar): frame.rs `glass_rail` (:156, :670-705): instance rows (instance_icons :157,186; instances_page::initial :209), sponsor card :682-685, rail update card :870-975 (`self_update`); links row :707-771 (discord/sponsor/roadmap/docs/website); account chip :1013-1268.
- Blocky sidebar: blocks_chrome.rs:133-419 (account chip :73-84; partner card :319-326).
- Partner ad card: also servers_page.rs:1416-1423, glass_servers.rs:824-846; timer launcher_setup.rs:28-50; setting backend/settings.rs:129.
- Update card (game version): frame.rs:238 (updates.horizon), launcher_setup.rs:469-604 (check_updates/update_launcher/hand_over/follow_min_launcher/install_update); Stable install launcher_play.rs:87.
- New instance entry points: glass_home.rs:223-227; frame.rs row menu; palette NewInstance.

## 8. Cargo.toml dependencies (launcher/Cargo.toml)

| Dependency | Users today | After DELETE list |
|---|---|---|
| horizon-addresses | installer, modpacks, native_mods, partner, wardrobe | **drop** |
| object | native_mods/library_record.rs only | **drop** |
| syn (main dep) | native_mods* | drop main dep; **keep dev-dep** (i18n/tests.rs) |
| proc-macro2 | native_mods/patch.rs + i18n/tests.rs | keep (dev tests use it); main-dep use dies |
| quote | no `quote::`/`quote!`/`format_ident` hits found (comment at Cargo.toml:24 stale) | **drop** (UNVERIFIED beyond these searches) |
| sha1 | browser, downloads, heads, sources | **keep** (downloads/heads/sources) |
| ureq | account, backend, browser, download_retry, game_log, github, heads, proxy, servers, tailscale, wardrobe | **keep** (backend, download_retry, game_log, heads, servers) |
| reqwest, tokio, futures-util, httpdate | downloads.rs (+tests) | keep |
| sha2 | backend, data_location, windows_runtime (+deleted) | keep |
| chacha20poly1305 | account.rs only | **drop** |
| pulldown-cmark | backend/release_notes.rs, markdown.rs | keep |
| flate2 | backend, worlds (+deleted) | keep |
| tar | backend (+deleted) | keep |
| toml | native_mods*, packs, **servers.rs** | keep (servers server.toml) |
| semver | native_mods* only | **drop** |
| zip | backend, worlds, datapacks tests (+deleted) | keep |
| trash | desktop_seams.rs | keep |
| base64 | heads, server_list (+deleted) | keep |
| chrono | glass_home, worlds, worlds_page (+deleted) | keep |
| getrandom | backend, downloads, mac_update | keep |
| image | glass_shot_blur, heads, picture_cache, pictures, themes (+deleted) | keep |
| png | pictures.rs | keep |
| cap-std | process/*, run_leases (+deleted) | keep (process, run_leases) |
| cap-fs-ext | native_cache, storage, cache_cleanup | keep iff native_cache+cache_cleanup stay (else drop) |
| clap / clap_complete | cli, cli_tools, native_mods, storage | **drop** |
| tempfile | many kept (backend, console_page, data_location, refusals, servers_page, windows_runtime) | keep |
| dirs | kept: backend, console_page, data_location, desktop, game_log, launcher_setup, shortcut, sources, widgets | keep |
| sys-locale | i18n.rs | keep |
| horizon-server-launch | servers.rs | keep (hosting) |
| horizon-theme, horizon-launcher-scenery, gpui, gpui_platform, serde, serde_json, glam | shell/render/data | keep |
| horizon-platform | features at Cargo.toml:55 | keep `prepare`, `protocol`, `world`, `mod-package` (implied), **`mod-runtime-process`** (runtime_leases: run_leases/process/native_cache/servers/windows_runtime/backend + startup.rs:58 holder); **drop `mod-build`** (native_mods codegen), **`mod-index`**, **`mod-prebuilt`** (browser/native_mods). `server-files` not currently enabled |
| horizon-cosmetics | main.rs:155-156, wardrobe* | **drop** (after Page::from_env edit) |
| horizon-mcp | game_tools.rs, mcp.rs | **drop** |
| horizon-friends | backend.rs:3402, blocks_versions.rs:18, glass_home.rs:1140, home.rs:434 | decide (§3) |
| horizon-port-pack | datapacks.rs:73,260, main.rs, proxy.rs + deleted mods stack | keep iff datapacks keeps zip_intake (or drop by inlining zip listing) |
| horizon-shader-api | browser, game_shader_api, packs | **drop** |
| horizon-release-sign | backend.rs, backend/release_selection, update_selection, nightly, signed, update_handoff | keep iff release downloads stay (they must: installs come from releases); native_mods users die |
| horizon-telemetry | telemetry, bug_report, cli, collection, **instances.rs:796, server_list.rs:17** | drop after inlining the two (§3) |
| winreg | installer.rs, private_window.rs | **drop** |
| keyring / oo7 / security-framework (target-specific) | account.rs | **drop** with account |
| windows/windows-sys/winresource/libc/objc2-*, wgpu (linux), raw-window-handle, futures-lite | shell/platform | keep |

## 9. Tests and other files

- `launcher/tests/`: DELETE owner_gate.rs (account gate), storage.rs + storage_consent.rs (storage/native consent), developer_install.rs, modpack_headline.rs, logs.rs (drives `horizon-launcher logs` CLI); REVIEW startup.rs (entry-stack test — keep, it tests startup.rs), desktop.rs (desktop menu — keep), hidden_processes.rs (process — keep), durable_publication.rs (native package publication boundary — likely GOES, UNVERIFIED detail). fixtures/loaders (loaders tests — GOES), support/ (console_probe).
- `launcher/docs/`: png/webm screenshots referenced by README.md (play.png, servers-tailscale.png, modpacks.png…) — remove the ones for deleted pages when README is retrimmed.
- `docs/launcher-commands.md`: generated from cli.rs by `cli_tools::markdown()` (test in cli.rs; regen via HORIZON_WRITE_DOCS=1) — delete with cli.
- Root: `.github/workflows/launcher.yml` (build + installer assets — needs setup/installer decisions), `.github/workflows/ci.yml` (runs launcher checks), `tools/launcher_mcp_e2e.py` (MCP — GOES), `tools/mod_browser_harness.py` (GOES), `tools/test_launcher_macos_update.py` (GOES with self-update), `tools/lab.py` (HORIZON_LAUNCHER_HOME tmp dirs — keep), `tools/lang_coverage.py` + `launcher/lang/*.json` (i18n keys of deleted features become unused — the i18n test only checks used keys), `tools/lang_review/launcher/*.json`, root `README.md`/website build reads `launcher/links.json` (Links stays, but its discord/sponsor/roadmap/docs/min_launcher keys may go), `.githooks`/`tools/check_public_tree.py` unaffected.
- `scenery/` (own crate): untouched.
- `launcher/build.rs`: icon + stack size only — keep.

## 10. Proposed deletion order (each step aims to compile)

1. **Leaf pages nobody references**: screenshots_page, export_page, import_page, shortcut_page, location_page, launch_page, modpacks_page, mods_project, glass_home_mods, glass_home_packs, glass_mod_states, glass_account, switch_page, welcome, collection, agents_page, owner_page, wardrobe_page(+dir), glass_storage, glass_sheet, glass_instance, glass_instance_java, new_instance, instances_page, instances_mrpack, mods_page, github_mods_page, packs_page, instance_mods, instance_icons, mod_icons, internet, native_log_page, native_consent, native_registry_consent, native_build, build_mode, jar_launch, partner, cli_tools, game_tools, mcp, agents, bug_report, shop(+dir), wardrobe(+dir), tailscale, screenshots, export(+dir), import(+dir), other_launchers, jars(+dir+tests). Touch: main.rs (mod list, Page enum/ALL, fields §4, use lines), launcher_setup.rs, launcher_render.rs (§5 overlays), launcher_messages.rs (handlers), frame.rs, blocks_chrome.rs, home(_tabs).rs, glass_home*.rs, servers_page.rs, glass_servers.rs (internet/partner), glass_settings.rs, blocks_settings.rs (row groups), glass_palette.rs. First move §3 symbols out of instances_page (selection core) and glass_instance (dialog helpers for glass_server_dialog).
2. **Mods/native stack**: native_mods(+dir), native_status, native_fit, rust_toolchain, minecraft_versions, loaders(+dir), java, java_launch, game_shader_api, github_mods, github, browser(+dir), modpacks, packs (keep Kind/zip helpers → datapacks, §3), datapacks trim, build_import. Touch: backend.rs (version/prepare paths), instances.rs, glass_versions.rs, blocks_versions.rs, blocks_chrome.rs, home.rs, widgets.rs, launcher_play.rs, startup.rs:59, data_location/tests.rs. Extract `native_mods::{is_native,about,Profile,log_path,INSTALLED,cache_error}` per §3.
3. **Accounts**: account(+dir), microsoft, private_window, profile_heads, developer_offline. Touch: backend.rs:2141-2278 (offline session stub), launcher_play.rs, launcher_messages.rs, blocks_settings.rs:593-877, blocks_play.rs, game_log.rs:35, heads.rs, developer_play.rs(+backend/developer_play.rs), main.rs fields. Decide the offline session shape first (§3).
4. **Telemetry**: telemetry, collection (if not in step 1). Touch: home.rs, glass_home.rs, multiplayer_page.rs, launcher_setup.rs:366-368, launcher_messages.rs:64,114-115, main.rs:646, backend.rs:3091; inline server_list.rs:17-19 constants; instances.rs:796 consent move; drop horizon-telemetry dep.
5. **Updates/install machinery**: nightly (after moving tag fns, §3), signed, min_launcher, update_handoff, installer, setup_page, setup_args (simplify startup), relocation+move_window (decide), developer_install, proxy (decide: keep recommended), uninstall_aside, mac_update/update_recovery (with backend self-update removal). Touch: startup.rs (§6), launcher_setup.rs:469-604, blocks_settings.rs:76, backend.rs (github/signed/release paths kept for downloads! only self-update goes), frame.rs:870-975.
6. **Storage**: storage(+dir), glass_storage (if not yet), cache_cleanup (decide; recommended keep). Touch: backend.rs:3186, instances.rs:472, launcher_setup.rs:8, blocks_settings.rs:345, glass_home_lists.rs:157 (format_bytes → backend), main.rs storage fields. Keep eviction+inventory if versions can be shared with Java installs.
7. **CLI**: cli(+dir+cli_catalog_tests+cli_native_permission), console.rs (startup.rs:62). Touch: startup.rs:108-112. Delete docs/launcher-commands.md, tools/launcher_mcp_e2e.py.
8. **Instance reduction**: trim instances.rs to one fixed instance (drop Kind::Port/Pack/java paths, loaders field, browser images); trim `instance_install`/`install_next` fields. Touch: instances.rs, launcher_setup, frame rail.
9. **Cargo + platform sweep**: drop deps per §8 (horizon-addresses, object, semver, quote, chacha20poly1305, clap, clap_complete, horizon-{cosmetics,mcp,shader-api,telemetry}, winreg, keyring/oo7/security-framework, syn main dep), set horizon-platform features to `["prepare","protocol","world","mod-runtime-process"]`.
10. **Docs/CI**: launcher/docs images, README, workflows, tools (§9); lang key cleanup.

Steps 1-2 are the big ones; split step 1 into 1a (pages with only main.rs field refs) and 1b (pages with method callers) if needed.

## 11. Coupling risks (top 10)

1. **backend.rs is everything's client** (101 refs). Its prepare/install path mixes java/loaders/minecraft_versions (Java instances), native_mods version metadata, storage eviction and self-update. Cut in the order of §10.2/.5/.6, keeping `releases/download/install` (the school edition still installs Horizon builds from GitHub releases: github.rs+signed are DELETE-listed but backend.rs:514-1315 needs them — **reclassify github/signed as KEEP-adapt**, or accept losing release download).
2. **instances_page holds the instance-selection core** (`reload_instances`/`selected_instance`/`select_instance`, instances_page.rs:93-142) used by Home, Play, frame, palette. Move before deleting (§3), or Home cannot compile.
3. **run_leases / native_cache / cache_cleanup are listed under native_* / storage but are the install-cache lease layer** used by kept backend/servers/refusals/windows_runtime/process (backend.rs:4398-4463, servers.rs:742-754). Keeping them also keeps `mod-runtime-process` + cap-std/cap-fs-ext. Deleting them means rewriting cache-safe version deletion.
4. **Account removal is not mechanical**: backend's client start consumes `account::Launch/GameSession` (backend.rs:2141-2278); decide the offline-session replacement (a stub session file or `developer_offline`-style env) before step 3, or Play breaks. The owner gate, keychain thread (launcher_setup.rs:316-320) and `Widgets` account badge follow.
5. **Hosting keeps jar consent**: servers.rs:712-728 calls `jars::prepare_launch` with `jars::Permission`; jars is DELETE. Move those two items into servers.rs and keep `Msg::JarConsent` + `jar_launch::consent`-equivalent, or drop Java-server consent entirely (school: Horizon servers only?).
6. **`native_mods::about/is_native` is version metadata, not mods** (native_mods.rs:317,839): every Versions/Home label and backend's `prepared_tag`/base-version logic reads it (backend.rs:1424,1816,3361; glass_versions.rs:238,680). Extract ~100 lines or hardcode "always native" — but instance `native_profile` (instances.rs:183-289) serializes `native_mods::Profile` into instances.json (on-disk format!).
7. **Proxy is DELETE-listed but is the HTTP layer** for downloads/backend/heads (downloads.rs:312, backend.rs:471). Keep it (recommended) or every fetch site needs a plain agent.
8. **storage::eviction + cache_cleanup guard concurrent Java installs**; school edition with one fixed install could drop them, but backend.rs:3186/1378 and instances.rs:472 call them on every install/delete. Decide keep-vs-stub before step 6.
9. **glass_server_dialog.rs:7 imports widget helpers from glass_instance** (choice/list_box/section/tick/COLOURS) — move them first or the kept server dialog breaks. Same pattern: frame.rs instance rows use glass_instance::COLOURS + instance_icons.
10. **horizon-friends and telemetry constants cross crate boundaries** (backend.rs:3402 join code; server_list.rs:17 official SMP address from horizon-telemetry). Both are product decisions hiding in code: friend-join removal touches Home's Play form; the official server entry in the Multiplayer tab needs the address inlined or the feature dropped.

## 12. Journal

### Step 1 (2026-11-22): the tools and services layer

`cargo check` and `cargo check --all-targets` both green, no warnings. 69 files changed
under `launcher/` + `docs/`: +112 / −17,675 lines (git diff --stat).

**Modules deleted** (with their tests):
- CLI/tools: `cli.rs` + `cli/` (developer_builds, progress, settings_output, storage_tests),
  `cli_catalog_tests.rs`, `cli_native_permission.rs`, `cli_tools.rs`, `mcp.rs`, `game_tools.rs`,
  `agents.rs`, `agents_page.rs`, `console.rs` (only `attach_for_cli` lived there),
  `bug_report.rs` (its only caller was the CLI `bug` command; took the last
  `horizon_telemetry::scrub` use with it), `perf::benchmark` + its private helpers' dead half.
- Telemetry/shop: `telemetry.rs`, `collection.rs` (data-consent overlay), `welcome.rs`,
  `partner.rs`, `shop.rs` + `shop/tests.rs`, `wardrobe.rs` + `wardrobe/tests.rs`,
  `wardrobe_page.rs` + `wardrobe_page/{marks,purchases,shop,shop/buy,shop/thumbs,viewer}.rs`.
- Tests: `tests/logs.rs`, `tests/storage.rs`, `tests/desktop.rs`, `tests/owner_gate.rs`
  (each only drove CLI/MCP subcommands; storage/desktop/owner-gate behaviour itself is later
  steps' to remove). `tests/hidden_processes.rs` lost its CLI and MCP tests; `tests/startup.rs`
  lost the telemetry-consent fixture.
- `docs/launcher-commands.md` (was generated from cli.rs).
- Cargo.toml: dropped `horizon-telemetry`, `horizon-mcp`, and — their only users died with
  this step — `clap_complete` (CLI completions) and `horizon-cosmetics` (`Page::from_env`'s
  wardrobe/shop gates). `clap` stays (native_mods, storage). Cargo.lock regenerated.

**Kept files changed**: main.rs (Page loses Wardrobe/Shop; `in_top_bar()` takes no argument;
fields wardrobe*/shop*/collection/agents/playing/official_players gone; `shut_down` no longer
sends telemetry), startup.rs (windowed only: no console attach, no `cli::run()`), lib.rs,
launcher_setup.rs (partner timer, watch_shop, wardrobe preview, telemetry watchers gone),
launcher_messages.rs (Playing/OfficialPlayers handlers, `set_official_server`), backend/messages.rs
(Playing/OfficialPlayers variants), backend.rs (`client_command` now sets `HORIZON_NO_HEARTBEAT=1`
unconditionally — a student's game never pings; `Links` loses `official_server{,_previous}`),
backend/settings.rs (ping_off, ping_notice_seen, welcomed, partner_seen_secs, partner_hidden),
home.rs + glass_home.rs ("N playing now"/"M on the official SMP" rows), multiplayer_page.rs +
glass_home_lists.rs (Reach status ping died with `telemetry::server_players`; glass server rows
show the address with no dot and Join is enabled whenever the game can start), servers_page.rs +
glass_servers.rs (partner banners; Field::ShopGift/WardrobeSearch), frame.rs + blocks_chrome.rs
(sidebar Discord/sponsor/roadmap/docs/website links, partner cards, `Rail::links_open`), console_page.rs +
game_log.rs(+tests) (mclo.gs "Share log" upload fully removed — a student must not upload anything;
game_log keeps only the `matches` search helper), server_list.rs(+tests) (add_official,
move_official, `find`, OFFICIAL_* constants, `horizon_official_server.txt` marker; the rest of
servers.dat read/write stays), owner_page.rs (no consent gate before the owner gate),
glass_settings.rs (SECTIONS 10→9: "Data & privacy" gone, AI keeps only No AI), glass_palette.rs,
instances.rs(+tests) (the telemetry consent no longer follows a player into an instance),
effects.rs/themes_page.rs (stale "first-run welcome" comments), tests/startup.rs, tests/hidden_processes.rs.

**Deliberately left for later steps**:
- `native_mods::internals_rustc_wrapper` in startup.rs (step 2).
- Self-update machinery: `Msg::StableRetry` handler, `follow_min_launcher`, `update_handoff`,
  `min_launcher`, installer/setup_page/setup_args, mac_update (steps 4–5). `Links::min_launcher`
  and the links.json fetch stay for it.
- Accounts: the owner gate, keychain thread, `owner_notice_seen` (step 3).
- `blocks_play.rs` "Report on Discord" (crash card) and the sponsor rows on
  blocks_versions.rs/glass_versions.rs: not sidebar links; they die with their areas later.
- `backend::AGENT_DOCS_URL` (pub const, now unreferenced) and `HORIZON_TELEMETRY_URL=off` on
  developer-install game commands (a plain env guard, backend.rs client/server command).
- `tools/launcher_mcp_e2e.py`, `.github/workflows/launcher.yml`, launcher/README+docs images,
  lang key cleanup (map step 10; outside `launcher/` or cosmetic).
- `launcher/links.json` untouched (the website build reads it from main; unknown keys are ignored).

**Surprises**:
- `no_ai_row` lived in agents_page.rs but is a general Settings row used by both looks: moved to
  blocks_settings.rs. Its toggle's error used to land in `agents_section`'s status line, which
  died; it is now eprintln'd.
- tests/startup.rs's `cli_settings_work_with_one_mib_process_stack` drove the CLI with a patched
  1 MiB entry stack. Replaced with a read-only PE-header check
  (`desktop_executable_keeps_a_one_mib_entry_stack`): the windowed exe can't run headless, but
  the invariant "the entry thread reserves at most the 1 MiB default" is still asserted.
- `perf::benchmark` (the `horizon-launcher benchmark` driver) was CLI-only; the in-window bench
  (`perf::Bench`, HORIZON_LAUNCHER_BENCH) stays and tests/startup.rs still uses it.

### Step 2a (2026-11-22): the pages of mods, packs and the rest

`cargo check` and `cargo check --all-targets` both green, no warnings. 37 files deleted (17,576
lines). `git diff --stat -- launcher` also counts step 1, which is not committed: 135 files,
+441 / −36,288 (step 1's journal: +112 / −17,675), so this step is about +330 / −18,610.

**Modules deleted** (with their tests):
- Mods and packs: `mods_page.rs`, `mods_project.rs`, `glass_mods.rs`, `github_mods_page.rs`,
  `modpacks_page.rs`, `packs_page.rs`, `mod_icons.rs`, `instance_mods.rs`, `glass_home_mods.rs`,
  `glass_home_packs.rs`.
- The native mods build: `build_mode.rs`, `native_build.rs`, `native_log_page.rs`,
  `native_consent.rs`, `native_registry_consent.rs`, `glass_mod_states.rs`.
- Pages with no place: `launch_page.rs`, `export_page.rs`, `import_page.rs`, `shortcut_page.rs`,
  `screenshots_page.rs`, `datapacks_page.rs`, `jar_launch.rs` (dissolved, see surprises).
- Orphaned by those: `glass_shot_blur.rs` (the screenshots' privacy blur), `sticky.rs`.
- Library: `export.rs` + `export/`, `import.rs` + `import/`, `screenshots.rs`,
  `other_launchers.rs` + `other_launchers/`, `instances_mrpack.rs` + `instances_mrpack/`,
  `datapacks.rs` + `datapacks/`, `packs.rs` + `packs/`.
- Inside kept files: `Page::Mods`; `HomeTab::{Mods, Packs, Shaders, Screenshots}` (the glass look,
  which has no Overview, falls back to Worlds); `Msg::{Imported, PackLine, PackDone, ImportedPacks,
  FoundPacks, NativeProgress, NativeRegistryConsent, Native}`; `SheetTab::Hooks`;
  `Field::{PreLaunch, Wrapper, LaunchEnv, PostExit, MemoryLimit, InstanceModsFilter}`; the §4
  fields of "Mods/native" (except `version_menu`, `sheet_size`, `native_builds`),
  "Modpacks/packs", "Screenshots" and "Import/export/launch pages"; `Settings::mods_auto_update`
  and `auto_updates_mods()`; `Crash.leases_refused`; `refusals::crash_help`; the build's whole
  state (its card, "Build and play", the Build log panel, the consent dialog); the build-import UI
  ("Import a game build…" on Versions); the palette's mod search; `browser::installed::
  {DataPackInstall, install_datapack}`; `modpacks::{found_on_this_computer, found_folder,
  found_slug, found_summary}`; the helpers `badge`, `tabs`, `quiet_note`, `markdown::render`,
  `servers_page::set_field_invalid`.
- Cargo.toml: `glam` (its one user, `wardrobe_page/shop/thumbs.rs`, went in step 1).

**Kept files changed**: main.rs, lib.rs, launcher_setup.rs, launcher_render.rs,
launcher_messages.rs, launcher_play.rs, developer_play.rs, servers_page.rs,
backend/{messages,settings}.rs, frame.rs, blocks_{chrome,play,settings,versions}.rs,
glass_versions.rs, glass_home.rs, glass_home_lists.rs, glass_home_tabs.rs, glass_settings.rs,
glass_sheet.rs, glass_instance.rs, glass_palette.rs, glass_storage.rs, home.rs, home_tabs.rs,
instances_page.rs, instance_icons.rs, location_page.rs, worlds_page.rs, picture_cache.rs,
widgets.rs, blocks.rs, markdown.rs, play_step.rs, refusals.rs, pictures.rs (its doc), browser.rs,
browser/{details,installed,horizon}.rs and their tests, modpacks.rs, lang/fr.json (4 keys).
Nothing under `launcher/tests/`: storage_consent.rs and modpack_headline.rs compile, they test
later steps' library code.
- The instance's launch settings are gone with their editor: the pre-launch command, the wrapper,
  the post-exit command, the variables and the Linux switches (`launch::Launch` itself stays, as
  instances.json has it), and the Java heap row.
- The Worlds tab loses only its data-pack button, list and drop target: `worlds.rs` had no
  data-pack code. The Multiplayer and Console tabs are as they were.
- No drag-and-drop target is left anywhere in the window.
- The blocky Home's Instance row and the glass row menu: Edit, Copy or Duplicate, Open folder,
  Delete. The instance sheet: General, Installation, Launch (a Java instance: its JVM arguments and
  window size), Storage (the space used).
- The crash card keeps Open report, Copy report, Open logs folder and Dismiss, and has no guidance
  line any more: a later step may want one that says to ask the supervisor.
- New strings, French added to `lang/fr.json`: "How the game starts.", "Space this instance
  takes.", "Play an instance, open a page", "Close the game and servers, and wait for downloads and
  sign-in to finish before moving data."

**Deliberately left for later steps**:
- `instance_icons.rs` was on this step's list and is kept whole: it draws the instance marks of the
  sidebar (frame.rs), of Home and Settings (`horizon_mark`, `grass_mark`), of the instance dialog
  and sheet. Its one tie to the deleted pages (`self.packs`, a ported pack's loader) is gone, so a
  ported pack shows Horizon's mark. It goes with the instance dialog (instance reduction).
- §4 files `version_menu` (main.rs:222) and `sheet_size` (main.rs:224) under "Mods/native": they
  are the instance dialog's and the sheet's Storage row's. Instance reduction.
- Native builds as *version metadata* are untouched, as asked: `native_mods::{is_native, about}`
  and `native_status` are read at backend.rs:1406, 1798, 3344, launcher_setup.rs:503
  (`native_build_of`; the field `native_builds`, main.rs:289), frame.rs `row_state` and
  instances_page.rs:244 ("Native build · {version}"), blocks_versions.rs:86, 233,
  glass_versions.rs:188, 213, 630, 650, glass_instance.rs:617, 643, and startup.rs:59
  (`internals_rustc_wrapper`). Step 2b.
- The native-mods *permission* setting: `Settings::native_mods_allowed_at` (backend/settings.rs:148),
  `allow_native_mods` (:179; nothing in the window calls it now), `storage::revoke_native_mods`
  (storage.rs:489) and the Storage dialog's "Native mods / Turn off" row (glass_storage.rs:329).
  Its sentences still say "before the next build" and "its instance's Installed mods"
  (glass_storage.rs:337, 360, 364, 370, 498): reword with the storage step. tests/storage_consent.rs
  tests `revoke_native_mods`: the same step.
- `build_import`: its reads are as they were; Versions still lists "Imported builds"
  (glass_versions.rs:195, blocks_versions.rs:240).
- `browser` has two callers left in the window: `browser::images::fit_png` (instance_icons.rs:7)
  and `browser::java::set_enabled` (glass_storage.rs:485). `browser::horizon::fetch_datapack`
  (browser/horizon.rs:877) has none, nor have `github_mods` and `game_shader_api`. Step 2b.
- `modpacks.rs` is still read by instances.rs; `modpacks::{list, remove, headline}` (modpacks.rs:335,
  477, 102) have no caller in the window, and tests/modpack_headline.rs tests "the Modpacks page's
  headline". Step 2b.
- `launch.rs` is kept as asked; `memory_at` (:64), `parse_memory` (:78), `memory_fraction` (:94),
  `total_memory_mb` (:99), `env_line` (:307) and `Launch::summary` (:240) are now used by
  launch.rs's own tests only.
- `shortcut.rs`: `make`, `find`, `remove` (shortcut.rs:430, 455, 462) have no caller (the CLI went
  in step 1, the toggles here); the module stays for installer.rs and game_app.rs:65 (`xml`).
- A ported modpack's or a registry pack's instance (`Kind::Port`, `Kind::Pack`) is still listed in
  both sidebars. Its row used to open its page on the Mods page; a click now selects it and opens
  Home, like any other row.
- glass_home.rs:856: the folder menu's "Open the screenshots folder" stays (it opens a folder; the
  game still saves screenshots there).
- Comments of later steps' modules that still say "the Mods page" or "native builds" are not
  rewritten: native_status.rs:3, 70, 83, 255; storage.rs:139; relocation.rs:167 ("Close games,
  servers and native builds before moving data").
- Step 1's other leftovers (`backend::AGENT_DOCS_URL`, links.json, README, workflows, tools, lang
  keys) are where its journal left them.

**Surprises**:
- `jar_launch` was not a leaf. `client` was the off-thread tail of every Play (its result is
  `Msg::JarClient`, the message that starts the game), and `with_consent`/`consent` are the two
  halves of the jar question the Servers page's hosting asks. It is dissolved: `prepare_start`
  lives in developer_play.rs (its one caller, with its test), `with_consent` and `jar_consent` in
  servers_page.rs, beside `run_managed_server`. `Msg::JarConsent` and `Msg::JarClient` stay.
- `launch_page` was not a leaf either: beside the blocky Home's Launch… panel it held the sheet's
  Hooks section, the memory row of the sheet and of the Edit dialog, and the Enter handling of
  those fields in servers_page.rs.
- `shortcut_page`'s toggles were also in frame.rs's row menu, glass_home.rs's Play menu and the
  sheet. Beside the shortcut, the Play menu's "Copy the launch command" copied
  `horizon-launcher play --instance <id>`, a subcommand step 1 removed: gone too.
- `shot_blur` was in §4 with "keep if picture_cache keeps Look::Blurred": nothing constructs
  `Look::Blurred` once the Screenshots tab is gone, so `glass_shot_blur.rs`, the field, the
  thumbnail size (`thumb_side`, `THUMB_POINTS`) and the blurred look went.
- `datapacks` and `other_launchers` had users among later steps' modules, hence the two entries
  above (`install_datapack` in browser/installed.rs; four functions and a test in modpacks.rs).
  Two browser tests lost the lines that copied a data pack into a world, or read the launcher's
  shader-pack list.
- `Msg::Native*` were only ever sent by the GUI modules deleted here (native_build.rs,
  glass_mod_states.rs): backend.rs needed no change for the build.
- No dependency could go but `glam`. `horizon-port-pack` is still used by build_import.rs,
  browser/{download,java}.rs, instances/tests.rs, jars.rs, modpacks.rs, proxy.rs and
  tests/modpack_headline.rs; `horizon-shader-api` by game_shader_api.rs and browser/horizon.rs.
  `mod-build` (native_mods.rs, native_mods/{internals_sync,patch}.rs), `mod-index` (github_mods,
  browser, native_mods, native_status, storage/inventory.rs) and `mod-prebuilt`
  (native_mods/{prebuilt,sdk_cache}.rs) are all still needed.
- `mods_auto_update` had exactly two users, the registry mods' update before Play and a Settings
  row: both deleted, so the setting is too (`Settings` is not `deny_unknown_fields`: an old
  settings.json still loads).
- Tests were compiled (`--all-targets`), not run: the rule of this step is no `cargo build`.

### Step 2b (2026-11-22): the library stack under the mods pages

`cargo check` and `cargo check --all-targets` both green, no warnings, on Windows x64;
`cargo clippy --all-targets` has no warning either. The Unix- and macOS-only modules
(`game_app.rs`, `mac_update.rs`, `process/supervisor.rs`) are not compiled on this machine: none
was edited, and rg shows none names anything this step removed. 240 files, +972 / −79,620
(`git diff --shortstat -- launcher`), which still counts steps 1 and 2a (step 2a ended at 135
files, +441 / −36,288): this step is about 105 files, +531 / −43,332.

**Modules deleted** (72 source files, about 39,600 lines, with their tests):
- `native_mods.rs` + `native_mods/` (29 files), `native_status.rs`, `native_fit.rs`,
  `rust_toolchain.rs`.
- `java.rs` + `java/`, `java_launch.rs` + `java_launch/`, `loaders.rs` + `loaders/`,
  `minecraft_versions.rs`: a Java Edition instance can no longer be made, edited or started.
- `browser.rs` + `browser/` (15 files), `modpacks.rs`, `github_mods.rs` + `github_mods/`,
  `game_shader_api.rs`, `build_import.rs`.
- `jars.rs` + `jars/launch.rs` + `jars_tests.rs`.
- In the window: `new_instance.rs` (what remains of it, the "My Horizon" name, is in
  `instances_page.rs`), `glass_instance_java.rs`, and `instances/switch.rs` (Horizon and Java in
  one instance).
- Tests and assets: `tests/modpack_headline.rs`, `tests/storage_consent.rs`,
  `tests/fixtures/loaders/` (6 files), `assets/logos/{fabric,forge,neoforge,quilt}.png`.

**Inside kept files** (the state, UI and handlers of what goes):
- `instances.rs`: `Kind::{Port, Pack}`, `JavaGame`, `Meta::{native_profile, java, java_kept}`,
  `Instance::{work, native_profile, java}`, `create_java`, `adopt_icon`, the profile's
  deserializer, `apply`'s pack branch: `Kind` is `Default | Own`. An old `instances.json` still
  loads, and its unknown fields are gone at the next save (one test). `fit_png` moved here from
  `browser/images.rs` (`instances::fit_png`, the one thing kept code took from `browser`), and
  the folder-name `slug` from `modpacks.rs`.
- `backend.rs`: `java_game`, `download_version_for_owner`, `prepared_in` (a build reads its own
  folder), the base mapping in `reuse_prepared_for`, `NotReady` and `ready` (`need_prepared` is
  the plain check), the `resource_dependents` veto of `remove` and `prune_old_versions`,
  `native_mod_architecture_error`, `issues_url`, `save_checked_until` (the stoppable download
  had no caller left), `browser::horizon::before_play` and the registry checks before Play (in
  `client_as`). `cache_cleanup::delete_unleased_game_unit` takes no veto and
  `native_cache::Error::Dependent` is gone.
- Messages: `Msg::{JavaGame, JarConsent}`. `Msg::JarClient` is `Msg::GamePrepared`, since a
  start has no jar phase; `play_step` loses "Preparing Java jars…".
- The jar consent: `jars::Permission` was the download consent for the Java jars a *mod set*
  reads (Modrinth, SHA-512), not Minecraft's EULA, so it went with mods. `servers::run_managed`
  takes no permission; `servers_page::{run_managed_server, with_consent, jar_consent}` are gone.
  The EULA itself (`backend::{agree_eula, eula_agreed, server}`) is untouched for the
  `eula_accepted` step.
- `run_leases::shared` no longer stamps the jars of enabled mods.
- Settings: `Settings::native_mods_allowed_at`, `allow_native_mods`, `revoke_native_mods`, and
  the Storage dialog's "Native mods" card (`glass_storage.rs`) with its three `Launcher` fields.
- `worlds.rs`: `World::mods`, `saved_with` and the mod lists read from level.dat, which only
  native builds asked for.
- Left without a caller by the deletions: `native_cache::{Cache::share, Cache::directory,
  waited, side_lock, checked_tree}`, `storage::eviction::{after_build, after_build_logged}`
  and `Storage::stamp_build` (the cleanup after a build), `nightly::short_commit`,
  `proxy::for_programs`, and `developer_install::uses_developer` (it is `is_developer` now).
- `cache_error` moved beside the cache (`native_cache.rs:24`, exported as
  `horizon_launcher::cache_error`). `startup.rs` no longer dispatches the rustc wrapper.
- In the window: the instance dialog and sheet lose their Java half, the Horizon / Vanilla /
  Modded tabs, Mojang's version list, the loader grid, the Launch section (JVM arguments, window
  size) and the switch notice, with `Field::{InstanceVersion, JavaArgs, WindowWidth,
  WindowHeight}`. The glass top bar shows Horizon's mark again where the edition switch was; the
  blocky Home's Java rows and Play's Java branch (`start_java`, `can_join_selected`) are gone.
  Versions loses "Native mod builds" and "Imported builds". The glass look keeps "Installed
  here" (installed builds the release list doesn't name, `glass_versions.rs:582`), and its line
  is now the Minecraft version the build plays, read from its `research-lock.json`
  (`backend::game_version`, backend.rs:1336); the blocky look shows the release list alone, as
  it already did for builds that were neither.
- `build_meta.rs` was not created. After the trim nothing asks whether a build is a native one:
  every page that branched on `native_mods::is_native` lost the branch, and the one fact a kept
  page needs from a build's folder is `backend::game_version(tag)`, which already reads
  `research-lock.json`.
- The crash card has a guidance line again: `refusals::crash_help` (refusals.rs:85, drawn at
  blocks_play.rs:529), "Tell the club's supervisor that the game crashed." French: « Prévenez le
  responsable du club que le jeu a planté. »
- Cargo.toml: dropped `object`, `semver`, `quote`, `syn`, `proc-macro2` (the last two stay as
  dev-dependencies: the translation test reads Rust tokens), `horizon-shader-api`, and the
  dev-dependencies `horizon-mod-vocab` and `horizon-mod-api-gen`. `horizon-platform` keeps
  `prepare`, `protocol`, `world` and `mod-runtime-process`: `mod-build`, `mod-index` and
  `mod-prebuilt` are gone. The lock file was regenerated offline.

**Kept files changed**: lib.rs, backend.rs, backend/{installation,settings,update_selection,
developer_play,messages}.rs, instances.rs (+tests), launch.rs (+tests), account.rs (+tests),
servers.rs, run_leases.rs (+control, tests), process.rs, process/leases.rs, storage.rs,
storage/{inventory,eviction}.rs, native_cache.rs (+tests), cache_cleanup.rs, refusals.rs,
developer_install.rs, developer_offline.rs, nightly.rs, proxy.rs, worlds.rs (+tests), startup.rs,
data_location/tests.rs, main.rs, launcher_setup.rs, launcher_messages.rs, launcher_play.rs,
developer_play.rs, play_step.rs, servers_page.rs, blocks_play.rs, blocks_versions.rs,
blocks_chrome.rs, home.rs, glass_home.rs, glass_versions.rs, glass_instance.rs, glass_sheet.rs,
glass_storage.rs, glass_palette.rs, glass_home_lists.rs, frame.rs, instances_page.rs,
instance_icons.rs, multiplayer_page.rs, Cargo.toml, Cargo.lock, lang/fr.json (1 key), shell.nix
(its comment), tests/developer_install.rs.

**Deliberately left for later steps**:
- `horizon-port-pack` stays a dependency: `proxy.rs` reads its proxy rules
  (`horizon_port_pack::net`, proxy.rs:25-76), and proxy is a later step's module.
- `horizon-addresses` stays: `installer.rs:27` (`ABOUT_URL`). `clap` stays: `storage.rs:28`
  still derives `clap::ValueEnum`, for a CLI that is gone.
- The CLI's Play in the backend, unused by the window: `client` (backend.rs:3172), `client_as`
  (backend/developer_play.rs:5), `join_friend` (backend.rs:3180), `server` (backend.rs:3189),
  `SIGN_IN_HINT` (backend.rs:3177). Two sentences still give commands step 1 removed:
  `prepare_hint` (backend.rs:3143, "`horizon-launcher prepare --version …`"; hosting shows it
  through `need_prepared`, backend.rs:3154) and `no_version_installed` (backend.rs:2986, "Run
  `horizon-launcher install latest`"). Both want the club's own sentence ("ask the supervisor").
- `mojang::java_runtime` (mojang.rs:224) and `mojang::download` (mojang.rs:52): the
  closure-fetch paths only tests call (already so before this step; `java_runtime_http` and
  `download_http` are the preparer's).
- Storage still measures native-mod build caches: `BUILD_CACHES` (storage/inventory.rs:10),
  `unused_jars` (storage/inventory.rs:864), `cache_error`'s sentences (native_cache.rs:28, 42)
  and the rows' sentences in glass_storage.rs:394-402. Two rules changed inside it because what
  they read is gone: a spare native build is no longer listed for eviction (a release one never
  was), and `Entry::revision` (storage/inventory.rs:134) reads a unit's revision without
  checking its target against the toolchain's hosts.
- `game_app.rs` (macOS) still links a version's native-mod files into the .app bundle
  (game_app.rs:85, 96) and words the microphone prompt for native mods (game_app.rs:66): not
  compiled here, so not edited.
- `developer_offline.rs:25-26`: two messages that speak of native mods and of
  `developer-install`. The instance sheet's two strings that mention mods (glass_sheet.rs:653,
  702). `launch.rs`'s Java heap helpers, its own tests only (launch.rs:64, 78, 94, 99, 307, 240).
- `process::leases::ChildLeases` (process/leases.rs:14) and `process::supervisor`
  (process/supervisor.rs:65) have had no caller since step 1.
- `heads.rs:129` (session skins) and every other accounts leftover: step 3, as in 2a's journal.
- rustfmt has one difference left, `server_list.rs:59` (a blank line from step 1).
- The root `THIRD_PARTY_NOTICES.md` still names the four loader logos, `launcher/README.md`,
  `launcher/docs/` images and the translation keys of deleted features are where map step 10
  left them. `launcher/bootstrap/` (untracked, another workstream's) was not touched.

**Surprises**:
- "No window code needs them" held for the mods half, not the Java half: a Java Edition instance
  was half of the instance dialog and sheet, the edition switch of the glass top bar, a branch of
  Play and of the palette. Removing `java`, `java_launch`, `loaders` and `minecraft_versions`
  removed Java instances from the window too.
- `new_instance.rs` was almost all Java: what is left is `free_name` and `new_instance_name`
  (instances_page.rs:13, 274).
- The mods stack reached further than backend.rs: `run_leases::shared` (mod jars),
  `worlds.rs` (the mods a world was saved with), `storage/inventory.rs` (the build caches),
  `native_cache.rs` (the build lease's side locks) and `developer_install.rs` (it checked the
  game program through `build_import::holds_game`; `is_game` is its own now).
- Kept code was compiled before the files went: every module to delete was undeclared in
  lib.rs at once, the errors in kept code were fixed, and only then were the files removed.
- Tests that compiled but pinned what was removed were fixed or deleted with it: the instance
  list's pack entries and native profile (instances/tests.rs), `select_after_update` on an
  imported build (backend.rs), the storage revision grammar (storage/inventory.rs), a ported
  pack's cache in a move (data_location/tests.rs), a developer base through imported and native
  builds (tests/developer_install.rs).
- `instances::find`'s error named a removed command ("horizon-launcher instance list shows
  them"): it is "No instance {q}." now.
- Tests were compiled (`--all-targets`) and linted, not run: the rule of this step is no
  `cargo build`.

### Step 3 (2026-11-22): no accounts, the Windows session names the player

`cargo check`, `cargo check --all-targets` and `cargo clippy --all-targets` all green, no
warnings, on Windows x64; `cargo fmt --check` is clean. Cumulative `git diff --shortstat --
launcher` (steps 1-3, uncommitted): 254 files, +1,356 / −89,294; step 2b ended at 240 files,
+972 / −79,620, so this step is about 14 paths, +384 / −9,674 (11 files deleted, 7,468 lines).

**Modules deleted** (11 files, 7,468 lines, with their tests):
- `account.rs` + `account/{session,tests}.rs`: Microsoft sign-in (device-code flow, tokens,
  XSTS, profiles), accounts.json, session files, the owner gate's logic, the sign-in secrets
  file (FileStore with chacha20poly1305), `store()`/`Layered`.
- `microsoft.rs`, `private_window.rs`, `profile_heads.rs`, `owner_page.rs` (the owner-gate
  screen, both looks), `glass_account.rs`, `switch_page.rs`, `developer_offline.rs`.
- `backend/developer_play.rs`: the CLI Play's session selection (`client_as`), replaced by
  `backend/game_start.rs`.

**New modules**:
- `identity.rs` (`launcher/src/identity.rs`): `player_name(login)` folds the Windows login to a
  Minecraft name (ASCII, letters/digits/`_`, lower-cased, first letter capitalised; past 16
  characters the first 12, `_` and three hex digits of FNV-1a-32's low 12 bits over the folded
  login; fewer than three folded characters is `IdentityError::TooShort`), and `login()`
  (`USERNAME` else `USER`, part after the last `\`). Tests pin `j.dupont`→`Jdupont`,
  `Jérôme.Dùpont`→`Jeromedupont`, the 16-character shortening (structure, case-insensitivity,
  two 17-character logins sharing 13 characters stay apart) and the published FNV-1a vectors
  ("" / "a" / "foobar").
- `keychain.rs`: the OS-keychain half of account.rs (SecretStore, MemoryStore, Bounded,
  SystemStore with keyring/oo7, pieces on Windows), extracted because `internet.rs` — a later
  step's file — keeps the Tailscale access token there. `HORIZON_NO_KEYCHAIN` is gone with the
  account tests; the `not(test)` guard keeps unit tests off the player's keychain.
- `src/game_start.rs` (renamed from `developer_play.rs`): `handle_game_start` (was
  `handle_game_session`) and `prepare_start`.

**Kept files changed**: lib.rs, main.rs (module list; `Page::from_env`'s `sign-in` preview doc;
§4 fields accounts/auth_*/profile_heads/keychain/owner_*/friend_*/joining gone, `player: Option<String>`
in; tests `play_prepares_first_for_an_owner`/`play_says_what_it_will_do` rewritten without
`owner`, `a_completed_sign_in_is_handled_once_across_ui_polls` deleted), launcher_setup.rs
(keychain thread, `go()`'s account reloads), launcher_messages.rs (Msg::Keychain/Owner/
Account*/Session handlers, the sign-in watcher, `discard_unread_sessions` calls;
`PrepareFix::Mojang` and `prepare_offers_mojang` uses), launcher_render.rs (owner-gate overlay,
friend focus), launcher_play.rs (`start_client` gates on `self.player` and says through `tr` that
the Windows session's name cannot make a player name; the thread completes the assets and sends
`Msg::AssetsDone`; the sign-in/status/head helpers gone), frame.rs + blocks_chrome.rs (the chip is
the player's name, a plain label with the user glyph, no menu, no Sign in with Microsoft),
blocks_play.rs (owner-gated rows and buttons; Prepare is always from the computer's install),
blocks_settings.rs (the Account group and `account_rows`/`microsoft_rows`/`sign_in_parts`),
glass_settings.rs (SECTIONS 9→8, renumbered), glass_home.rs + home.rs (the Join-a-friend form
and card; the card is What's new only, `Option` when there are no notes), glass_home_lists.rs
(the signed-out Join arm), blocks_versions.rs (`join_friend`), servers_page.rs (`own_name` reads
`self.player`), glass_server_dialog.rs (friend chips always the plain user glyph),
widgets.rs (`play_press(prepared, busy)`/`play_label(press)` without owner;
`account_status_line`/`sign_in_change`/`account_row` gone), location_page.rs (sign-in busy gates),
heads.rs (the session-server lookup, profile faces and cached skins gone; the default skin from
the prepared files stays), backend.rs, backend/{messages,settings}.rs (`owner_notice_seen`),
play_step.rs ("Reading the sign-in…" step), internet.rs (four sites repointed at
`keychain::keychain()`), instances.rs/shell.rs/glass_toast.rs/frame.rs doc comments, Cargo.toml
(`chacha20poly1305` and `horizon-friends` dropped), Cargo.lock (offline), lang/fr.json (5 keys),
server_list.rs (step 2b's rustfmt leftover, fixed by this step's `cargo fmt`).

**In backend.rs**: `download_for_owner`/`download_for` and `refill_own_store` are gone —
`MinecraftFrom::Mojang` and the not-found path now refuse with "This launcher has no Microsoft
sign-in, so it can't download {version} from Mojang."; `client`/`client_as`/`SIGN_IN_HINT`/
`join_friend` (CLI-only, unused since step 1) deleted; `client_command` moved to
`backend/game_start.rs`, adds `--name <player name>` from `identity` and no longer sets
`HORIZON_ACCOUNT_SESSIONS`/`HORIZON_LAUNCHER_EXE`. The `mojang_downloads_only_for_an_owner` test
died with the gate it pinned; `incomplete_minecraft_files_...` expects the new refusal sentence.

**Deliberately left for later steps**:
- `internet.rs:108,182,183,194` and `keychain.rs` (with the `keyring`/`oo7`/
  `security-framework` deps): the Tailscale token store, until the internet/tailscale step
  deletes the panel and the module with it. The map's §8 expectation that those deps go now
  did not hold: something kept still uses them.
- `data_location/tests.rs` (accounts.json as a move fixture, :84,163,178,254,480,654): the
  data-location step.
- `prepare_hint` (backend.rs:3007) and `no_version_installed` still name `horizon-launcher`
  commands step 1 removed (the account clause is trimmed); the club-admin sentence is the
  club.toml step's.
- `mojang.rs` and `sources.rs` untouched: `mojang::download_http` now has no caller the window
  can reach, and dies with the downloads step.
- `installer.rs` ("for this account"), `proxy.rs:7` (doc listing Microsoft sign-in among
  proxied traffic), `servers.rs:951` (`check_accounts`, the server's online mode — a hosting
  feature that stays), `glass_server_dialog.rs:520` comment: their own steps.
- Lang keys of the deleted features, README/docs/workflows, `tools/*` (map step 10).
- `game_log.rs` needed nothing: step 1 had already removed the mclo.gs upload, and with it the
  account-name scrubbing the map still listed.

**Surprises**:
- `launcher/build.rs` no longer reads `HORIZON_MSA_CLIENT_ID` (the map was stale): the ID came
  through `option_env!` in account.rs, so deleting account.rs removed the last read and the
  forbidden ID's path into the binary. build.rs needed no change.
- `Msg::Session` became `Msg::AssetsDone` (the start's thread now only completes the assets);
  `HORIZON_LAUNCHER_PREVIEW_OWNER` became `HORIZON_LAUNCHER_PREVIEW_READY` in glass_home.rs.
- `play_press` unprepared now means "Prepare & play" for everyone (from the computer's own
  install), where it used to need an owner: the school edition has nobody to ask for one.
- `heads.rs`'s `sha1`/`base64` uses died with the profile faces; both deps stay (downloads,
  sources, server_list).
- rustfmt wrote LF into the files it touched; they were converted back to the tree's CRLF.

### Step 4 (2026-11-22): nothing downloads, updates or installs; builds are the club's folders

`cargo check`, `cargo check --all-targets` and `cargo clippy --all-targets` all green, no
warnings, on Windows x64. Cumulative `git diff --shortstat -- launcher` (steps 1-4, uncommitted):
289 files, +3,117 / -116,387; step 3 ended at 254 files, +1,356 / -89,294, so this step is about
35 paths, +1,761 / -27,093 (43 files deleted by `rm`, untracked `src/club.rs` added).

**Modules deleted** (43 files):
- Release machinery: `github.rs` + tests, `signed.rs`, `nightly.rs`, `min_launcher.rs`,
  `backend/{installation,release_assets,release_notes,release_selection,update_selection,
  release_loading_tests}.rs`, and in `backend.rs` the release list, the channels (Nightly,
  Stable), `install`/`install_as`, `install_tag`, `fetch_startup_releases`, `remove_version`,
  `remove`, `prune_old_versions`, `stale_versions`, `in_use`, `disk_use`, `Updates`,
  `releases_and_updates`, `check_updates`, `nightly_downgrade`, `release_notes`, the HTTP helpers
  (`agent`/`get`/`get_bytes`/`get_unless_missing`/`download`/`save_checked`/`unpack`), the whole
  GameArchitecture block (settings field and Versions rows with it), and `Links` + `links()` with
  `launcher/links.json`.
- Self-update and installer: `update_handoff.rs`, `installer.rs`, `setup_page.rs`,
  `setup_args.rs`, `uninstall_aside.rs`, `update_recovery.rs`, `mac_update.rs` + tests,
  `developer_install.rs`, and in `backend.rs` the exe-path self-update half (update_target,
  stage folders, place_checked, replace_binary, restart, cleanup_launcher_updates,
  package_managed_launcher, download_page...).
- Movable data folder: `data_location.rs` + tests, `location_page.rs`, `relocation.rs` + tests,
  `move_window.rs`; `HORIZON_LAUNCHER_LOCATION_FILE` handling went with them.
- Storage report and eviction: `storage/{eviction,inventory,inventory_regressions,policy,tests}.rs`,
  `glass_storage.rs`; `storage.rs` reduced to `format_bytes`, `allocated_bytes` (the walk) and
  `mark_used` (what `run_leases`, `glass_sheet` and `glass_home_lists` still call);
  `cache_cleanup.rs` reduced to `Links` (`Access`/`Proof`/`Plan`/`delete_*` lost every caller);
  `native_cache.rs` reduced to `Error`/`cache_error`/`checked`/`linked`/the lock re-exports
  (`Cache`/`PROFILES`/`native_root` had none). `Launcher`'s eleven `storage_*` fields,
  `disk`/`disk_for`, `installing`/`install_error`/`confirm_remove`/`install_started` gone.
- Internet exposure: `internet.rs`, `tailscale.rs` + tests, `keychain.rs`; servers_page's
  `internet_tick`/`internet_panel` calls and glass_servers' `glass_internet`/`internet_invites`
  cards with them (also `Field::{Token,Email}`).
- Orphaned helpers: `download_retry.rs`, `notes_cache.rs`, `markdown.rs`, `servers.rs`'s
  runtime-link half (RuntimeLinks, registered_services, scheduled_tasks...), `instances.rs`'s
  `game_caches`, `windows_runtime.rs`'s `in_use` family, `run_leases::installation`,
  `servers_page::keep_running_through_update`, `widgets::{removable,update_name,download_error}`,
  `glass_ui::bar`, `glass_settings::tile`, `cull::estimated`, `glass_home_tabs`' patch-notes
  parser, `main.rs`'s `Load`/`SelfUpdate`/`PrepareFix`/`OPENING_MAX`.
- Tests that pinned removed behaviour: backend's release/install/self-update/prune/reuse tests
  (101 to 31 kept), `native_cache/tests.rs`, `cache_cleanup/tests.rs`, `tests/developer_install.rs`,
  glass_instance's release-picker test, windows_runtime's `in_use` tests, cull's `estimated` test.

**New module: `src/club.rs`** — the club's folders, with tests for the pure parts:
- `root()`: `HORIZON_CLUB_ROOT` if set; else, when the executable is exactly
  `root\app\<version>\<name>.exe` (its folder's parent named `app`), three levels up; else the
  nearest folder at or above the current directory, four levels up at most, holding `club.toml`;
  an empty path when none is found (`has_root()` says so, and Home and Versions then ask for the
  supervisor). The answer is fixed on the first call, before startup changes the current
  directory.
- `data_dir()` / `game_home()`: `HORIZON_LAUNCHER_HOME` / `HORIZON_HOME` if set, else
  `%LOCALAPPDATA%\HorizonClub\{launcher,game}`, made on first use; tests get a per-process
  scratch folder under the temp dir. `backend::data_dir`/`game_home` are one-line delegations
  to them (a dozen kept callers read unchanged).
- `versions_dir()`: `<root>\core`; `client_program_name()`: `horizon-client.exe` (else
  `horizon-client`). `backend::installed()` lists the folders of `core\` that hold that file,
  newest name first; `is_installed` checks it; `game_version`/`is_prepared`/`check_prepared`
  read what they read before (`research-lock.json`, the prepared manifest).
- Startup (`startup.rs`): resolves `club::root()`, then makes the data folder the current
  directory (the share is read-only, a UNC path a poor working directory); the quiet installer,
  relocation worker, update handoff, replaced-binary cleanup and setup-window modes are gone.

**The launcher writes nothing inside a build folder** — every write found is removed:
- the `.installed`/`COMPLETE` marker (reads now check `horizon-client.exe`),
- `.auto-installed` (`mark_auto_installed` gone),
- `horizon-data.bin` after an install (`write_data_file` gone),
- `complete_assets` and its five asset-store copies (the whole Completion table; Play's start
  thread now only checks `need_prepared` and refuses with the supervisor sentence),
- `complete_reports` (registries/commands copied between builds),
- `reuse_prepared`/`reuse_prepared_for`/`reuse_prepared_everywhere` and the preparer-output
  compatibility compare (`resources_compatible`, `same_preparer`, `LEGACY_*`),
- `Msg::Reused` and the startup thread that ran it,
- `servers.rs`'s `complete()` error no longer names the marker file.
What still writes into a build folder: the *preparation* itself (`backend::preparer`,
`prepare_in_process`, `sounds`), kept whole for the later step that moves it into club-admin.

**Kept files changed** (besides the above): main.rs (`shut_down` stops only the servers; the §4
groups "Self-update", "Storage UI", Location, notes/links fields gone; `download_seen` stays: it
times the preparation's Mojang download), launcher_setup.rs (no release fetch, no links fetch,
no update check), launcher_messages.rs (drain reduced to
AssetsDone/PlayStep/GamePrepared/Line/Exit), launcher_play.rs (no install/remove/reinstall;
`version_still_installed` without a Reinstall offer), launcher_render.rs (the opening no longer
waits for the release list), blocks_play.rs / home.rs / glass_home.rs (no update offers, no
What's-new, no developer builds; Play's menu keeps only Prepare), glass_versions.rs +
blocks_versions.rs (rewritten: the builds of `core\`, name + "Minecraft {version}" + In use /
Newest, no Install/Remove/notes; the no-root sentence), glass_instance.rs (the version dropdown
lists `installed` under Latest, no channel segments, no Stable offers), instances.rs +
instances_page.rs (a pinned-but-missing build says to ask the supervisor; `Channel` stays for
old instances.json files), blocks_settings.rs + glass_settings.rs (SECTIONS 8 to 6: Updates,
Storage, Location gone; General/Appearance/Game/Network/AI/About; the glass About keeps Version
and Logs), frame.rs + blocks_chrome.rs (no rail update card, no sidebar update plate; the
version line and picker stay, the picker's "auto-update" badge is "the launcher's"),
widgets.rs (version_tag shows the raw tag), servers_page.rs + glass_servers.rs (internet panel
and Token/Email fields gone), refusals.rs (no_build cards and the mac-Host machinery),
backend/settings.rs (fields `following`, `game_architecture`, `install_new_versions`,
`prune_old_versions`, `keep_versions`, `cache_limit_gib` gone; an old settings.json still
loads), backend/game_start.rs and `server_command` (no developer-install env), Cargo.toml
(dropped horizon-addresses, horizon-release-sign, clap, keyring, oo7, security-framework,
winreg, pulldown-cmark, tar; tempfile moved to dev-dependencies; lock file regenerated
offline), lang/fr.json (17 new keys).

**Deliberately left for later steps** (where it lives now):
- The preparation stack, as asked: `backend::preparer` (backend.rs, "How a version's resources
  are prepared"), `mojang.rs`, `sources.rs`, `downloads.rs`, `proxy.rs`; the old
  `prepare_hint` CLI sentence is replaced by the supervisor sentence, but Home's Prepare button
  and its Minecraft-folder row stay (blocks_play.rs, launcher_play.rs `prepare`).
- The lease layer's remains: `run_leases.rs` (mark/shared/spawn/publish/snapshot/
  game_snapshot/Registry alive; `installation` deleted), `native_cache.rs`
  (Error/cache_error/checked), `cache_cleanup.rs` (Links), `storage.rs`
  (format_bytes/allocated_bytes/mark_used), `process.rs`; `backend::build_dir` keeps its
  ASCII-safe resolution for them.
- `instances.rs`, `instances_page.rs`, `glass_instance.rs`, `glass_sheet.rs`, `instance_icons.rs`:
  the instance dialog and sheet keep working over the builds of `core\`; a later step reduces
  them.
- `launch.rs`, `shortcut.rs`, `desktop.rs`, `desktop_seams.rs`, the hosting (`servers*.rs`,
  including its own "Updates" section that runs `horizon-server update` on the server folder:
  servers_page.rs `updates_section`), theme and widget modules, `game_app.rs` (macOS, not
  compiled here).
- `launcher/README.md`, `launcher/docs/` images, the website's read of `launcher/links.json`
  (now gone from this branch), lang keys of deleted features: map step 10.
- The `windows` crate's `Win32_System_TaskScheduler` feature stays enabled though
  `scheduled_tasks` died; harmless, and trimming features is the Cargo sweep's business.

**Surprises**:
- `backend::installed` used GitHub's kept release list to order same-day Nightlies; builds are
  now ordered by folder name descending, which is the supervisor's dating scheme.
- `servers.rs`'s runtime-link machinery and `windows_runtime::in_use` existed only for installs,
  moves and deletes; deleting those made them dead code, so they went with this step rather than
  staying as warnings.
- `LAUNCHD_PREFIX` (servers.rs) survives: macOS-only, used by parts of servers.rs not compiled
  on this machine (verified by rg, not by the compiler).
- The blocky Versions page had a "Loading releases..." state at boot, and the opening animation
  waited for the list; both now resolve at once, and `OPENING_MAX` went with them.
- `storage::mark_used` writes a `.last-used` stamp — into cache units under the *data* folder,
  never a build folder, so it stays; the no-write-in-builds rule was checked against every write
  path in backend.rs (the preparation's own writes aside, kept for its step).

### Step 5 (2026-11-22): one game folder, no lease layer, any kind of drive

`cargo check`, `cargo check --all-targets` and `cargo clippy --all-targets` all green, no
warnings, on Windows x64. The code only another system compiles (`game_app.rs`, the Unix half
of `backend.rs`, `servers/tests.rs`'s Unix test) was edited where it named a deleted module or
pinned deleted behaviour, and is not compiled here. Tests were compiled and linted, not run.
Cumulative `git diff --shortstat -- launcher` (steps 1-5, uncommitted): 305 files, +3,277 /
−127,584; step 4 ended at 289 files, +3,117 / −116,387, so this step is about 16 paths,
+160 / −11,197 (the diff counts neither the untracked `src/horizon_mark.rs`, 65 lines, nor the
untracked `src/game_start.rs` it deleted, 111).

**Modules deleted** (31 files by `rm`, about 9,950 lines):
- The lease layer: `run_leases.rs` + `run_leases/{control,tests}.rs`, `native_cache.rs`,
  `cache_cleanup.rs`, `storage.rs` + `storage/listing.rs`, `process/leases.rs` +
  `process/leases/{windows.rs,tests.rs,tests/windows_tests.rs}`, `test_support.rs`, and
  `refusals.rs` (the lease layer's and the cleanup's sentences; its one other line, the crash
  card's, is `backend::crash_help`).
- With it, two files that only it used: `process/supervisor.rs` + its tests (no caller since
  step 1: neither hosting nor the game start went through it) and `process/output_tests.rs`
  (tested the engine's output capture, which only the run holder installed).
- Instance management: `instances.rs` + `instances/tests.rs`, `instances_page.rs`,
  `glass_instance.rs`, `glass_sheet.rs`; `instance_icons.rs` is `horizon_mark.rs` now
  (Horizon's mark alone, which the rail, the top bar and Home's title draw), and
  `assets/logos/grass-block.png` (the instance dialog's grass icon) is gone.
- Launch wrappers and shortcuts: `launch.rs` + `launch/{hook,tests}.rs`, `shortcut.rs` + its
  tests, `desktop.rs` (the Linux applications-menu entry), the window's `game_start.rs` (the
  hop that applied an instance's profile and wrapped its command) and `backend/play_log.rs`
  (which spawned through the holder).

**The staging as implemented** (`windows_runtime.rs`, Windows only):
- `backend::client_command` marks the game's command with the folder of its build, as the club
  root spells it. The mark is an environment override that only the launcher reads back: `stage`
  stages the marked command and takes the mark off it, so no process sees it; any other command
  (the preparer's) runs as it is and reaches no data folder.
- The slot is `<data>\run\<key>\game\horizon-client.exe`, `key` being the SHA-256 of the game
  home as the game gets it (no canonical form: nothing on the game's path is `canonicalize`d any
  more). The firewall sees that one path whatever build plays.
- The slot's lock (`run.lock`) is held from the staging until the game's process has ended, and
  the build's `horizon-client.exe` is held open, shared with readers only, for as long: a second
  launcher of the same game home is refused, and the build's program can be neither replaced nor
  deleted meanwhile.
- What goes along: the program and everything beside it in the build folder but `.local` (which
  `--resources` names on the share): every file (`dxcompiler.dll` among them) and every folder,
  copied whole.
- When: `staged.json` in the slot records the build folder as it was copied, each top-level entry
  with its name, whether it is a folder, its length and its modification time. At each Play the
  build folder is listed again (one stat per entry on the share, no file is read) and the build is
  copied again only when that listing differs from the record, when the record is missing or
  unreadable, or when a recorded entry is missing from the slot (an antivirus that quarantined a
  file, a folder cleaned by hand). An unchanged build costs no copy.
- How: the program first (Windows refuses to replace a running one, and then nothing was
  removed), then everything else in the slot's game folder goes, then the build's other entries
  are copied. The record is removed before the copy and written after it, so a copy that stops
  halfway never vouches for what the slot holds.
- Where: on the start's own thread. `Launcher::start_client` (`launcher_play.rs:153`) builds the
  command (where the next start goes at once, the player's extra arguments), a thread says the
  step, calls `backend::prepare_command` (`backend.rs:1224`) and sends `Msg::GamePrepared`; `drain`
  starts the process. Before, the copy ran on the window's thread, at every Play. A copy says so
  on stderr.
- Known limits: a folder beside the program is known again by its own time, so a change inside
  `LICENSES\` or `exporters\` alone is not seen; a time that a FAT share reports shifted by
  daylight saving makes one more copy.
- Tests, compiled not run: `a_time_is_kept_as_seconds_then_nanoseconds_on_either_side_of_1970`,
  `a_build_is_copied_once_and_again_only_when_it_changes`, `only_the_marked_game_is_staged_and_each_home_has_its_slot`
  and `a_second_game_of_the_same_home_is_refused_while_the_first_runs`.

**The drive refusals removed, and what holds instead**:
- They all came from one place: the lease registry of the platform (`horizon-platform`'s
  `mod_package::runtime_leases`) refuses a file system that is not a local disk when it checks or
  locks a folder, and reported it as `UnsupportedFilesystem`, worded by `refusals.rs` ("its file
  system isn't supported"). It fired for the data folder (`Registry::open` at every start), for
  the game home, its logs and its cache folders (`run_leases::shared`), for the build folder and
  its `.local` (the `--resources` paths and the marked source, locked and, on Windows, leased as
  units of the staged game), and for whatever the cleanup measured (`storage`'s walk). The linked
  folder refusals of the cleanup ("is a link", "Refusing native cache cleanup…") and
  `unwritable` ("Make sure your account can write to…") went the same way. Nothing replaces them
  by a check: there is nothing left that locks, cleans or needs a local file system.
- Data folder, game home, logs and crash reports: `club::data_dir()` and `club::game_home()`
  (`%LOCALAPPDATA%\HorizonClub`), and the working directory is the data folder (step 4).
- The build folder and the game's files: read as they are, through the path `club::root()` has.
  A relative `HORIZON_CLUB_ROOT` is made absolute where the launcher starts, without Windows'
  verbatim prefix (`club::absolute`, `club.rs:46`; test
  `a_relative_root_is_made_absolute_and_an_absolute_one_keeps_its_spelling`), so the paths built
  on it hold after the working directory changes and are spelled as given to the game. The three
  users of `canonicalize` on that path are gone with the layer (`backend::build_dir`,
  `run_leases`, the slot key and `windows_runtime`'s `source.json`), and with them `backend::plain`,
  which existed to undo the `\\?\` prefix. What is left is off the game's path: `sources.rs`
  (comparing two folders of other launchers, in the preparation stack), `backend/java_tests.rs`
  and `game_app.rs` (macOS).
- The rule file: the game reads `managed.txt` from the path `--managed` names
  (`crates/horizon-client/src/managed.rs`). No step passes `--managed` yet; when the
  `club.toml` step passes `<root>\managed.txt`, the root is absolute and spelled as given. Nothing
  else was needed here.

**Kept files changed**: lib.rs (declares `game_app` again, see surprises), backend.rs,
backend/{game_start,messages,settings}.rs, windows_runtime.rs, servers.rs (+tests), process.rs,
worlds.rs, desktop_seams.rs (its doc), game_app.rs (macOS), club.rs, main.rs, startup.rs,
launcher_setup.rs, launcher_render.rs, launcher_messages.rs, launcher_play.rs, play_step.rs,
frame.rs, blocks_chrome.rs, blocks_play.rs, blocks_settings.rs, home.rs, glass_home.rs,
glass_home_lists.rs, glass_home_tabs.rs, glass_palette.rs, glass_settings.rs, glass_versions.rs,
glass_server_dialog.rs, home_tabs.rs, worlds_page.rs, multiplayer_page.rs, console_page.rs,
servers_page.rs, blocks_versions.rs, widgets.rs, blocks.rs, glass_ui.rs, shell.rs, theme.rs,
server_list.rs (docs), Cargo.toml, Cargo.lock, lang/fr.json, tests/{hidden_processes,startup}.rs.
- In backend.rs: `build_dir` with its ASCII folder for MinGW tools (`ascii_build_dir`,
  `short_path`, `tool_safe`), `exe_path`, `appimage_path`, `Logged` and `LOG_KEEP` (the CLI's
  process holder), `RuntimeGuard`, `spawn_attached`, `startup_cleanup`, `resolve_version` (the
  instances' pins), `switch_after_start_crash` (an instance's own home) and `Running::leases_refused`
  are gone. `prepare_command(cmd)` answers a `Staged` (the command as it runs, with the slot's
  lock), `spawn(job, staged, tx)` starts it, `Running::kill` comes from `run_leases/control.rs`
  (`send` had no caller left but a test), and a start that fails names the program and its folder
  (`not_started`). On Unix a game that dies by a signal now reports the signal: exit code 1 used to
  be the run holder's word for it.
- Messages: `Msg::AssetsDone` is gone; `Msg::GamePrepared(Box<Result<Staged, String>>)`; a game
  that ends badly always says `Exited with …` (code 78's lease sentence is gone with the leases).
- `Settings`: `instance` and `desktop_entry` are gone (an old settings.json still loads).
- Hosting: `servers::run_managed` no longer leases the engine on Windows, and `set_up_in` no
  longer writes the `.installed` marker. The game takes leases only under a `run-leases` folder
  above its executable (`holder/game_entry.rs` in `horizon-mod-package`), and nothing makes one
  now; `horizon-server` writes its own marker when it leases (`crates/horizon-server/src/engine.rs`).
- `Launcher`: `version_menu`, `sheet_size`, `sheet_tab`, `client_home`, `starting_instance`,
  `pre_launch_stop`, `client_run`, `client_vars`, `instances` and `rooms` are gone;
  `play_when_prepared` is a build's tag. The instance dialog, the sheet and their Escape handling
  are gone from `launcher_render.rs` and `servers_page.rs` (`Field::{InstanceName, InstanceGroup}`).
- The rail (glass): one row, "Horizon": its mark, its name and its state (Running, the step a
  start is on, Ready · the build, Needs preparing, No version installed). A click opens Home. Its
  ⋮ and a right click open a menu with "Open folder" only (the game home). No "Add instance".
  The row stays marked on every page, as the selected instance's row did.
- The blocky sidebar: the same row, without a heading or a + (it never had a menu). The blocky
  Home loses its Instance row; its Folder button opens the game home. Its version picker now picks
  the launcher's build (`Launcher::pick_version`, `Settings::pick`): the "the launcher's" badge
  marked an instance's pin.
- The glass Home: the title is Horizon's mark and name, with no edit dialog behind it; the
  settings pencil is gone; the folder menu is Open the game folder, the screenshots folder and
  the Minecraft folder; the version chip is a label (it opened instance settings); the status
  line is "Playing for {time} now" while a game runs. Play time and last played lived in
  `instances.json` and are gone with it; `play_time` is in glass_home.rs.
- The palette: Play, the pages, Open the game folder (no Play per instance, no New instance).
  Versions: no "Used by {names}". Home's tabs read the game home, and `Worlds` and `Multiplayer`
  keep no folder any more. Settings: no "Applications menu" (Linux).
- The server-create dialog keeps its six widget helpers (`choice`, `choice_text`, `list_box`,
  `list_choice`, `section`, `tick`), now private functions of glass_server_dialog.rs.
- Left dead by the deletions and removed: `glass_ui::{Rooms, room, cut, big_icon}`,
  `blocks::slot`, `widgets::row_menu_after_press`, `frame::{RowPhase, running_ids, row_phase}`,
  `storage::format_bytes` (it is `worlds::format_bytes`), `Launcher.rooms`.
- Cargo.toml: dropped `cap-std`, `cap-fs-ext` and `windows` (its one user left was
  `shortcut.rs`); `horizon-platform` keeps `prepare`, `protocol`, `world` and loses
  `mod-runtime-process`; `windows-sys` keeps the two features `backend::process_alive` needs
  (`Win32_Foundation`, `Win32_System_Threading`). `same-file` was never a dependency of the
  launcher (it is in the lock file through other crates). The lock file was regenerated offline.
- lang/fr.json, 5 keys: "1 server", "{count} servers", "More rows below", "Play, open a page",
  "Could not complete game launch: {error}" (the last existed in `backend/play_log.rs` without
  French).
- Tests: deleted with what they pinned: backend's build-folder and `plain` tests (3),
  `a_start_crash_switches_the_game_home_only_after_the_crashed_runs_own`,
  `only_an_appimage_file_counts`; `servers`' `a_release_carries_the_completion_marker_the_game_opens`,
  `the_engine_finds_every_unit_it_leases_registered`,
  `an_engine_from_outside_the_launcher_is_left_alone`; main.rs's
  `a_press_on_a_sidebar_row_opens_its_menu_or_closes_it` and
  `a_row_shows_the_game_of_its_own_instance_only`; glass_home's `played_time_reads_as_the_mockup_writes_it`
  (its `since` test stays). Changed: `a_queued_play_waits_for_another_start_to_finish` (was
  "…for_a_pre_launch_command_to_finish"), `steps_reach_the_window_in_order_and_clear_when_the_start_ends`
  (two steps, the new message), `stop_is_not_a_native_crash` (Unix half),
  `programs_are_laid_out_as_install_sh_does` (Unix: no marker), `tests/hidden_processes.rs`
  (prepares the command, writes into the pipe itself), `tests/startup.rs` (its settings fixture).
  New beside the staging's: `a_running_time_reads_as_minutes_then_hours` (from `instances`).

**Deliberately left for later steps** (where it lives now):
- The CLI's leftovers in the backend, unused by the window since step 1:
  `backend::{server_command, server, minecraft_dir, server_dir, read_logs, newest_crash_report}`
  (backend.rs:1068, 1134, 1100, 1108, 959, 769) and `Job::Server`. `server` and the EULA
  (`agree_eula`, `eula_agreed`) are the `eula_accepted` step's, as step 2b left them.
- The preparation stack: `backend::preparer` (backend.rs:501), `sounds`, Home's Prepare button
  and its Minecraft-folder row, as step 4 left them.
- Hosting: `servers::set_up_in` (servers.rs:266) still copies a release into `server-runtime`
  (and no longer completes it with `.installed`): the hosting step reads `core\` in place.
- The glass look has nowhere to pick a build now: `version_chip` (glass_home.rs:443) is a label,
  and the blocky Home's picker (blocks_chrome.rs:205) is the one place. `[versions] active` in
  `club.toml` supersedes both.
- The Multiplayer tab still adds and removes servers, with two strings this step had to reword
  (glass_home_lists.rs:262): the club's server list rewrites the tab.
- `perf.rs` still writes a report under `benchmarks/` of the data folder at each session
  (perf.rs:69) and its doc still names `horizon-launcher benchmark`; `specimen.rs` is the debug
  page behind `HORIZON_LAUNCHER_PAGE=specimen`, with English literals. Both have callers, so both
  stay as asked.
- `game_app.rs` (macOS, game_app.rs:65; `backend::client_program`, backend.rs:1058) writes
  `Horizon.app` and hard links into the *build folder*, which a read-only share refuses. Not
  compiled here.
- The data folders of launchers before the trim may still hold `run-leases` (a game finding it
  above its executable takes leases and exits with 78), slots keyed by the canonical game home
  under `run\`, and `source.json` files. The club's `%LOCALAPPDATA%\HorizonClub` never had them.
- Outside `launcher/`, still naming the deleted grass block: `docs/design-system.md:182`,
  `docs/mod-browser.md:143`, `tools/launcher_grass_block.py`. Lang keys of deleted features,
  the README and steps 1-4's other leftovers are where they were.

**Surprises**:
- The drive refusal was not a check of the launcher's own: it was the platform's lease registry
  refusing network file systems, reached through four paths. Deleting the layer removed every
  one of them, and the five refusal sentences with it.
- `Running::{send, kill}` lived in `run_leases/control.rs`; `prepare_command` used to lease the
  game home, its logs and caches, the build folder and `.local` before every start.
- `#[cfg(unix)] pub mod game_app;` had dropped out of lib.rs in an earlier step while
  `backend::client_program` (macOS) and `servers/tests.rs` still call `crate::game_app`: the
  launcher could not have built there. It is declared again, with a local `xml` (was
  `shortcut::xml`).
- The old staging copied the whole build at every Play, on the window's thread. With the record,
  a Play is one directory listing of the share.
- `Command::get_envs` reports a variable a command takes off, as `None`: the staging's test asks
  for that, not for the mark's absence.
- `tests/hidden_processes.rs` reached the child's stdin through `Running::send`, whose only
  other user was the CLI's `Logged::stop`: the test writes into the pipe itself.
- `a_build_is_copied_once…` was wrong as first written, twice over: its `get_envs` assertion
  (caught by reading) and a local that shadowed its helper (caught by the compiler). Both matter
  because the rule of this step is no `cargo test`: the new tests have never run.
- `tests/hidden_processes.rs` and `tests/startup.rs` were LF in the working tree (every other
  file under `launcher/src` and `launcher/tests` is CRLF); they are CRLF again, which git's
  normalisation does not show.

### Step 6a (2026-11-23): the club's configuration drives the launcher

`cargo check`, `cargo check --all-targets`, `cargo clippy --all-targets` and `cargo fmt --check`
all clean, no warnings, on Windows x64; the lock file followed the manifest offline. Cumulative
`git diff --shortstat -- launcher` (steps 1-6a, uncommitted): 311 files, +3,860 / −129,734; step
5 ended at 305 files, +3,277 / −127,584, so this step is 6 paths, about +583 / −2,150 in tracked
files, plus the four files it adds untracked (1,173 lines) and its changes to the untracked
`club.rs`, `identity.rs` and `backend/game_start.rs`. No file deleted.

The launcher's own tests were compiled and linted, not run (the rule of this step). The 24 unit
tests of the five pure modules it adds or rewrites (`config`, `online`, `first_play`, `identity`,
`server_list`) were run for real, outside the workspace: a scratch crate in the temp folder
includes those files by path, with stand-ins for `i18n` and `club` and a one-module
`horizon-platform`. That run caught a bug before it could ship (surprises, first item). The
ordering of build names and the status ping's pure parts ran the same way.

**New modules** (library, declared in `lib.rs`):
- `config.rs`: `Config`, read from `<root>\club.toml` with the `toml` crate (`Config::load`;
  `Config::read(root)` and `Config::parse(text)` for tests). Tables `[club]`, `[[servers]]`,
  `[game]`, `[lan]`, `[hosting]`, `[versions]`, every key optional. An empty file, a missing file
  or no club root at all is `Config::default()`: the club is "Club Minecraft", in French, named
  by the Windows session, no shared session, no server, render distance 8, simulation distance 5,
  60 frames a second, LAN open, hosting enabled from port 25565 to 25575 with the EULA accepted,
  no active build. The scalars are the spec's example; its `["eleve"]` and its one server are
  examples, so the two lists start empty. A file that cannot be read is `config::Error`: the
  sentence Home shows, with where the parser stopped ("TOML parse error at line 4, column 12:
  unknown variant `xx`, expected `fr` or `en`").
- `online.rs`: the reader of `<root>\online\*.json`. `parse` takes an announcement of the spec's
  shape and refuses a broken one; `list(dir, now)` keeps those under 45 seconds old
  (`FRESH`), by name then address. `local(host)` is the rule below (surprises).
- `first_play.rs`: the three files of a student's first Play as `options(game, language)`,
  `PERFORMANCE` and `NO_AI`, and `write_missing(home, game, language)`, which writes them when
  `options.txt` is not there yet and never again.
- `server_status.rs`: the server-list status ping (handshake, status request, the JSON answer's
  players), brought back from `crates/horizon-telemetry/src/server_status.rs` at HEAD.

**The rules as implemented**:
- **`club.toml` is strict.** `deny_unknown_fields` on every table: a name misspelt
  (`shared_acounts`) is an error the window shows, never a rule that silently doesn't apply. A
  value of another kind, an identity mode or a language the launcher doesn't have, and a server
  with no name or no address are errors too. The window still opens; Home says
  "The club's file club.toml could not be read: … Ask the club's supervisor to fix it.", the
  rail's row says the same in three words, and Play does nothing. Until the file reads again the
  last good one stands for the window (its name, its servers), or the default one.
- **The club's file is read again** at the window's start, before every Play, when a game ends,
  every 5 seconds while Home says the game can't start (`RECHECK`, main.rs), and with each refresh
  of the Multiplayer tab's list. So what the supervisor fixed shows without a restart.
- **Play's checks, in this order** (`Launcher::refusal`, launcher_play.rs): the club's file; who
  plays; a build to play; its files; the rule of the game. Each is one sentence on Home (above
  the tabs) and a few words on the rail's row: "The club's file could not be read", "No player
  name", "Shared session", "No version installed", "Needs preparing", "The rule of the game is
  missing"; else "Ready · <build>". Play stays pressable while refused: a press reads the file
  again and starts the game when nothing keeps it from starting.
- **Who plays**: `identity::player(shared_accounts)` folds the Windows login as before, and
  refuses it, whatever the case either is written in, when `shared_accounts` lists it
  (`IdentityError::Shared`): "This Windows session is shared by several students, so the game
  can't start. Ask the club's supervisor for a session of your own." The top bar's chip shows no
  name then, as for a login too short to make one.
- **The build in use** (`config::Versions::build`): `[versions] active` when `core\` holds it;
  else the only build there; else the newest folder name. An `active` that names no build of
  `core\` falls to the same rules. Names compare a run of digits at a time as a number
  (`club::newer`), because the CI names builds `club-<run>-<commit>` and plain text order took
  `club-9-…` for the newer of it and `club-10-…`; it orders the Versions page too.
- **The rule of the game**: `--managed <root>\managed.txt` (`club::managed_file()`). The launcher
  only checks the file is there and never writes inside the club root; missing, Play says "The
  club's file managed.txt is missing, so the game can't start. Ask the club's supervisor for
  it."
- **No preparation in the window.** Home's Prepare button, its ⋯ item, its steps and download
  bar, its time left, the Resources and Minecraft folder rows and the "Choose the Minecraft
  folder" menu item are gone, with the state behind them: `Launcher::{preparing, play_when_prepared,
  prepare_error, prepare_started, prepare_tag, download_seen}`, the `getting-ready` preview and
  `HORIZON_LAUNCHER_PREVIEW_READY`. `prepare_log` is `console_log`: it only ever held the game's
  lines now, and the Console tab shows them as before. `Job` is gone (only the game is spawned):
  `Msg::Line(String)`, `Msg::Exit(Result<(), String>)`, `backend::spawn(staged, tx)`.
- **The server list** (`server_list::replace`, `write_club_list`, `club_servers`): `servers.dat` in
  the game home is rewritten before every Play and whenever the Multiplayer tab's list refreshes
  (every 5 seconds while Home shows, `REFRESH`; never while a game runs, whose list it is then):
  the `[[servers]]` of `club.toml`, then the announcements under 45 seconds old. Every entry is
  `name`, `ip`, `acceptTextures:0` (the game reads a set flag as the pack's status: disabled) and
  `hidden:0`; a server already in the list keeps the picture the game last saw of it; the hidden
  entries of Direct Connection go; the old file stays as `servers.dat_old`, as the game keeps it.
  A failure is said on Home and doesn't keep the game from starting. The tab lists the same
  entries (name, address, "3 of 10" or "Offline" from the status ping, Join) and has no Add, no
  Remove; `server_list`'s `add`, `remove`, `shown_file` and the loading `List` are deleted. The
  Home tab is "Multiplayer" in both looks.
- **First Play**: the three files below are written by `start_client`, before the game starts.
  They are written again only while `options.txt` is missing: a student who changed anything in
  the game's screens keeps it, a deleted `horizon-extras.txt` included. A failed write is said on
  Home and doesn't keep the game from starting.
- **The window**: its title (the taskbar's and the drawn title bar's) is `club.name`, and so are
  Home's heading and the rail's row, above the row's state as before; the top bar keeps the
  player's name, Horizon's mark stays. The blocky build picker is gone with `Settings::selected`,
  `pick_version`, "No version" and "Manage versions"; the glass build chip is a label.
- **Settings** has three parts. General: language (the club's language, English and French in the
  dropdown; the club's until a student picks one), what the window does while the game runs.
  Appearance: theme, accent, Glass and Blur (glass look), performance mode, motion, effects, Home
  background; the glass look's "Reset the launcher's look" row is gone ("Nothing else"). About:
  the version ("Launcher <version>, game <build>", Copy), the club's name, the logs folder (Open),
  the licence (from `CARGO_PKG_LICENSE`), and "Not an official Minecraft product. Not approved by
  or associated with Mojang or Microsoft." Gone with their state: the opening switch
  (`Settings::skip_opening`; the opening always shows unless motion is reduced), Game arguments
  (`Settings::game_args`, `split_args`, `append_game_args`: a student cannot add an argument to
  the game's command), Proxy (`Settings::proxy`), No AI (`backend::{no_ai, set_no_ai}`: the first
  Play writes it).
- **Language**: `Settings::language(club)` is the student's pick, else the club's
  (`[club] language`, `fr` or `en`). The system's language is no longer read: `Language::
  {system, pick, parse}` and the `sys-locale` dependency are gone. The 87-language enum and the
  85 other tables stay (below).

**The game's command line** (`backend::client_command(tag, player, managed)`, launcher_play.rs
`start_client`): the staged copy `<data>\run\<key>\game\horizon-client.exe` on Windows (the
staging of step 5, unchanged), working directory the game home, environment `HORIZON_HOME=<game
home>`, `HORIZON_NO_HEARTBEAT=1`, `PIPEWIRE_PROPS` when unset; then

```text
--resources <root>\core\<tag>\.local\horizon-client\<Minecraft version>
--managed <root>\managed.txt
--name <Name>
[--server <address> | --world <name>]
```

and nothing else: no session file, no environment variable of an account, no extra argument.
`--server` is the entry's address as written; `--world` a folder of `saves\`.

**The three first-Play files** (the game reads them, keeps the values and completes the files):
`options.txt`, the first three lines and `lang` from `[game]` and `[club] language`
(`en_us` when the club's language is not French):

```text
renderDistance:8
simulationDistance:5
maxFps:60
lang:fr_fr
onboardAccessibility:false
skipMultiplayerWarning:true
renderClouds:"false"
particles:2
entityShadows:false
biomeBlendRadius:0
mipmapLevels:0
entityDistanceScaling:0.5
guiScale:0
fullscreen:false
```

`horizon-graphics.txt`, what `apply_preset(Preset::Performance)` assigns to the game's `Graphics`
among that file's keys, in the order `Graphics::serialize` writes them. Ten of the 26 lines
differ from `Graphics::default()`: `preset`, `gpuCulling`, `entityCulling`, `particleCulling`
and six `blockEntityCache…` (not `…Signs`, true by default):

```text
preset:performance
lodDistance:0
caveCulling:true
entityCulling:async
particleCulling:async
opaqueIce:false
blockEntityCacheShulkerBoxes:true
blockEntityCacheSigns:true
blockEntityCacheDecoratedPots:true
blockEntityCacheBells:true
blockEntityCacheChests:true
blockEntityCacheBanners:true
blockEntityCacheCopperGolemStatues:true
chunkUpdates:0
uploadBudgetKiB:0
sky:true
fog:true
weather:true
animatedTextures:true
ditherCutoutMips:false
resolutionScale:100
dynamicResolution:false
fxaa:false
taa:false
upscaling:native
gpuCulling:auto
msaa4x:false
```

`horizon-extras.txt`: `noAi:true`. The Performance table also sets `cutoutLeaves:false` and
`weatherRadius:3` among vanilla's options; the shape this step was given leaves both to the
game's defaults, as it gives the preset's simulation distance (6) to the club's (5).

**In backend.rs**: deleted with their last caller: `server`, `server_command`, `minecraft_dir`,
`server_dir`, `read_logs` (the CLI's leftovers), `newest_crash_report`, `tail_of`,
`default_minecraft_dir`, `gone`, `gone_from`, `forget_prepared`, `has_sounds`, `sounds_in`,
`resources_summary`, `no_ai`, `set_no_ai`, `crash_reports_dir`, `split_args`, `game_args`,
`append_game_args`, `PrepareStep` with its statics and the four `set_prepare_step` calls of the
kept preparer. Added `not_prepared(tag)`, the sentence `need_prepared` uses; `installed_in` sorts
with `club::newer`. The preparation code stays in the library, its tests with it: `preparer`
(backend.rs:411), `sounds` (:577), `prepare_in_process` (:621), `java_tests` (:1076), `mojang.rs`,
`sources.rs`, `downloads.rs`.

**Kept files changed**: lib.rs, club.rs (`managed_file`, `online_dir`, `newer`; `MARKER` is
`CONFIG_FILE`), identity.rs (`IdentityError::Shared`, `message`, `is_shared`, `player`),
backend.rs, backend/{game_start,messages,settings,look_settings}.rs, proxy.rs (reads no setting
from `settings.json` any more: the window has no proxy to give it), server_list.rs (+ tests),
servers.rs (`can_host`'s two sentences, the game-files problem, and `create`, which read a
sentence as a signal: surprises), i18n.rs (+ tests), main.rs, launcher_setup.rs,
launcher_play.rs (rewritten: `selected`, `reload_club`, `refusal`, `no_build`, `play`,
`can_start`, `start_client`), launcher_messages.rs (rewritten), launcher_render.rs, startup.rs,
titlebar.rs (`options(title)`, `render(theme, title, window)`), frame.rs, blocks_chrome.rs,
blocks_play.rs, blocks_settings.rs (rewritten: the page, `about_rows`), blocks_versions.rs,
home.rs (rewritten), home_tabs.rs (rewritten: no Overview tab in either look; what Play needs is
said above the tabs), glass_home.rs (rewritten: no picker state, no Play menu), glass_home_lists.rs,
glass_home_tabs.rs, glass_settings.rs (SECTIONS 6 to 3), glass_versions.rs, glass_palette.rs,
glass_ui.rs, language_page.rs, multiplayer_page.rs (rewritten), themes_page.rs, while_playing.rs,
widgets.rs, shell.rs (`Action::Raised`), servers_page.rs (`Field::{ServerName, ServerAddress}`;
`Fix::Prepare` on a build without its files now opens Home, which says what the supervisor has
to do), console_page.rs, worlds_page.rs, Cargo.toml (`sys-locale` dropped), Cargo.lock,
lang/fr.json (57 keys: 23 for the strings this step adds or rewords, 34 for strings that remained
without French, spec section 11), tests/{hidden_processes,startup}.rs.

**Deliberately left for later steps** (where it lives now):
- Hosting, spec section 7: `servers::set_up_in` still copies a build into `server-runtime`
  (servers.rs:266); `game_files_ready` names 26.3 (servers.rs:228); `[hosting]` of `club.toml`
  is read and shown by nothing (ports, `enabled`, `eula_accepted`; `agree_eula` backend.rs:870);
  the announcement's writer (`online\<COMPUTERNAME>.json` every 10 seconds) doesn't exist:
  `online.rs` is the reader; the Servers page's problems are English `format!`s and its fix
  button still says "Prepare the game files" (glass_servers.rs:340, glass_server_dialog.rs:921,
  servers_page.rs:1411, 3166; servers.rs:361).
- The preparation stack, as asked: backend.rs:242 to :627 and its four modules, for the
  supervisor's tool. `sources.rs:353` still says "choose the folder where your launcher keeps
  Minecraft" to a reader that no longer exists.
- Languages, spec section 11: `Language::ALL` has 87 entries (i18n.rs:194) and 85 other tables
  are in `lang/`; the dropdown offers the club's language, English and French only
  (language_page.rs `SHIPPED`). The five strings only `i18n/tests.rs`'s fixtures use have no
  entry, and need none.
- `tests/startup.rs:56`: the ignored interactive test's fixture wrote `skip_opening` into
  `settings.json`; the setting is gone, so that launcher opens with the wordmark.
- `perf.rs` (the benchmark report), `specimen.rs` (the debug page), `game_app.rs` (macOS): as
  step 5 left them. `launcher/README.md` and `launcher/docs/` still describe the old launcher.
- `backend::is_prepared`'s first answer of every 5 seconds reads the prepared manifest (the
  list of about 22,000 files) on the window's thread, where `row_state` asked it at every frame
  before this step.
- A game built without the `managed` feature refuses `--managed` as an unknown option
  (`crates/horizon-client/src/game/runtime/launch_options.rs:352`): the club's build carries it.

**Surprises**:
- Run for real, `Config::parse` refused the spec's own example: `#[serde(rename_all =
  "lowercase")]` on `config::Language` wanted `french` and `english`, not `fr` and `en`. Fixed
  by naming the two variants (`#[serde(rename = "fr")]`). Without the scratch run the tests
  would have been wrong the first time anyone ran them.
- The Multiplayer tab had no ping left to keep: it died in step 1 with `telemetry::server_players`
  (it lived in the telemetry crate). "(keep its ping)" could only mean bringing it back, so
  `server_status.rs` does, and the launcher makes a network connection of its own again: one
  status ping to each server of its tab every 5 seconds, 3-second timeout, off the window's
  thread. Two limits: an announcement is admitted only when its `host` is a literal address of
  the local network (`online::local`: RFC 1918, RFC 4193, loopback, link-local), because
  `online\` is every student's to write and the ping and `servers.dat` would otherwise send the
  whole club wherever a planted file names; and there is no SRV lookup, as the managed game
  connects by the address as written. The club's own `[[servers]]` are the supervisor's and are
  pinged wherever they are. Drop `server_status.rs` and the tab's `reach` if the launcher is to
  make no connection at all.
- `servers::create` offered its fix by reading a sentence: `e.contains("prepared")`. The
  reworded `can_host` would have silently stopped offering "Prepare the game files"; it asks the
  build now (`backend::check_prepared`).
- `cargo fmt --check` was not clean when this step began: 64 hunks in 31 files, several in files
  this step never opened (`cull.rs`, `glass_servers.rs`, `windows_runtime.rs`, `sources.rs`).
  `cargo fmt` ran once on the crate; the 32 files it wrote as LF are CRLF again, and the tree is
  formatted.
- The looks have two icon sets: `blocks::icon` reads `icons/<name>.svg` (the pixel set),
  `glass_ui::icon` reads `icons/glass/<name>.svg`. The blocky Settings' Appearance heading takes
  `image`: `color-picker` exists in the glass set only.
- `Window::window_title()` answers an empty string on every real platform (the trait's default
  in gpui-fast 0.1.13): the launcher keeps the title it last set instead of asking the window.
- `first_play::write_missing` did not make the game home, which exists by then in a real Play
  (the server list makes it first) and did not in its test: it makes it.
- This map's section 8 lists `sys-locale` under keep: the window's language is the club's now.
- The game must read `acceptTextures` as "set and 0 means disabled" for the list to take no
  pack: it does (`crates/horizon-protocol/src/servers.rs:61`, `ServerEntry::from_tag`).
- Whole files this step rewrote were written with the editor's own tool, not a script; every
  file under `launcher/src`, `launcher/tests` and `lang/fr.json` is CRLF with no bare line feed.

### Step 6b (2026-11-23): hosting, the announcements, and the supervisor's tool

`cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings` and
`cargo fmt --check` all clean, no warnings, on Windows x64; the lock file followed the manifest
offline (clap 4 was already in it). Cumulative `git diff --shortstat -- launcher`
(steps 1-6b, uncommitted): 320 files, +5,963 / −131,167; step 6a ended at 311 files,
+3,860 / −129,734, so this step is about 9 tracked paths (+2,100 / −1,430), the four files it
adds untracked (`src/bin/club_admin.rs`, 727 lines, and `src/bin/club_admin/{edits,managed,report}.rs`,
1,534 lines together), `src/online.rs` untracked and rewritten, and the two `servers/remote*.rs`
deleted. The launcher's own tests were compiled and linted, not run in the workspace; the 26
unit tests of the five pure modules (`online`, `config`, `edits`, `managed`, `report`) ran for
real in a scratch crate outside the repository (`extern crate self as horizon_launcher` over
`#[path]` includes, as step 6a's scratch, plus a stub-`tr`ing `i18n` and `club`). That run caught
two bugs before they could ship (surprises, first two).

**Modules deleted**: `servers/remote.rs` + `servers/remote_tests.rs` (hosting over SSH). In
`backend.rs`: `MinecraftFrom` (the window prepares nothing; the tool always prepares from a
folder it has completed first), `eula_agreed` and `agree_eula` (the EULA file is written by
`horizon-server init --eula` when the club accepted it, never by the launcher).

**New modules**:
- `src/bin/club_admin.rs` and `src/bin/club_admin/{edits,managed,report}.rs`: the supervisor's
  tool, a second executable of the launcher crate (`[[bin]] name = "club-admin"`; `clap` with
  derive added, its line-continuation tests included). The club root: `--root`, else
  `HORIZON_CLUB_ROOT`, else the folder above the exe's own `admin\` folder, else `club::root`'s
  search; whatever it is, it is given back to the library as `HORIZON_CLUB_ROOT` before any
  folder is read, so every path the tool writes is the one the students' launchers see. Its
  messages go through `tr`/`trf` (the club's language, French by default).
- `online.rs` grew the writer the spec named: `Announcement::text()` (one JSON line, the spec's
  fields, order and spacing), `file_name(computer)` (a NetBIOS name made file-safe),
  `write(dir, …)` (temporary file beside, flushed, renamed) and `remove(dir)`.

**Hosting as implemented** (spec section 7; `servers.rs`, `servers_page.rs`, `glass_servers.rs`,
`glass_server_dialog.rs`):
- Nothing is copied into a `server-runtime`: `set_up`, `set_up_in`, `runtime_root`,
  `runtime_release`, `game_files`, `game_files_ready`, `copy_release`, `point` and the whole
  releases layout are gone. `horizon-server` runs in place: `servers::program(tag)` is
  `<root>\core\<tag>\horizon-server.exe`, and `run`/`run_streamed`/`run_managed`/`config`/
  `set_config`/`backups` all take the build's tag (the page passes `selected()`, the build in
  use). The engine and game files are found beside the supervisor, and `settings_args` also
  points `[engine] resources` at the build's prepared folder, plays offline
  (`server.online_mode=false`) and turns the web panel off (`panel.bind=`).
- The server's own files stay in `servers\<slug>\` of the launcher's data folder. A new server
  takes a port from `[hosting] first_port..=last_port` (`servers::free_port`, now
  range-bounded and `Option`): none free is one sentence through `trf` and no server. With
  `eula_accepted = false` the page says the supervisor has to accept Minecraft's EULA in
  `club.toml` (create refuses; a running server's EULA problem says the same) and nothing is
  created. `init` always passes `--eula --offline`.
- Deleted besides: the "Anyone with the address" choice (a hosted server is created
  friends-only; the Settings tab keeps the whitelist toggle, reworded "everyone on the club's
  network"), the "Check Minecraft accounts" toggle (there are no accounts), the Port settings
  row (`Field::Port`, `save_port`; a busy port is moved by the `Fix::PortTaken` button, which
  picks a free port of the club's range itself), the "Run in the background and at login" row
  and `service_installed`/`service_suffix` (a hosted server lives as long as its launcher
  window; `stop_on_quit` stops every server it started, service check gone), the Updates
  section (`updates_section`, `Event::UpdateCheck`, the update fields of `Status`, the
  "Updating: {step}" phase, `older_horizon`) , the web panel button and `Api::panel_link`, and
  "Add a server folder…" (`add_server_folder`/`add_existing`) in both looks: not in the keep
  list, and it existed to adopt folders this edition's students do not have. The blocky
  create dialog's EULA tick and the glass dialog's EULA foot are gone; the glass foot keeps the
  status row and Cancel/Create (`create_foot_row` lost its two EULA parameters, and its layout
  test with them).
- The page exists only when `[hosting] enabled`: `Page::in_top_bar(hosting)` filters it from
  the top bar and the palette (blocks_chrome.rs, frame.rs, glass_palette.rs), the sidebars hide
  the hosted-server rows, `go()` routes a stale Servers page to Home, and `reload_club` leaves
  the page when the club turns hosting off.
- The Servers page's English `format!` problems and notes that step 6a left are `tr`/`trf` now
  (the trash messages, "Couldn't run horizon-server", the busy port, "The game didn't start…",
  "horizon-server failed without saying why", "My server", the created-server note, the
  whitelist note, the world copy); `Theme.on_accent` went with its one reader, the EULA tick.

**The announcements as implemented** (spec section 6): while a server this launcher started
runs (`STARTED_HERE` and its status), `Launcher::announce` (servers_page.rs) rewrites
`<root>\online\<COMPUTERNAME>.json` every 10 seconds (`ANNOUNCE`; `FRESH` is 45), and takes it
away once none runs; `stop_on_quit` removes it before quitting. The poll of an open server also
polls the launcher's other running servers, so a second hosted server's stop is seen even while
another is open. The file holds `name` (the server's), `owner` (the player's name),
`host` (`servers::lan_address()`), `port` (`servers::port_of`, read from `server.toml`) and
`at`. A folder the student cannot write to is not an error: it is said once on the launcher's
own log (`ANNOUNCE_QUIET`, one sentence through `tr`), never in the window.

**club-admin's commands** (each prints what it did, in the club's language):
- `check`: the report of the share — `club.toml` and its errors; each build of `core\`: name,
  the Minecraft version it plays, prepared files, `dxcompiler.dll`, `horizon-data.bin`,
  `horizon-server.exe`; the build in use (and a `[versions] active` naming no build);
  `managed.txt` beside what `sync` would write; `app\current.txt` and its launcher; the
  bootstrap. Facts are gathered on the disk, verdicts decided by `report::verdicts` (pure,
  tested): each verdict that keeps a student from playing (club.toml unreadable, no build, the
  build unprepared, managed.txt missing or wrong, `app\current.txt` missing or pointing at
  nothing, no bootstrap) says what to run, and the exit code is 1; `dxcompiler.dll`,
  `horizon-data.bin` and `horizon-server.exe` missing are warnings only.
- `sync`: writes `<root>\managed.txt` from `club.toml` (temporary file + rename): `lan:true` or
  `lan:false`, and one `server:<address>` line per `[[servers]]` entry **whose host is not on
  the local network** (launcher-target.md section 3, matching `managed.rs`'s own semantics: a
  literal local address needs no line, a name is local when every address it resolves to is,
  and a name that resolves to nothing keeps its line for the game to answer at run time). The
  window never writes this file.
- `set-version <tag>`: refuses a tag `core\` does not hold, then edits the
  `[versions] active` line in place (the line, else the section, else a new section at the
  end), keeping comments and every other key; the edited line keeps its own gap and comment.
- `server add <name> <address>` / `server remove <name>`: the `[[servers]]` blocks, same text
  editing (add after the last block, remove the block whose `name` it is); a duplicate name is
  refused, and every answer is `Config::parse`-checked before it is written. Both say to run
  `sync`.
- `publish --launcher <exe> --version <X> [--bootstrap <exe>]`: refuses an `app\<X>` that
  exists (a running launcher cannot be replaced on a share), copies the launcher to
  `app\<X>\HorizonClubLauncher.exe`, writes `app\current.txt` atomically, and with
  `--bootstrap` copies the bootstrap to `<root>\HorizonClub.exe`.
- `install-core --from <folder> [--tag <tag>]`: copies the built game folder to
  `core\<tag>\` (tag defaults to the folder's name, an existing target refused), then writes
  the data file.
- `data-file [--tag <tag>]`: writes `horizon-data.bin` in the build's prepared folder
  (`horizon_platform::prepared_files::write_with_readers`, up to 8 readers), through the path
  the students' launchers use, skipping when the file is there and saying to prepare first when
  the files are not. A root given as a mapped drive is warned about (`WNetGetConnectionW`;
  `windows-sys` gains `Win32_NetworkManagement_WNet`), because the file remembers its folder's
  exact path.
- `prepare [--tag <tag>] (--minecraft-dir <dir> | --download) [--server-jar <file>]
  [--java-home <dir>]`: `--java-home` sets `JAVA_HOME` and the front of `PATH`;
  `--server-jar` copies the jar where the preparer checks it; `--download` runs
  `mojang::download_http` into the launcher data's `minecraft\` (the code the signed-in owner
  once used; the guard below covers it), then everything is the library's preparation
  (`backend::preparer` — `InProcess` via `prepare_in_process`, the version's own binary via
  the new `backend::run_preparer`, which streams its lines — then `backend::sounds`), then the
  data file. `backend::download_refused` now names `club-admin prepare --download` where the
  window once apologised for having no account.

**Where the download guard sits**: `downloads.rs`, the one place the tool's HTTP requests are
built (`Downloader::request`, at the top of its redirect loop, so every hop is checked; the
closure-based fetch paths of `mojang`/`sources` go through it too, and `Api`'s loopback agent
and the status ping never did). `guard(url)`: scheme `http`/`https` only; the host must be one
of Mojang's five (`piston-meta`, `piston-data`, `resources.download.minecraft.net`,
`libraries.minecraft.net`, `launchermeta.mojang.com`, case apart, `localhost` refused by name);
the host is resolved before any request and every address must be public — loopback, private
(RFC 1918/4193), link-local, unspecified, multicast, broadcast, documentation and 100.64/10
refused (v6's mapped v4 unwrapped); and every file keeps its size-and-SHA-1 check, which was
already there (`downloads::valid`, `mojang::fetch_file`). `Config.guard` (`Guard::Mojang`
default) is the one off-switch, for `downloads/tests.rs`'s own loopback server, which says so
in a comment; `host_allowed`, `address_allowed` and the refusal-before-resolution are unit
tested in `downloads.rs`'s `guard_tests` (compiled here, not run in the workspace).

**Kept files changed** besides the above: lib.rs (unchanged: `online` was already declared),
Cargo.toml (`[[bin]]`, `clap`, the `windows-sys` feature), Cargo.lock (offline), main.rs
(`Page::in_top_bar(hosting)`), frame.rs, blocks_chrome.rs, glass_palette.rs (the gate),
launcher_setup.rs (`go`'s guard), launcher_play.rs (`reload_club` leaves a gone page),
theme.rs (`on_accent` gone), downloads/tests.rs (`Guard::Off`), backend.rs (the paragraphs
above), and `lang/fr.json` (+81 keys: every string this step adds or rewords, including the
whole tool and its report; the reworded old keys stay for map step 10's sweep).
`servers_format.rs` needed nothing: its "Before updating to …" labels name horizon-server's own
backups, which stay.

**Deliberately left for later steps** (where it lives now):
- The stale `lang/fr.json` keys of the deleted hosting parts (the EULA tick wording, "Anyone
  with the address", the Tailscale/forwarding sentences, the update section, the background
  row): map step 10's lang sweep.
- `check`'s facts do not test the bootstrap's behaviour (another workstream owns it), only that
  the file is there; `publish --bootstrap` copies whatever it is given.
- The window's Servers page can still start a server whose build was switched in `club.toml`
  after it was created: its `server.toml` pins `[engine] resources` to the build it was made
  from, which keeps working while that build stays in `core\` (the spec's own rule).
- `servers::player_worlds` and the create dialog's world copy read the game home only; a
  student without a world sees "No worlds yet" — unchanged from before.

**Surprises**:
- `serde_json::to_string` writes no spaces; the spec's announcement (`{"name": "…", "port": 25565}`)
  has them. `Announcement::text` formats the line itself (strings escaped through
  `serde_json::Value`), and the scratch test pinned it — without that run the writer's file and
  the spec's example would have differed in every field.
- `set_active`'s first draft dropped the edited line's own comment gap (`active = "x" # …`
  instead of the file's nine spaces): the scratch test caught it, and `replace_value` now keeps
  the gap the comment sat behind.
- A bin target's `mod edits;` resolves to `src/bin/edits.rs` — which cargo would then build as
  its own binary. The tool's modules are declared with `#[path]` (the house style) and live in
  `src/bin/club_admin/`.
- `status.update_*` was how the old page said "Updating…" while the supervisor rolled a
  release; nothing in a club share can update in place any more, so the fields went with the
  section, and `phase_label`'s first arm with them.
- The guard's `Ipv4Addr::is_shared` (100.64/10) is unstable still; the range is spelled out by
  its octets. `cargo fmt` wrote LF into 13 files; they are CRLF again, and the four new files
  were converted once.

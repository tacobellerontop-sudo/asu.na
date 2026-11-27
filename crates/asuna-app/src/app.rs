//! App state and everything that is not drawing.

use crate::art::{self, ArtState};
use asuna_core::library::{self, ScanResult};
use asuna_core::store::{data_dir, CustomGame, StoreData};
use asuna_core::*;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Library,
    Store,
    Launchers,
    Settings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shelf {
    All,
    Installed,
    NotInstalled,
    Favorites,
    Recent,
    Hidden,
}

impl Shelf {
    pub const ALL: [Shelf; 6] = [Shelf::All, Shelf::Installed, Shelf::NotInstalled, Shelf::Favorites, Shelf::Recent, Shelf::Hidden];
    pub fn label(self) -> &'static str {
        match self {
            Shelf::All => "All games",
            Shelf::Installed => "Installed",
            Shelf::NotInstalled => "Ready to install",
            Shelf::Favorites => "Favorites",
            Shelf::Recent => "Recently played",
            Shelf::Hidden => "Hidden",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Title,
    Recent,
    Playtime,
    Size,
}

impl Sort {
    pub const ALL: [Sort; 4] = [Sort::Title, Sort::Recent, Sort::Playtime, Sort::Size];
    pub fn label(self) -> &'static str {
        match self {
            Sort::Title => "A to Z",
            Sort::Recent => "Recent",
            Sort::Playtime => "Most played",
            Sort::Size => "Largest",
        }
    }
}

pub struct Toast {
    pub id: u64,
    pub text: SharedString,
    pub error: bool,
}

pub struct AddGameForm {
    pub name: Entity<InputState>,
    pub path: Entity<InputState>,
    pub args: Entity<InputState>,
    pub error: Option<SharedString>,
}

pub struct AsunaApp {
    pub data: StoreData,
    store_path: PathBuf,
    scan_cache_path: PathBuf,
    art_dir: PathBuf,
    pub scan: ScanResult,
    pub games: Vec<Game>,
    pub scanning: bool,
    pub from_cache: bool,

    pub view: Page,
    pub shelf: Shelf,
    pub launcher_filter: Option<LauncherId>,
    pub sort: Sort,
    pub search: Entity<InputState>,
    pub query: String,

    pub selected: Option<String>,
    pub exe_candidates: Option<(String, Vec<PathBuf>)>,

    pub store_search: Entity<InputState>,
    pub store_results: Option<Vec<StoreResult>>,
    pub store_busy: bool,

    pub add_game: Option<AddGameForm>,
    pub toasts: Vec<Toast>,
    next_toast: u64,

    pub art: HashMap<String, ArtState>,
    art_queue: Vec<String>,
    art_running: bool,
    _subscriptions: Vec<Subscription>,
}

impl AsunaApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let dir = data_dir();
        let store_path = dir.join("asuna.json");
        let scan_cache_path = dir.join("scan-cache.json");
        let data = StoreData::load(&store_path);
        let cached = library::load_scan_cache(&scan_cache_path);

        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search your games"));
        let store_search = cx.new(|cx| InputState::new(window, cx).placeholder("Search Steam, Battle.net and Riot"));
        let subs = vec![
            cx.subscribe_in(&search, window, |this, state, ev: &InputEvent, _w, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.query = state.read(cx).value().to_string();
                    cx.notify();
                }
            }),
            cx.subscribe_in(&store_search, window, |this, state, ev: &InputEvent, _w, cx| {
                if let InputEvent::PressEnter { .. } = ev {
                    let q = state.read(cx).value().to_string();
                    this.run_store_search(q, cx);
                }
            }),
        ];

        let mut this = Self {
            store_path,
            scan_cache_path,
            art_dir: dir.join("art"),
            scan: cached.clone().unwrap_or_default(),
            games: vec![],
            scanning: false,
            from_cache: cached.is_some(),
            view: Page::Library,
            shelf: Shelf::All,
            launcher_filter: None,
            sort: Sort::Title,
            search,
            query: String::new(),
            selected: None,
            exe_candidates: None,
            store_search,
            store_results: None,
            store_busy: false,
            add_game: None,
            toasts: vec![],
            next_toast: 0,
            art: HashMap::new(),
            art_queue: vec![],
            art_running: false,
            _subscriptions: subs,
            data,
        };
        this.rebuild(cx);
        // The last scan shows instantly; a fresh one runs in the background.
        if cached.is_none() || this.data.settings.scan_on_startup {
            this.rescan(cx);
        }
        this
    }

    // ---- persistence ----

    pub fn save(&self, cx: &mut Context<Self>) {
        let data = self.data.clone();
        let path = self.store_path.clone();
        cx.background_spawn(async move {
            if let Err(e) = data.save(&path) {
                log::warn!("could not save settings: {e}");
            }
        })
        .detach();
    }

    /// Re-applies user state on top of the last scan.
    pub fn rebuild(&mut self, cx: &mut Context<Self>) {
        self.games = library::build(&self.scan, &self.data);
        self.queue_art(cx);
        cx.notify();
    }

    pub fn rescan(&mut self, cx: &mut Context<Self>) {
        if self.scanning {
            return;
        }
        self.scanning = true;
        cx.notify();
        let data = self.data.clone();
        let cache_path = self.scan_cache_path.clone();
        let task = cx.background_spawn(async move {
            let cache = Mutex::new(data.cache.clone());
            let result = library::scan(&data, &cache);
            library::save_scan_cache(&cache_path, &result);
            (result, cache.into_inner().unwrap_or_default())
        });
        cx.spawn(async move |this, cx| {
            let (result, cache): (ScanResult, HashMap<String, Value>) = task.await;
            let _ = this.update(cx, |this, cx| {
                this.scan = result;
                this.data.cache = cache;
                this.scanning = false;
                this.from_cache = false;
                this.save(cx);
                this.rebuild(cx);
            });
        })
        .detach();
    }

    // ---- toasts ----

    pub fn toast(&mut self, text: impl Into<SharedString>, error: bool, cx: &mut Context<Self>) {
        let id = self.next_toast;
        self.next_toast += 1;
        self.toasts.push(Toast { id, text: text.into(), error });
        cx.notify();
        let secs = if error { 6 } else { 3 };
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(secs)).await;
            let _ = this.update(cx, |this, cx| {
                this.toasts.retain(|t| t.id != id);
                cx.notify();
            });
        })
        .detach();
    }

    // ---- queries ----

    pub fn game(&self, key: &str) -> Option<&Game> {
        self.games.iter().find(|g| g.key == key)
    }

    /// Everything that belongs in the library (catalog-only titles live under "Get games").
    pub fn library_games(&self) -> impl Iterator<Item = &Game> {
        self.games.iter().filter(|g| !g.info.catalog_only || g.info.installed)
    }

    pub fn shelf_count(&self, shelf: Shelf) -> usize {
        self.library_games().filter(|g| Self::on_shelf(g, shelf)).count()
    }

    fn on_shelf(g: &Game, shelf: Shelf) -> bool {
        if (shelf == Shelf::Hidden) != g.hidden {
            return false;
        }
        match shelf {
            Shelf::All | Shelf::Hidden => true,
            Shelf::Installed => g.info.installed,
            Shelf::NotInstalled => !g.info.installed,
            Shelf::Favorites => g.favorite,
            Shelf::Recent => g.info.last_played.is_some(),
        }
    }

    pub fn visible_games(&self) -> Vec<&Game> {
        let q = self.query.trim().to_lowercase();
        let mut list: Vec<&Game> = self
            .library_games()
            .filter(|g| Self::on_shelf(g, self.shelf))
            .filter(|g| self.launcher_filter.is_none_or(|l| g.info.launcher == l))
            .filter(|g| q.is_empty() || g.info.title.to_lowercase().contains(&q))
            .collect();
        let sort = if self.shelf == Shelf::Recent { Sort::Recent } else { self.sort };
        match sort {
            Sort::Title => list.sort_by_key(|g| (!g.favorite || self.shelf != Shelf::All, g.info.title.to_lowercase())),
            Sort::Recent => list.sort_by_key(|g| std::cmp::Reverse(g.info.last_played.unwrap_or(0))),
            Sort::Playtime => list.sort_by_key(|g| std::cmp::Reverse(g.info.playtime_minutes.unwrap_or(0))),
            Sort::Size => list.sort_by_key(|g| std::cmp::Reverse(g.info.size_bytes.unwrap_or(0))),
        }
        list
    }

    pub fn continue_playing(&self) -> Vec<&Game> {
        let mut list: Vec<&Game> = self.library_games().filter(|g| g.info.installed && !g.hidden && g.info.last_played.is_some()).collect();
        list.sort_by_key(|g| std::cmp::Reverse(g.info.last_played.unwrap_or(0)));
        list.truncate(4);
        list
    }

    pub fn launcher_counts(&self) -> Vec<(LauncherId, usize)> {
        LauncherId::ALL
            .iter()
            .map(|l| (*l, self.library_games().filter(|g| !g.hidden && g.info.launcher == *l).count()))
            .filter(|(_, n)| *n > 0)
            .collect()
    }

    // ---- game actions ----

    pub fn run_game_action(&mut self, key: &str, action: GameAction, window: &mut Window, cx: &mut Context<Self>) {
        let Some(game) = self.game(key).cloned() else { return };
        let title = game.info.title.clone();
        let label = game.info.launcher.label();
        let result = match action {
            GameAction::OpenFolder => match &game.info.install_path {
                Some(p) => asuna_core::actions::open_folder(p),
                None => Err("No install folder known for this game".into()),
            },
            _ => {
                let a = match action {
                    GameAction::Launch => &game.actions.launch,
                    GameAction::Install => &game.actions.install,
                    GameAction::Uninstall => &game.actions.uninstall,
                    _ => &game.actions.store,
                };
                match a {
                    Some(a) => asuna_core::actions::run(a),
                    None => Err(format!("{title} can't do that from AsuNa")),
                }
            }
        };
        match result {
            Err(e) => self.toast(e, true, cx),
            Ok(()) => {
                let msg = match action {
                    GameAction::Launch if game.launch_mode == LaunchMode::Direct => format!("Starting {title} directly"),
                    GameAction::Launch => format!("Launching {title}"),
                    GameAction::Install => format!("Opening {label} to install {title}"),
                    GameAction::Uninstall => format!("Opening the uninstaller for {title}"),
                    GameAction::Store => format!("Opening {title} in {label}"),
                    GameAction::OpenFolder => format!("Opening the folder for {title}"),
                };
                self.toast(msg, false, cx);
                if action == GameAction::Launch {
                    self.data.last_played.insert(key.to_string(), library::now_ms());
                    self.save(cx);
                    self.rebuild(cx);
                    match self.data.settings.on_game_launch {
                        OnGameLaunch::Nothing => {}
                        OnGameLaunch::Minimize => window.minimize_window(),
                        OnGameLaunch::Quit => {
                            let _ = self.data.save(&self.store_path);
                            cx.quit();
                        }
                    }
                }
            }
        }
    }

    pub fn toggle_favorite(&mut self, key: &str, cx: &mut Context<Self>) {
        if !self.data.favorites.remove(key) {
            self.data.favorites.insert(key.to_string());
        }
        self.save(cx);
        self.rebuild(cx);
    }

    pub fn toggle_hidden(&mut self, key: &str, cx: &mut Context<Self>) {
        if !self.data.hidden.remove(key) {
            self.data.hidden.insert(key.to_string());
        }
        self.save(cx);
        self.rebuild(cx);
    }

    pub fn open_details(&mut self, key: &str, cx: &mut Context<Self>) {
        self.selected = Some(key.to_string());
        self.exe_candidates = None;
        cx.notify();
    }

    pub fn close_details(&mut self, cx: &mut Context<Self>) {
        self.selected = None;
        self.exe_candidates = None;
        cx.notify();
    }

    /// Switches a game between "through its launcher" and "run the game directly".
    pub fn set_launch_mode(&mut self, key: &str, mode: LaunchMode, cx: &mut Context<Self>) {
        let Some(g) = self.game(key) else { return };
        let exe = g.direct_executable.clone();
        if mode == LaunchMode::Direct && exe.is_none() {
            self.find_exe_candidates(key, cx);
            self.toast("Pick the game's program below first", false, cx);
            return;
        }
        let entry = self.data.launch_overrides.entry(key.to_string()).or_default();
        entry.mode = mode;
        if mode == LaunchMode::Launcher && entry.executable.is_none() && entry.args.is_none() {
            self.data.launch_overrides.remove(key);
        }
        self.save(cx);
        self.rebuild(cx);
    }

    pub fn set_direct_executable(&mut self, key: &str, exe: PathBuf, cx: &mut Context<Self>) {
        let entry = self.data.launch_overrides.entry(key.to_string()).or_default();
        entry.executable = Some(exe);
        entry.mode = LaunchMode::Direct;
        self.exe_candidates = None;
        self.save(cx);
        self.rebuild(cx);
    }

    pub fn find_exe_candidates(&mut self, key: &str, cx: &mut Context<Self>) {
        let Some(g) = self.game(key) else { return };
        let Some(dir) = g.info.install_path.clone() else {
            self.toast("AsuNa doesn't know where this game is installed. Use Browse instead.", true, cx);
            return;
        };
        let title = g.info.title.clone();
        let known = g.info.executable.clone();
        let key = key.to_string();
        let task = cx.background_spawn(async move {
            let mut found = asuna_core::executables::find_executables(&dir, &title, asuna_core::Platform::current());
            if let Some(k) = known.filter(|k| !found.contains(k)) {
                found.insert(0, k);
            }
            found
        });
        cx.spawn(async move |this, cx| {
            let found = task.await;
            let _ = this.update(cx, |this, cx| {
                if found.is_empty() {
                    this.toast("No programs found in the install folder. Use Browse instead.", true, cx);
                }
                this.exe_candidates = Some((key, found));
                cx.notify();
            });
        })
        .detach();
    }

    /// Native file picker. `then` receives the chosen path.
    pub fn pick_path(
        &mut self,
        directories: bool,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, PathBuf, &mut Context<Self>) + 'static,
    ) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: !directories,
            directories,
            multiple: false,
            prompt: Some(if directories { "Choose folder" } else { "Choose program" }.into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(mut paths))) = rx.await {
                if let Some(p) = paths.pop() {
                    let _ = this.update(cx, |this, cx| then(this, p, cx));
                }
            }
        })
        .detach();
    }

    // ---- custom games ----

    pub fn open_add_game(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("My Game"));
        let path = cx.new(|cx| InputState::new(window, cx).placeholder(if cfg!(windows) { r"C:\Games\MyGame\game.exe" } else { "/path/to/game" }));
        let args = cx.new(|cx| InputState::new(window, cx).placeholder("-windowed"));
        self.add_game = Some(AddGameForm { name, path, args, error: None });
        cx.notify();
    }

    pub fn submit_add_game(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &mut self.add_game else { return };
        let title = form.name.read(cx).value().trim().to_string();
        let exe = form.path.read(cx).value().trim().to_string();
        let args = form.args.read(cx).value().trim().to_string();
        if title.is_empty() || exe.is_empty() {
            form.error = Some("Name and program are both needed.".into());
            cx.notify();
            return;
        }
        self.data.custom_games.push(CustomGame {
            id: format!("{:x}", library::now_ms()),
            title,
            executable: PathBuf::from(exe),
            args: (!args.is_empty()).then_some(args),
        });
        self.add_game = None;
        self.save(cx);
        self.rebuild(cx);
        self.toast("Game added to your library", false, cx);
    }

    pub fn remove_custom(&mut self, key: &str, cx: &mut Context<Self>) {
        let id = key.trim_start_matches("custom:");
        self.data.custom_games.retain(|c| c.id != id);
        self.selected = None;
        self.save(cx);
        self.rebuild(cx);
    }

    // ---- store ----

    pub fn run_store_search(&mut self, term: String, cx: &mut Context<Self>) {
        let q = term.trim().to_lowercase();
        if q.is_empty() {
            return;
        }
        self.store_busy = true;
        cx.notify();
        let online = self.data.settings.online_lookups;
        let task = cx.background_spawn(async move { search_store(&q, online) });
        cx.spawn(async move |this, cx| {
            let results = task.await;
            let _ = this.update(cx, |this, cx| {
                this.store_results = Some(results);
                this.store_busy = false;
                this.queue_art(cx);
                cx.notify();
            });
        })
        .detach();
    }

    pub fn install_from_store(&mut self, r: &StoreResult, cx: &mut Context<Self>) {
        let key = format!("{}:{}", r.launcher.as_str(), r.id);
        let action = match r.launcher {
            LauncherId::Steam => asuna_core::providers::steam::actions(&r.id).install,
            LauncherId::Battlenet => Some(LaunchAction::uri(format!("battlenet://{}", r.id))),
            _ => self.game(&key).and_then(|g| g.actions.install.clone()),
        };
        let result = match action {
            Some(a) => asuna_core::actions::run(&a),
            None => Err(format!("Install {} first (Launchers page)", r.launcher.label())),
        };
        match result {
            Ok(()) => self.toast(format!("Opening {} for {}", r.launcher.label(), r.title), false, cx),
            Err(e) => self.toast(e, true, cx),
        }
    }

    // ---- launchers & settings ----

    pub fn set_launcher_enabled(&mut self, id: LauncherId, on: bool, cx: &mut Context<Self>) {
        if on {
            self.data.disabled_launchers.remove(&id);
        } else {
            self.data.disabled_launchers.insert(id);
        }
        self.save(cx);
        self.rescan(cx);
    }

    pub fn open_launcher(&mut self, id: LauncherId, cx: &mut Context<Self>) {
        let status = self.scan.launchers.iter().find(|l| l.id == id);
        if let Some(a) = library::open_launcher_action(id, status) {
            if let Err(e) = asuna_core::actions::run(&a) {
                self.toast(e, true, cx);
            }
        }
    }

    pub fn get_launcher(&mut self, id: LauncherId, cx: &mut Context<Self>) {
        let url = self.scan.launchers.iter().find(|l| l.id == id).and_then(|l| l.download_url.clone());
        if let Some(u) = url {
            if let Err(e) = asuna_core::actions::open_url(&u) {
                self.toast(e, true, cx);
            }
        }
    }

    pub fn update_settings(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut Settings)) {
        let before = self.data.settings.clone();
        f(&mut self.data.settings);
        let after = self.data.settings.clone();
        if before.open_at_login != after.open_at_login {
            if let Err(e) = asuna_core::autostart::set(after.open_at_login) {
                self.data.settings.open_at_login = before.open_at_login;
                self.toast(format!("Couldn't change start on login: {e}"), true, cx);
            }
        }
        self.save(cx);
        if before.path_overrides != after.path_overrides || before.online_lookups != after.online_lookups && after.online_lookups {
            self.rescan(cx);
        }
        if before.show_art != after.show_art {
            self.queue_art(cx);
        }
        cx.notify();
    }

    pub fn data_folder(&self) -> PathBuf {
        data_dir()
    }

    // ---- art ----

    pub fn art_for(&self, url: Option<&String>) -> Option<&ArtState> {
        url.and_then(|u| self.art.get(u))
    }

    /// Queues downloads for every art URL we don't have yet.
    fn queue_art(&mut self, cx: &mut Context<Self>) {
        if !self.data.settings.show_art {
            return;
        }
        let mut wanted: Vec<String> = Vec::new();
        let mut seen = HashSet::new();
        // Wide banners and logos are big, so only fetch them for the games the home page features.
        let featured: Vec<String> = self
            .continue_playing()
            .iter()
            .flat_map(|g| [g.info.art.hero.clone(), g.info.art.logo.clone()])
            .flatten()
            .collect();
        let urls = self
            .games
            .iter()
            .flat_map(|g| [g.info.art.cover.clone(), g.info.art.header.clone()])
            .chain(featured.into_iter().map(Some))
            .chain(self.store_results.iter().flatten().map(|r| r.image.clone()))
            .flatten();
        for url in urls {
            if self.art.contains_key(&url) || !seen.insert(url.clone()) {
                continue;
            }
            let path = art::cache_path(&self.art_dir, &url);
            if path.exists() {
                self.art.insert(url, ArtState::Ready(path));
            } else if art::miss_path(&self.art_dir, &url).exists() {
                self.art.insert(url, ArtState::Missing);
            } else if self.data.settings.online_lookups {
                self.art.insert(url.clone(), ArtState::Loading);
                wanted.push(url);
            }
        }
        self.art_queue.extend(wanted);
        self.pump_art(cx);
    }

    /// Downloads queued art in small parallel batches, redrawing as each batch lands.
    fn pump_art(&mut self, cx: &mut Context<Self>) {
        if self.art_running || self.art_queue.is_empty() {
            return;
        }
        self.art_running = true;
        let batch: Vec<String> = self.art_queue.drain(..self.art_queue.len().min(8)).collect();
        let dir = self.art_dir.clone();
        let task = cx.background_spawn(async move {
            std::thread::scope(|s| {
                let handles: Vec<_> = batch.iter().map(|u| s.spawn(|| (u.clone(), art::fetch(&dir, u)))).collect();
                handles.into_iter().filter_map(|h| h.join().ok()).collect::<Vec<_>>()
            })
        });
        cx.spawn(async move |this, cx| {
            let results = task.await;
            let _ = this.update(cx, |this, cx| {
                for (url, path) in results {
                    this.art.insert(url, path.map(ArtState::Ready).unwrap_or(ArtState::Missing));
                }
                this.art_running = false;
                cx.notify();
                this.pump_art(cx);
            });
        })
        .detach();
    }
}

fn search_store(q: &str, online: bool) -> Vec<StoreResult> {
    use asuna_core::providers::{battlenet, riot};
    let mut out: Vec<StoreResult> = battlenet::CATALOG
        .iter()
        .filter(|(_, t, _)| t.to_lowercase().contains(q))
        .map(|(code, t, _)| StoreResult { launcher: LauncherId::Battlenet, id: code.to_string(), title: t.to_string(), image: None, price: Some("Battle.net".into()) })
        .chain(riot::CATALOG.iter().filter(|(_, t)| t.to_lowercase().contains(q)).map(|(p, t)| StoreResult {
            launcher: LauncherId::Riot,
            id: p.to_string(),
            title: t.to_string(),
            image: None,
            price: Some("Free".into()),
        }))
        .collect();
    if !online {
        return out;
    }
    let url = format!("https://store.steampowered.com/api/storesearch/?term={}&l=english&cc=US", encode(q));
    if let Ok(json) = asuna_core::net::get_json(&url) {
        for it in json["items"].as_array().into_iter().flatten() {
            let Some(id) = it["id"].as_u64() else { continue };
            let price = match it["price"]["final"].as_u64() {
                Some(cents) => format!("{:.2} {}", cents as f64 / 100.0, it["price"]["currency"].as_str().unwrap_or("")),
                None => "Free".into(),
            };
            out.push(StoreResult {
                launcher: LauncherId::Steam,
                id: id.to_string(),
                title: it["name"].as_str().unwrap_or("").to_string(),
                image: Some(format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{id}/header.jpg")),
                price: Some(price),
            });
        }
    }
    out
}

fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub fn format_bytes(n: Option<u64>) -> Option<String> {
    let mut n = n? as f64;
    if n <= 0.0 {
        return None;
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut i = 0;
    while n >= 1024.0 && i < units.len() - 1 {
        n /= 1024.0;
        i += 1;
    }
    Some(if n >= 10.0 || i == 0 { format!("{n:.0} {}", units[i]) } else { format!("{n:.1} {}", units[i]) })
}

pub fn format_playtime(m: Option<u64>) -> Option<String> {
    let m = m?;
    Some(if m < 60 { format!("{m} min") } else if m < 600 { format!("{:.1} h", m as f64 / 60.0) } else { format!("{} h", m / 60) })
}

pub fn format_ago(ts: Option<i64>) -> Option<String> {
    let days = (library::now_ms() - ts?) / 86_400_000;
    Some(match days {
        d if d <= 0 => "Today".into(),
        1 => "Yesterday".into(),
        d if d < 30 => format!("{d} days ago"),
        d if d < 365 => format!("{} months ago", d / 30),
        d => format!("{} years ago", d / 365),
    })
}

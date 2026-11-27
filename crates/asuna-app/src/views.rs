//! Everything that draws. State changes go through methods in app.rs.

use crate::app::*;
use crate::art::ArtState;
use crate::theme::*;
use asuna_core::*;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{Disableable, Sizable, TitleBar};
use gpui_kit::prelude::*;
use gpui_kit::*;

const CARD_W: f32 = 158.;

fn sid(s: impl Into<String>) -> ElementId {
    ElementId::Name(SharedString::from(s.into()))
}

fn accent_gradient() -> Background {
    linear_gradient(135., linear_color_stop(c(ACCENT), 0.), linear_color_stop(c(ACCENT_2), 1.))
}

fn launcher_tint(l: LauncherId, alpha: u8) -> Rgba {
    ca(l.color(), alpha)
}

fn dot(l: LauncherId) -> Div {
    div().size(px(8.)).rounded_full().flex_none().bg(c(l.color()))
}

fn heading(text: impl Into<SharedString>) -> Div {
    div().text_size(px(15.)).font_weight(FontWeight::SEMIBOLD).text_color(c(TEXT)).mt_6().mb_3().child(text.into())
}

fn muted(text: impl Into<SharedString>) -> Div {
    div().text_color(c(MUTED)).child(text.into())
}

fn empty_box(text: impl Into<SharedString>) -> Div {
    div()
        .p_10()
        .rounded(px(10.))
        .border_1()
        .border_dashed()
        .border_color(c(LINE))
        .text_color(c(MUTED))
        .flex()
        .justify_center()
        .child(text.into())
}

fn one_line(d: Div) -> Div {
    d.overflow_hidden().whitespace_nowrap().text_ellipsis()
}

impl Render for AsunaApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.view {
            Page::Library => self.render_library(cx).into_any_element(),
            Page::Store => self.render_store(cx).into_any_element(),
            Page::Launchers => self.render_launchers(cx).into_any_element(),
            Page::Settings => self.render_settings(cx).into_any_element(),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(c(BG))
            .text_color(c(TEXT))
            .text_size(px(14.))
            .font_family(".SystemUIFont")
            .child(self.render_title_bar(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.render_sidebar(window, cx))
                    .child(div().id("main").flex_1().min_w_0().overflow_y_scroll().px_7().pb_16().child(body)),
            )
            .when_some(self.selected.clone(), |d, key| d.child(self.render_details(&key, cx)))
            .when(self.add_game.is_some(), |d| d.child(self.render_add_game(cx)))
            .child(self.render_toasts())
    }
}

impl AsunaApp {
    // ---------- title bar ----------

    fn render_title_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let status = if self.scanning {
            "Scanning launchers…".to_string()
        } else {
            let installed = self.games.iter().filter(|g| g.info.installed).count();
            format!("{installed} installed · {} in library", self.library_games().count())
        };
        TitleBar::new().bg(c(PANEL)).border_b_1().border_color(c(LINE)).child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .flex_1()
                .pr_2()
                .child(
                    div()
                        .size(px(20.))
                        .rounded(px(6.))
                        .bg(accent_gradient())
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(12.))
                        .font_weight(FontWeight::EXTRA_BOLD)
                        .text_color(gpui_kit::white())
                        .child("A"),
                )
                .child(div().font_weight(FontWeight::SEMIBOLD).child("AsuNa"))
                .child(div().text_size(px(12.)).text_color(c(MUTED)).child(status))
                .child(div().flex_1())
                .child(
                    Button::new("tb-rescan")
                        .ghost()
                        .xsmall()
                        .label(if self.scanning { "Scanning…" } else { "Rescan" })
                        .disabled(self.scanning)
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.rescan(cx))),
                ),
        )
    }

    // ---------- sidebar ----------

    fn render_sidebar(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let nav = |id: &'static str, label: &'static str, view: Page, cx: &mut Context<Self>| {
            let active = self.view == view;
            div()
                .id(id)
                .px_3()
                .py_2()
                .rounded(px(8.))
                .text_size(px(15.))
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .text_color(c(if active { TEXT } else { MUTED }))
                .when(active, |d| d.bg(c(PANEL_2)))
                .hover(|s| s.text_color(c(TEXT)))
                .child(label)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.view = view;
                    cx.notify();
                }))
        };
        let row = |id: ElementId, label: AnyElement, count: Option<usize>, active: bool| {
            div()
                .id(id)
                .flex()
                .items_center()
                .justify_between()
                .px_3()
                .py(px(7.))
                .rounded(px(8.))
                .cursor_pointer()
                .text_color(c(if active { TEXT } else { MUTED }))
                .when(active, |d| d.bg(c(PANEL_2)))
                .hover(|s| s.text_color(c(TEXT)))
                .child(label)
                .when_some(count, |d, n| d.child(div().text_size(px(12.)).text_color(c(MUTED)).child(n.to_string())))
        };
        let side_label = |t: &'static str| div().px_3().pt_4().pb_1().text_size(px(11.)).text_color(c(MUTED)).child(t.to_uppercase());

        let mut col = div()
            .id("sidebar")
            .w(px(232.))
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(2.))
            .p_3()
            .bg(c(PANEL))
            .border_r_1()
            .border_color(c(LINE))
            .overflow_y_scroll()
            .child(nav("nav-lib", "Library", Page::Library, cx))
            .child(nav("nav-store", "Get games", Page::Store, cx))
            .child(nav("nav-launchers", "Launchers", Page::Launchers, cx))
            .child(nav("nav-settings", "Settings", Page::Settings, cx));

        if self.view == Page::Library {
            col = col.child(side_label("Shelves"));
            for shelf in Shelf::ALL {
                let active = self.shelf == shelf;
                col = col.child(
                    row(sid(format!("shelf-{shelf:?}")), div().child(shelf.label()).into_any_element(), Some(self.shelf_count(shelf)), active).on_click(
                        cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.shelf = shelf;
                            cx.notify();
                        }),
                    ),
                );
            }
            col = col.child(side_label("Launchers")).child(
                row(sid("lf-any"), div().child("Every launcher").into_any_element(), None, self.launcher_filter.is_none())
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.launcher_filter = None;
                        cx.notify();
                    })),
            );
            for (l, n) in self.launcher_counts() {
                let label = div().flex().items_center().gap_2().child(dot(l)).child(l.label()).into_any_element();
                col = col.child(row(sid(format!("lf-{}", l.as_str())), label, Some(n), self.launcher_filter == Some(l)).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.launcher_filter = Some(l);
                        cx.notify();
                    }),
                ));
            }
        }

        col.child(div().flex_1().min_h_4()).child(
            Button::new("add-game")
                .ghost()
                .small()
                .label("+ Add a game")
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.open_add_game(window, cx))),
        )
    }

    // ---------- library ----------

    fn render_library(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut sort_row = div().flex().gap_1();
        for s in Sort::ALL {
            let active = self.sort == s && self.shelf != Shelf::Recent;
            sort_row = sort_row.child(
                Button::new(sid(format!("sort-{s:?}")))
                    .small()
                    .label(s.label())
                    .when(active, |b| b.primary())
                    .when(!active, |b| b.ghost())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.sort = s;
                        cx.notify();
                    })),
            );
        }
        let toolbar = div()
            .flex()
            .items_center()
            .gap_3()
            .pt_5()
            .child(div().w(px(380.)).child(Input::new(&self.search).cleanable(true)))
            .child(sort_row);

        let mut page = div().child(toolbar);

        let recent = self.continue_playing();
        if self.shelf == Shelf::All && self.launcher_filter.is_none() && self.query.is_empty() && !recent.is_empty() {
            let mut row = div().flex().gap_4();
            for g in recent {
                row = row.child(self.render_hero(g, cx));
            }
            page = page.child(heading("Jump back in")).child(row);
        }

        let mut title = self.shelf.label().to_string();
        if let Some(l) = self.launcher_filter {
            title = format!("{title} · {}", l.label());
        }
        let games = self.visible_games();
        page = page.child(heading(title));
        if self.games.is_empty() && self.scanning {
            page.child(empty_box("Looking for games across your launchers…"))
        } else if games.is_empty() {
            page.child(empty_box(if self.library_games().count() == 0 {
                "No games found yet. Check the Launchers page, or add a game yourself."
            } else {
                "Nothing matches this filter."
            }))
        } else {
            let mut grid = div().flex().flex_wrap().gap_x_4().gap_y_5();
            for g in games {
                grid = grid.child(self.render_card(g, cx));
            }
            page.child(grid)
        }
    }

    fn art_image(&self, url: Option<&String>) -> Option<PathBuf> {
        if !self.data.settings.show_art {
            return None;
        }
        match self.art_for(url) {
            Some(ArtState::Ready(p)) => Some(p.clone()),
            _ => None,
        }
    }

    /// Box art with a fallback chain: cover, then header, then a generated tile.
    fn render_cover(&self, g: &Game, large: bool) -> AnyElement {
        let cover = self.art_image(g.info.art.cover.as_ref());
        let header = self.art_image(g.info.art.header.as_ref());
        let dim = !g.info.installed;
        if let Some(p) = cover.or(header) {
            return img(p)
                .size_full()
                .object_fit(ObjectFit::Cover)
                .when(dim, |i| i.grayscale(true).opacity(0.65))
                .into_any_element();
        }
        let initials: String = g
            .info
            .title
            .split_whitespace()
            .filter_map(|w| w.chars().find(|c| c.is_alphanumeric()))
            .take(2)
            .collect::<String>()
            .to_uppercase();
        div()
            .size_full()
            .p_3()
            .flex()
            .flex_col()
            .justify_between()
            .bg(linear_gradient(
                150.,
                linear_color_stop(launcher_tint(g.info.launcher, if dim { 0x40 } else { 0x88 }), 0.),
                linear_color_stop(c(PANEL), 0.75),
            ))
            .child(div().text_size(px(if large { 44. } else { 32. })).font_weight(FontWeight::EXTRA_BOLD).child(initials))
            .when(!large, |d| d.child(div().text_size(px(13.)).font_weight(FontWeight::SEMIBOLD).child(g.info.title.clone())))
            .into_any_element()
    }

    fn render_card(&self, g: &Game, cx: &mut Context<Self>) -> impl IntoElement {
        let key = g.key.clone();
        let key2 = g.key.clone();
        let installed = g.info.installed;
        let group = SharedString::from(format!("card-{}", g.key));
        div()
            .id(sid(format!("card-{}", g.key)))
            .group(group.clone())
            .w(px(CARD_W))
            .flex_none()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.open_details(&key, cx)))
            .child(
                div()
                    .relative()
                    .w(px(CARD_W))
                    .h(px(CARD_W * 1.5))
                    .rounded(px(10.))
                    .overflow_hidden()
                    .bg(c(PANEL_2))
                    .shadow_md()
                    .child(self.render_cover(g, false))
                    .when(g.favorite, |d| d.child(div().absolute().top_2().right_2().text_color(c(FAV)).child("★")))
                    .child(
                        div()
                            .absolute()
                            .bottom_0()
                            .left_0()
                            .right_0()
                            .p_2()
                            .invisible()
                            .group_hover(group, |s| s.visible())
                            .child(
                                Button::new(sid(format!("play-{}", g.key)))
                                    .w_full()
                                    .small()
                                    .label(if installed { "▶  Play" } else { "↓  Install" })
                                    .when(installed, |b| b.primary())
                                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                        cx.stop_propagation();
                                        let action = if installed { GameAction::Launch } else { GameAction::Install };
                                        this.run_game_action(&key2, action, window, cx);
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .pt_2()
                    .child(one_line(div().font_weight(FontWeight::SEMIBOLD)).child(g.info.title.clone()))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_size(px(12.))
                            .text_color(c(MUTED))
                            .child(dot(g.info.launcher))
                            .child(g.info.launcher.short())
                            .when(!installed, |d| d.child(tag("Not installed")))
                            .when(g.launch_mode == LaunchMode::Direct, |d| d.child(tag("Direct"))),
                    ),
            )
    }

    fn render_hero(&self, g: &Game, cx: &mut Context<Self>) -> impl IntoElement {
        let key = g.key.clone();
        let key2 = g.key.clone();
        let header = self.art_image(g.info.art.header.as_ref());
        div()
            .id(sid(format!("hero-{}", g.key)))
            .relative()
            .flex_1()
            .max_w(px(320.))
            .h(px(150.))
            .rounded(px(10.))
            .overflow_hidden()
            .cursor_pointer()
            .bg(linear_gradient(150., linear_color_stop(launcher_tint(g.info.launcher, 0x77), 0.), linear_color_stop(c(PANEL), 1.)))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.open_details(&key, cx)))
            .when_some(header, |d, p| d.child(img(p).size_full().object_fit(ObjectFit::Cover)))
            .child(
                div()
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .px_3()
                    .py_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(linear_gradient(180., linear_color_stop(ca(0, 0), 0.), linear_color_stop(ca(0, 0xd0), 1.)))
                    .child(one_line(div().font_weight(FontWeight::SEMIBOLD)).child(g.info.title.clone()))
                    .child(
                        Button::new(sid(format!("hero-play-{}", g.key)))
                            .xsmall()
                            .label("▶ Play")
                            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                cx.stop_propagation();
                                this.run_game_action(&key2, GameAction::Launch, window, cx);
                            })),
                    ),
            )
    }

    // ---------- details drawer ----------

    fn render_details(&self, key: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(g) = self.game(key) else { return div().into_any_element() };
        let k = |s: &str| SharedString::from(format!("{s}-{key}"));
        let key_s = key.to_string();
        let l = g.info.launcher;

        let facts: Vec<(&str, Option<String>)> = vec![
            ("Status", Some(if g.info.installed { "Installed".into() } else { "Not installed".into() })),
            ("Size on disk", format_bytes(g.info.size_bytes)),
            ("Play time", format_playtime(g.info.playtime_minutes)),
            ("Last played", format_ago(g.info.last_played)),
            ("Folder", g.info.install_path.as_ref().map(|p| p.display().to_string())),
        ];
        let mut fact_list = div().flex().flex_col().gap_2().my_5();
        for (name, v) in facts {
            if let Some(v) = v {
                fact_list = fact_list.child(
                    div().flex().gap_3().child(div().w(px(110.)).flex_none().text_color(c(MUTED)).child(name)).child(one_line(div().flex_1().min_w_0()).child(v)),
                );
            }
        }

        let action_btn = |id: &str, label: String, action: GameAction, cx: &mut Context<Self>| {
            let key = key_s.clone();
            Button::new(sid(format!("{id}-{key_s}")))
                .small()
                .label(label)
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| this.run_game_action(&key, action, window, cx)))
        };

        let mut links = div().flex().flex_wrap().gap_2();
        if g.info.installed && g.info.install_path.is_some() {
            links = links.child(action_btn("folder", "Open folder".into(), GameAction::OpenFolder, cx));
        }
        if g.actions.store.is_some() {
            links = links.child(action_btn("store", format!("Open in {}", l.short()), GameAction::Store, cx));
        }
        if g.info.installed && g.actions.uninstall.is_some() {
            links = links.child(action_btn("uninstall", "Uninstall".into(), GameAction::Uninstall, cx));
        }
        let kh = key_s.clone();
        links = links.child(
            Button::new(sid(format!("hide-{key_s}")))
                .small()
                .label(if g.hidden { "Show in library" } else { "Hide from library" })
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.toggle_hidden(&kh, cx))),
        );
        if l == LauncherId::Custom {
            let kr = key_s.clone();
            links = links.child(
                Button::new(sid(format!("remove-{key_s}")))
                    .small()
                    .danger()
                    .label("Remove from AsuNa")
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.remove_custom(&kr, cx))),
            );
        }

        let primary = {
            let key = key_s.clone();
            let installed = g.info.installed;
            Button::new(k("primary"))
                .primary()
                .large()
                .flex_1()
                .label(if installed {
                    if g.launch_mode == LaunchMode::Direct { "▶  Play (direct)".to_string() } else { "▶  Play".to_string() }
                } else {
                    format!("↓  Install with {}", l.short())
                })
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    let a = if installed { GameAction::Launch } else { GameAction::Install };
                    this.run_game_action(&key, a, window, cx)
                }))
        };
        let kf = key_s.clone();
        let fav = Button::new(k("fav"))
            .large()
            .label(if g.favorite { "★" } else { "☆" })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.toggle_favorite(&kf, cx)));

        let drawer = div()
            .id("drawer")
            .w(px(440.))
            .h_full()
            .bg(c(PANEL))
            .border_l_1()
            .border_color(c(LINE))
            .overflow_y_scroll()
            .occlude()
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(div().h(px(280.)).relative().overflow_hidden().child(self.render_cover(g, true)).child(
                div().absolute().bottom_0().left_0().right_0().h(px(120.)).bg(linear_gradient(
                    180.,
                    linear_color_stop(ca(PANEL, 0), 0.),
                    linear_color_stop(c(PANEL), 1.),
                )),
            ))
            .child(
                div()
                    .px_6()
                    .pb_6()
                    .mt(px(-30.))
                    .relative()
                    .child(
                        div()
                            .flex()
                            .child(
                                div()
                                    .px_3()
                                    .py(px(3.))
                                    .rounded_full()
                                    .text_size(px(12.))
                                    .bg(launcher_tint(l, 0x33))
                                    .border_1()
                                    .border_color(launcher_tint(l, 0x66))
                                    .text_color(c(l.color()))
                                    .child(l.label()),
                            ),
                    )
                    .child(div().text_size(px(26.)).font_weight(FontWeight::BOLD).mt_2().mb_4().child(g.info.title.clone()))
                    .child(div().flex().gap_2().child(primary).child(fav))
                    .child(fact_list)
                    .child(links)
                    .when(g.info.installed && l != LauncherId::Custom, |d| d.child(self.render_launch_options(g, cx))),
            );

        div()
            .id("drawer-backdrop")
            .absolute()
            .inset_0()
            .bg(ca(0, 0x90))
            .flex()
            .justify_end()
            .occlude()
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.close_details(cx)))
            .child(drawer)
            .into_any_element()
    }

    /// "Through the launcher" vs "run the game directly".
    fn render_launch_options(&self, g: &Game, cx: &mut Context<Self>) -> impl IntoElement {
        let key = g.key.clone();
        let l = g.info.launcher;
        let mode_btn = |id: &str, label: String, mode: LaunchMode, cx: &mut Context<Self>| {
            let key = key.clone();
            let active = g.launch_mode == mode;
            Button::new(sid(format!("{id}-{}", g.key)))
                .small()
                .label(label)
                .when(active, |b| b.primary())
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.set_launch_mode(&key, mode, cx)))
        };
        let warning = match l {
            LauncherId::Steam => Some("Most Steam games need Steam running for their DRM and will open it anyway. DRM-free ones start on their own."),
            LauncherId::Ea | LauncherId::Ubisoft | LauncherId::Battlenet | LauncherId::Riot => {
                Some("Games from this launcher usually need it running for sign-in and anti-cheat, so starting directly may fail.")
            }
            LauncherId::Xbox => Some("Xbox / Game Pass games are protected and normally can't be started directly."),
            _ => None,
        };
        let exe_line = g
            .direct_executable
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "No program chosen yet".into());

        let k1 = key.clone();
        let k2 = key.clone();
        let mut section = div()
            .mt_6()
            .pt_5()
            .border_t_1()
            .border_color(c(LINE))
            .flex()
            .flex_col()
            .gap_3()
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Launch options"))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(mode_btn("mode-l", format!("Through {}", l.short()), LaunchMode::Launcher, cx))
                    .child(mode_btn("mode-d", "Run the game directly".into(), LaunchMode::Direct, cx)),
            )
            .when_some(warning, |d, w| d.child(div().text_size(px(12.)).text_color(c(MUTED)).child(w)))
            .child(one_line(div().text_size(px(12.)).text_color(c(MUTED))).child(exe_line))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new(sid(format!("find-exe-{key}")))
                            .small()
                            .label("Find the game's program")
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.find_exe_candidates(&k1, cx))),
                    )
                    .child(Button::new(sid(format!("browse-exe-{key}"))).small().label("Browse…").on_click(cx.listener(
                        move |this, _: &ClickEvent, _, cx| {
                            let key = k2.clone();
                            this.pick_path(false, cx, move |this, p, cx| this.set_direct_executable(&key, p, cx));
                        },
                    ))),
            );

        if let Some((ck, list)) = &self.exe_candidates {
            if *ck == key && !list.is_empty() {
                let root = g.info.install_path.clone();
                let mut picks = div().flex().flex_col().gap_1();
                for (i, p) in list.iter().enumerate() {
                    let shown = root.as_ref().and_then(|r| p.strip_prefix(r).ok()).unwrap_or(p).display().to_string();
                    let key = key.clone();
                    let path = p.clone();
                    picks = picks.child(
                        div()
                            .id(sid(format!("exe-{i}-{key}")))
                            .px_3()
                            .py_2()
                            .rounded(px(8.))
                            .bg(c(PANEL_2))
                            .cursor_pointer()
                            .hover(|s| s.bg(c(HOVER)))
                            .text_size(px(13.))
                            .child(shown)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.set_direct_executable(&key, path.clone(), cx))),
                    );
                }
                section = section.child(div().text_size(px(12.)).text_color(c(MUTED)).child("Pick the one that starts the game:")).child(picks);
            }
        }
        section
    }

    // ---------- store ----------

    fn render_store(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut page = div().child(heading("Get games")).child(
            div().flex().gap_3().child(div().w(px(420.)).child(Input::new(&self.store_search).cleanable(true))).child(
                Button::new("store-go").primary().label(if self.store_busy { "Searching…" } else { "Search" }).disabled(self.store_busy).on_click(cx.listener(
                    |this, _: &ClickEvent, _, cx| {
                        let q = this.store_search.read(cx).value().to_string();
                        this.run_store_search(q, cx);
                    },
                )),
            ),
        );

        if let Some(results) = &self.store_results {
            let mut list = div().flex().flex_col().gap_2().mt_4().max_w(px(820.));
            if results.is_empty() {
                list = list.child(empty_box("No results. Steam search needs an internet connection."));
            }
            for (i, r) in results.iter().enumerate() {
                let key = format!("{}:{}", r.launcher.as_str(), r.id);
                let owned = self.game(&key).is_some_and(|g| !g.info.catalog_only || g.info.installed);
                let image = self.art_image(r.image.as_ref());
                let thumb = div()
                    .w(px(138.))
                    .h(px(64.))
                    .flex_none()
                    .rounded(px(6.))
                    .overflow_hidden()
                    .bg(linear_gradient(135., linear_color_stop(launcher_tint(r.launcher, 0x88), 0.), linear_color_stop(c(PANEL_2), 1.)))
                    .when_some(image, |d, p| d.child(img(p).size_full().object_fit(ObjectFit::Cover)));
                let r2 = r.clone();
                let action = if owned {
                    let key = key.clone();
                    Button::new(sid(format!("sr-{i}"))).small().label("In library").on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.open_details(&key, cx)))
                } else {
                    Button::new(sid(format!("sr-{i}"))).small().primary().label("Get").on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.install_from_store(&r2, cx)))
                };
                list = list.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_4()
                        .p_2()
                        .pr_3()
                        .rounded(px(10.))
                        .bg(c(PANEL))
                        .border_1()
                        .border_color(c(LINE))
                        .child(thumb)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(one_line(div().font_weight(FontWeight::SEMIBOLD)).child(r.title.clone()))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .text_size(px(12.))
                                        .text_color(c(MUTED))
                                        .child(dot(r.launcher))
                                        .child(r.launcher.label())
                                        .when_some(r.price.clone(), |d, p| d.child(tag(p))),
                                ),
                        )
                        .child(action),
                );
            }
            page = page.child(list);
        }

        let ready: Vec<&Game> = self.games.iter().filter(|g| !g.info.installed && !g.info.catalog_only && !g.hidden).collect();
        page = page.child(heading(format!("Owned, not installed · {}", ready.len())));
        page = if ready.is_empty() {
            page.child(empty_box("Everything you own is installed, or the launcher's owned list couldn't be read."))
        } else {
            page.child(self.chips("ready", &ready, cx))
        };
        for l in [LauncherId::Battlenet, LauncherId::Riot] {
            let catalog: Vec<&Game> = self.games.iter().filter(|g| g.info.launcher == l && g.info.catalog_only).collect();
            if !catalog.is_empty() {
                page = page.child(heading(format!("{} catalog", l.label()))).child(self.chips(l.as_str(), &catalog, cx));
            }
        }
        page
    }

    fn chips(&self, prefix: &str, games: &[&Game], cx: &mut Context<Self>) -> impl IntoElement {
        let mut row = div().flex().flex_wrap().gap_2();
        for g in games {
            let key = g.key.clone();
            row = row.child(
                div()
                    .id(sid(format!("chip-{prefix}-{}", g.key)))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py(px(6.))
                    .rounded_full()
                    .bg(c(PANEL_2))
                    .border_1()
                    .border_color(c(LINE))
                    .cursor_pointer()
                    .hover(|s| s.bg(c(HOVER)))
                    .child(dot(g.info.launcher))
                    .child(g.info.title.clone())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.open_details(&key, cx))),
            );
        }
        row
    }

    // ---------- launchers ----------

    fn render_launchers(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut grid = div().flex().flex_wrap().gap_4();
        for st in self.scan.launchers.iter().filter(|l| l.id != LauncherId::Custom) {
            let id = st.id;
            let (state, color) = if !st.supported {
                ("Not available on this OS", MUTED)
            } else if !st.enabled {
                ("Turned off", MUTED)
            } else if st.error.is_some() {
                ("Scan failed", BAD)
            } else if st.detected {
                ("Connected", OK)
            } else {
                ("Not found", WARN)
            };
            let mut actions = div().flex().gap_2().mt_auto();
            if st.detected && st.enabled {
                actions = actions
                    .child(Button::new(sid(format!("show-{}", id.as_str()))).small().label("Show games").on_click(cx.listener(
                        move |this, _: &ClickEvent, _, cx| {
                            this.launcher_filter = Some(id);
                            this.shelf = Shelf::All;
                            this.view = Page::Library;
                            cx.notify();
                        },
                    )))
                    .child(
                        Button::new(sid(format!("open-{}", id.as_str())))
                            .small()
                            .label(format!("Open {}", id.short()))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.open_launcher(id, cx))),
                    );
            } else if st.supported {
                actions = actions.child(
                    Button::new(sid(format!("get-{}", id.as_str())))
                        .small()
                        .label(format!("Get {}", id.short()))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.get_launcher(id, cx))),
                );
            }
            grid = grid.child(
                div()
                    .w(px(270.))
                    .min_h(px(150.))
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .rounded(px(12.))
                    .bg(c(PANEL))
                    .border_1()
                    .border_color(c(LINE))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .size(px(38.))
                                    .rounded(px(10.))
                                    .bg(c(id.color()))
                                    .text_color(c(0x111111))
                                    .font_weight(FontWeight::EXTRA_BOLD)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(id.short()[..1].to_string()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .child(div().font_weight(FontWeight::SEMIBOLD).child(st.name.clone()))
                                    .child(div().text_size(px(12.)).text_color(c(color)).child(state)),
                            )
                            .when(st.supported, |d| {
                                d.child(
                                    Switch::new(sid(format!("en-{}", id.as_str())))
                                        .checked(st.enabled)
                                        .on_click(cx.listener(move |this, on: &bool, _, cx| this.set_launcher_enabled(id, *on, cx))),
                                )
                            }),
                    )
                    .when(st.detected && st.enabled, |d| {
                        d.child(
                            div()
                                .flex()
                                .gap_5()
                                .text_size(px(13.))
                                .text_color(c(MUTED))
                                .child(stat(st.installed_count, "installed"))
                                .child(stat(st.owned_count.saturating_sub(st.installed_count), "ready to install")),
                        )
                    })
                    .when_some(st.path.clone(), |d, p| d.child(one_line(div().text_size(px(12.)).text_color(c(MUTED))).child(p.display().to_string())))
                    .child(actions),
            );
        }
        div()
            .child(heading("Launchers"))
            .child(muted("AsuNa reads each launcher's local files to build one library, then hands launches and installs back to that launcher.").mb_4().max_w(px(680.)))
            .child(grid)
    }

    // ---------- settings ----------

    fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let s = &self.data.settings;
        let toggle = |id: &'static str, title: &'static str, desc: &'static str, on: bool, f: fn(&mut Settings, bool), cx: &mut Context<Self>| {
            setting_row(title, desc).child(Switch::new(id).checked(on).on_click(cx.listener(move |this, v: &bool, _, cx| {
                let v = *v;
                this.update_settings(cx, |s| f(s, v));
            })))
        };

        let mut on_launch = div().flex().gap_1();
        for (opt, label) in [(OnGameLaunch::Nothing, "Stay open"), (OnGameLaunch::Minimize, "Minimize"), (OnGameLaunch::Quit, "Close AsuNa")] {
            let active = s.on_game_launch == opt;
            on_launch = on_launch.child(
                Button::new(sid(format!("ogl-{opt:?}")))
                    .small()
                    .label(label)
                    .when(active, |b| b.primary())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.update_settings(cx, |s| s.on_game_launch = opt))),
            );
        }

        let mut folders = div().flex().flex_col();
        for st in self.scan.launchers.iter().filter(|l| l.supported && l.id != LauncherId::Custom) {
            let id = st.id;
            let custom = s.path_overrides.get(&id);
            let shown = custom
                .map(|p| format!("{} (set by you)", p.display()))
                .or_else(|| st.path.as_ref().map(|p| format!("{} (found automatically)", p.display())))
                .unwrap_or_else(|| "Not found".into());
            folders = folders.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .py_3()
                    .border_b_1()
                    .border_color(c(LINE))
                    .child(div().w(px(150.)).flex_none().flex().items_center().gap_2().child(dot(id)).child(id.label()))
                    .child(one_line(div().flex_1().min_w_0().text_size(px(12.)).text_color(c(MUTED))).child(shown))
                    .child(Button::new(sid(format!("pf-{}", id.as_str()))).small().label("Change…").on_click(cx.listener(
                        move |this, _: &ClickEvent, _, cx| {
                            this.pick_path(true, cx, move |this, p, cx| this.update_settings(cx, |s| {
                                s.path_overrides.insert(id, p);
                            }));
                        },
                    )))
                    .when(custom.is_some(), |d| {
                        d.child(Button::new(sid(format!("pr-{}", id.as_str()))).small().ghost().label("Reset").on_click(cx.listener(
                            move |this, _: &ClickEvent, _, cx| this.update_settings(cx, |s| {
                                s.path_overrides.remove(&id);
                            }),
                        )))
                    }),
            );
        }

        let data_folder = self.data_folder().display().to_string();
        div()
            .max_w(px(820.))
            .child(heading("Settings"))
            .child(group("Startup"))
            .child(toggle(
                "s-login",
                "Open AsuNa when I sign in",
                "Starts with your computer, minimized, so your library is ready.",
                s.open_at_login,
                |s, v| s.open_at_login = v,
                cx,
            ))
            .child(toggle("s-min", "Always start minimized", "Open in the taskbar instead of on screen.", s.start_minimized, |s, v| s.start_minimized = v, cx))
            .child(toggle(
                "s-scan",
                "Rescan launchers on start",
                "Your last library shows instantly either way. Turn off for the fastest start.",
                s.scan_on_startup,
                |s, v| s.scan_on_startup = v,
                cx,
            ))
            .child(group("Playing"))
            .child(setting_row("When a game starts", "Get AsuNa out of the way while you play. Closing it frees all of its memory.").child(on_launch))
            .child(group("Library"))
            .child(toggle("s-art", "Show box art", "Turn off for a lighter, text-only library.", s.show_art, |s, v| s.show_art = v, cx))
            .child(toggle(
                "s-online",
                "Online lookups",
                "Lets AsuNa ask the Steam store for names of games you own, store search and box art. Nothing about you is sent.",
                s.online_lookups,
                |s, v| s.online_lookups = v,
                cx,
            ))
            .child(group("Launcher folders"))
            .child(muted("If AsuNa can't find a launcher, point it at the folder where the launcher is installed.").text_size(px(12.)).mb_2())
            .child(folders)
            .child(group("About"))
            .child(setting_row("AsuNa Launcher", "Built with Rust and GPUI.").child(muted(format!("Version {}", env!("CARGO_PKG_VERSION")))))
            .child(setting_row("Data folder", "Settings, the last scan and cached box art.").child(one_line(muted(data_folder).text_size(px(12.)).max_w(px(360.)))))
    }

    // ---------- add game ----------

    fn render_add_game(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(form) = &self.add_game else { return div().into_any_element() };
        let field = |label: &'static str, input: AnyElement| div().flex().flex_col().gap_2().child(div().font_weight(FontWeight::MEDIUM).child(label)).child(input);
        let path_state = form.path.clone();
        div()
            .id("add-backdrop")
            .absolute()
            .inset_0()
            .bg(ca(0, 0x90))
            .flex()
            .items_center()
            .justify_center()
            .occlude()
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.add_game = None;
                cx.notify();
            }))
            .child(
                div()
                    .id("add-dialog")
                    .w(px(480.))
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .rounded(px(14.))
                    .bg(c(PANEL))
                    .border_1()
                    .border_color(c(LINE))
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(div().text_size(px(18.)).font_weight(FontWeight::SEMIBOLD).child("Add a game"))
                    .child(muted("For games outside a supported launcher: emulators, itch.io downloads, DRM-free installs."))
                    .child(field(
                        "Program",
                        div()
                            .flex()
                            .gap_2()
                            .child(div().flex_1().child(Input::new(&form.path)))
                            .child(Button::new("add-browse").label("Browse…").on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                let path_state = path_state.clone();
                                this.pick_path(false, cx, move |this, p, cx| {
                                    let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                                    let shown = p.display().to_string();
                                    if let Some(window) = cx.active_window() {
                                        let _ = window.update(cx, |_, window, cx| {
                                            path_state.update(cx, |s, cx| s.set_value(shown, window, cx));
                                            if let Some(f) = &this.add_game {
                                                if f.name.read(cx).value().is_empty() {
                                                    f.name.update(cx, |s, cx| s.set_value(name, window, cx));
                                                }
                                            }
                                        });
                                    }
                                });
                            })))
                            .into_any_element(),
                    ))
                    .child(field("Name", Input::new(&form.name).into_any_element()))
                    .child(field("Launch options (optional)", Input::new(&form.args).into_any_element()))
                    .when_some(form.error.clone(), |d, e| d.child(div().text_color(c(BAD)).child(e)))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(Button::new("add-cancel").label("Cancel").on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.add_game = None;
                                cx.notify();
                            })))
                            .child(Button::new("add-ok").primary().label("Add game").on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.submit_add_game(cx)))),
                    ),
            )
            .into_any_element()
    }

    fn render_toasts(&self) -> impl IntoElement {
        let mut col = div().absolute().bottom_5().left_0().right_0().flex().flex_col().items_center().gap_2();
        for t in &self.toasts {
            col = col.child(
                div()
                    .px_4()
                    .py_2()
                    .rounded(px(8.))
                    .bg(c(0x2a2e3a))
                    .border_1()
                    .border_color(c(if t.error { BAD } else { LINE }))
                    .shadow_lg()
                    .child(t.text.clone()),
            );
        }
        col
    }
}

use std::path::PathBuf;

fn tag(text: impl Into<SharedString>) -> Div {
    div().ml_1().px(px(6.)).rounded(px(4.)).text_size(px(11.)).bg(c(PANEL_2)).border_1().border_color(c(LINE)).child(text.into())
}

fn stat(n: usize, label: &'static str) -> Div {
    div().flex().items_baseline().gap_1().child(div().text_size(px(18.)).text_color(c(TEXT)).font_weight(FontWeight::SEMIBOLD).child(n.to_string())).child(label)
}

fn group(title: &'static str) -> Div {
    div().mt_6().mb_1().text_size(px(11.)).text_color(c(MUTED)).child(title.to_uppercase())
}

fn setting_row(title: &'static str, desc: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_4()
        .py_3()
        .border_b_1()
        .border_color(c(LINE))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(div().font_weight(FontWeight::MEDIUM).child(title))
                .child(div().text_size(px(12.)).text_color(c(MUTED)).child(desc)),
        )
}

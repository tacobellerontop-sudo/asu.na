use super::{Detection, Provider, WINDOWS};
use crate::context::Ctx;
use crate::fsx::read_text;
use crate::types::*;
use serde::Deserialize;
use std::path::PathBuf;

pub struct Xbox;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AppxPackage {
    name: String,
    package_family_name: String,
    install_location: Option<String>,
}

const LIST_PACKAGES: &str = "Get-AppxPackage | Where-Object { -not $_.IsFramework -and $_.SignatureKind -eq 'Store' } | Select-Object Name, PackageFamilyName, InstallLocation | ConvertTo-Json -Compress";

fn attr<'a>(xml: &'a str, tag: &str, name: &str) -> Option<&'a str> {
    let start = xml.find(&format!("<{tag}"))?;
    let rest = &xml[start..];
    let end = rest.find('>')?;
    let el = &rest[..end];
    let a = el.find(&format!("{name}=\""))? + name.len() + 2;
    let len = el[a..].find('"')?;
    Some(&el[a..a + len])
}

fn element<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let s = xml.find(&format!("<{tag}>"))? + tag.len() + 2;
    let e = xml[s..].find(&format!("</{tag}>"))?;
    Some(xml[s..s + e].trim())
}

/// Display name and Store id from MicrosoftGame.config (only GDK games ship one).
pub fn parse_game_config(xml: &str) -> (Option<String>, Option<String>) {
    let title = attr(xml, "ShellVisuals", "DefaultDisplayName").filter(|t| !t.starts_with("ms-resource:")).map(str::to_string);
    (title, element(xml, "StoreId").map(str::to_string))
}

/// First <Application Id="..."> in AppxManifest.xml, needed for shell:AppsFolder links.
pub fn parse_app_id(manifest: &str) -> Option<String> {
    let mut from = 0;
    while let Some(i) = manifest[from..].find("<Application") {
        let at = from + i;
        // Skip <Applications>
        if manifest[at + "<Application".len()..].starts_with(char::is_whitespace) {
            return attr(&manifest[at..], "Application", "Id").map(str::to_string);
        }
        from = at + 1;
    }
    None
}

impl Provider for Xbox {
    fn id(&self) -> LauncherId {
        LauncherId::Xbox
    }
    fn platforms(&self) -> &'static [Platform] {
        WINDOWS
    }
    fn download_url(&self) -> &'static str {
        "https://www.xbox.com/apps/xbox-app-for-pc"
    }
    fn detect(&self, ctx: &Ctx) -> Detection {
        Detection { detected: ctx.platform == Platform::Windows, path: None }
    }
    fn open(&self, _d: &Detection) -> LaunchAction {
        LaunchAction::uri("xbox:")
    }

    fn scan(&self, ctx: &Ctx, _d: &Detection) -> Vec<DetectedGame> {
        let raw = (ctx.powershell)(LIST_PACKAGES);
        let raw = raw.trim();
        if raw.is_empty() {
            return vec![];
        }
        let pkgs: Vec<AppxPackage> = serde_json::from_str::<Vec<AppxPackage>>(raw)
            .or_else(|_| serde_json::from_str::<AppxPackage>(raw).map(|p| vec![p]))
            .unwrap_or_default();
        let mut out = Vec::new();
        for p in pkgs {
            let Some(loc) = p.install_location.map(PathBuf::from) else { continue };
            let Some(config) = read_text(loc.join("MicrosoftGame.config")) else { continue };
            let (title, store_id) = parse_game_config(&config);
            let Some(app_id) = read_text(loc.join("AppxManifest.xml")).and_then(|m| parse_app_id(&m)) else { continue };
            let store = match store_id {
                Some(id) => format!("ms-windows-store://pdp/?ProductId={id}"),
                None => format!("ms-windows-store://pdp/?PFN={}", p.package_family_name),
            };
            let mut g = DetectedGame::new(LauncherId::Xbox, &p.package_family_name, title.unwrap_or(p.name));
            g.installed = true;
            g.install_path = Some(loc);
            g.actions = Actions {
                launch: Some(LaunchAction::exec("explorer.exe", &[&format!("shell:AppsFolder\\{}!{app_id}", p.package_family_name)])),
                install: Some(LaunchAction::uri(&store)),
                uninstall: Some(LaunchAction::uri("ms-settings:appsfeatures")),
                store: Some(LaunchAction::uri(store)),
            };
            out.push(g);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::testing::{tree, Fakes};

    #[test]
    fn parsers() {
        assert_eq!(
            parse_game_config(r#"<Game><ShellVisuals DefaultDisplayName="Forza Horizon 5" /><StoreId>9NKX70BBCDRN</StoreId></Game>"#),
            (Some("Forza Horizon 5".into()), Some("9NKX70BBCDRN".into()))
        );
        assert_eq!(parse_game_config(r#"<ShellVisuals DefaultDisplayName="ms-resource:Name"/>"#).0, None);
        assert_eq!(parse_app_id(r#"<Applications><Application Id="Game" Executable="x.exe">"#), Some("Game".into()));
    }

    #[test]
    fn keeps_gdk_games_only() {
        let game = tree(&[
            ("MicrosoftGame.config", r#"<Game><ShellVisuals DefaultDisplayName="Forza Horizon 5"/></Game>"#),
            ("AppxManifest.xml", r#"<Package><Applications><Application Id="ForzaHorizon5" Executable="x.exe"/></Applications></Package>"#),
        ]);
        let app = tree(&[("AppxManifest.xml", r#"<Application Id="App"/>"#)]);
        let json = serde_json::json!([
            { "Name": "Forza", "PackageFamilyName": "Microsoft.Forza_8wekyb3d8bbwe", "InstallLocation": game.path() },
            { "Name": "Calc", "PackageFamilyName": "Microsoft.Calc_8wekyb3d8bbwe", "InstallLocation": app.path() },
            { "Name": "Broken", "PackageFamilyName": "x", "InstallLocation": null }
        ])
        .to_string();
        let mut f = Fakes::new();
        f.ps = Box::new(move |_| json.clone());
        let games = Xbox.scan(&f.ctx(Platform::Windows, &[]), &Detection::default());
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].title, "Forza Horizon 5");
        assert_eq!(
            games[0].actions.launch,
            Some(LaunchAction::exec("explorer.exe", &["shell:AppsFolder\\Microsoft.Forza_8wekyb3d8bbwe!ForzaHorizon5"]))
        );
    }
}

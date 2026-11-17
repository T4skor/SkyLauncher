#![cfg_attr(windows, windows_subsystem = "windows")]

use eframe::egui;
mod icons;
mod theme;
use serde_json::{json, Value};
use std::{
    fs,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

struct Launcher {
    child: Child,
    input: ChildStdin,
    events: mpsc::Receiver<Value>,
    tab: usize,
    busy: bool,
    status: String,
    progress: f32,
    client: String,
    java: String,
    ram: u32,
    version: String,
    snapshots: bool,
    versions: Vec<(String, String)>,
    installed: Vec<String>,
    search: String,
    installed_only: bool,
    account: String,
    session_notice: String,
    session_remembered: bool,
    session_can_restore: bool,
    session_can_forget: bool,
    offline: bool,
    offline_name: String,
    login_url: String,
    response_url: String,
    skin_path: String,
    slim: bool,
    texture: Option<egui::TextureHandle>,
    delete: Option<String>,
    mods: Vec<Value>,
    mod_search: String,
    mod_delete: Option<String>,
    last_error: bool,
    local_profiles: Vec<Value>,
    local_profile: String,
    profile_delete: Option<String>,
    loader: usize,
    skin_catalog: Vec<CatalogSkin>,
    catalog_after: String,
    catalog_loaded: bool,
    skin_store_open: bool,
    skin_back: bool,
    confirmed_skin: Option<(String, bool)>,
    skin_publish_url: String,
    skin_publish_name: String,
    skin_publish_notice: String,
    pending_action: String,
    mod_filter: ModFilter,
    mod_store_open: bool,
    mod_catalog: Vec<Value>,
    mod_catalog_query: String,
    mod_catalog_version: String,
    mod_catalog_context: String,
    mod_catalog_loaded: bool,
    mod_install: Option<Value>,
    loader_setup_open: bool,
}

#[derive(Clone, Copy, Default, PartialEq)]
enum ModFilter {
    #[default]
    All,
    Enabled,
    Disabled,
}

fn version_loader(version: &str) -> Option<&'static str> {
    let version = version.to_ascii_lowercase();
    if version.contains("fabric-loader") {
        Some("Fabric")
    } else if version.contains("neoforge") {
        Some("NeoForge")
    } else if version.contains("quilt") {
        Some("Quilt")
    } else if version.contains("forge") {
        Some("Forge")
    } else {
        None
    }
}

/// Friendly label for a launcher version id, keeping the technical id intact.
fn version_label(version: &str) -> String {
    let folded = version.to_ascii_lowercase();
    for (prefix, label) in [("fabric-loader-", "Fabric"), ("quilt-loader-", "Quilt")] {
        if folded.starts_with(prefix) {
            if let Some((_, game)) = version[prefix.len()..].split_once('-') {
                return format!("{label} {game}");
            }
        }
    }
    if folded.starts_with("neoforge-") {
        return format!("NeoForge {}", &version["neoforge-".len()..]);
    }
    if let Some(index) = folded.find("-forge-") {
        return format!("Forge {}", &version[..index]);
    }
    version.to_owned()
}

fn downloads_label(downloads: u64) -> String {
    if downloads >= 1_000_000 {
        let millions = downloads as f64 / 1_000_000.;
        if millions >= 10. {
            format!("{millions:.0} M")
        } else {
            format!("{millions:.1} M").replace('.', ",")
        }
    } else if downloads >= 1_000 {
        format!("{:.0} mil", downloads as f64 / 1_000.)
    } else {
        downloads.to_string()
    }
}

fn mod_matches(entry: &Value, query: &str, filter: ModFilter) -> bool {
    let enabled = entry["enabled"].as_bool().unwrap_or(false);
    entry["name"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase()
        .contains(&query.trim().to_lowercase())
        && match filter {
            ModFilter::All => true,
            ModFilter::Enabled => enabled,
            ModFilter::Disabled => !enabled,
        }
}

struct CatalogSkin {
    id: String,
    name: String,
    texture: Option<egui::TextureHandle>,
}

const LOADERS: [(&str, &str); 4] = [
    ("fabric", "Fabric"),
    ("forge", "Forge"),
    ("neoforge", "NeoForge"),
    ("quilt", "Quilt"),
];

impl Launcher {
    fn new(cc: &eframe::CreationContext<'_>, mut child: Child) -> Self {
        theme::install(&cc.egui_ctx);
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (tx, events) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                if let Ok(event) = serde_json::from_str(&line) {
                    if tx.send(event).is_err() {
                        return;
                    }
                    ctx.request_repaint();
                }
            }
            let _ = tx.send(json!({"event":"error","text":"El motor se ha cerrado. Comprueba Python y requirements.txt; reinicia el launcher."}));
            ctx.request_repaint();
        });
        let mut app = Self {
            child,
            input,
            events,
            tab: 0,
            busy: false,
            status: "Preparando…".into(),
            progress: 0.,
            client: String::new(),
            java: String::new(),
            ram: 4,
            version: String::new(),
            snapshots: false,
            versions: vec![],
            installed: vec![],
            search: String::new(),
            installed_only: false,
            account: String::new(),
            session_notice: String::new(),
            session_remembered: false,
            session_can_restore: false,
            session_can_forget: false,
            offline: true,
            offline_name: String::new(),
            login_url: String::new(),
            response_url: String::new(),
            skin_path: String::new(),
            slim: false,
            texture: None,
            delete: None,
            mods: vec![],
            mod_search: String::new(),
            mod_delete: None,
            last_error: false,
            local_profiles: vec![],
            local_profile: String::new(),
            profile_delete: None,
            loader: 0,
            skin_catalog: vec![],
            catalog_after: String::new(),
            catalog_loaded: false,
            skin_store_open: false,
            skin_back: false,
            confirmed_skin: None,
            skin_publish_url: String::new(),
            skin_publish_name: String::new(),
            skin_publish_notice: String::new(),
            pending_action: String::new(),
            mod_filter: ModFilter::All,
            mod_store_open: false,
            mod_catalog: vec![],
            mod_catalog_query: String::new(),
            mod_catalog_version: String::new(),
            mod_catalog_context: String::new(),
            mod_catalog_loaded: false,
            mod_install: None,
            loader_setup_open: false,
        };
        app.send(json!({"action":"init"}));
        app
    }

    fn send(&mut self, mut value: Value) {
        if self.busy {
            return;
        }
        if let Some(object) = value.as_object_mut() {
            object.entry("offline").or_insert(json!(self.offline));
            object
                .entry("local_profile")
                .or_insert(json!(self.local_profile));
        }
        match writeln!(self.input, "{value}").and_then(|_| self.input.flush()) {
            Ok(()) => {
                self.pending_action = value["action"].as_str().unwrap_or_default().into();
                if self.pending_action == "mod_catalog_search" {
                    self.mod_install = None;
                    self.mod_catalog_version = self.mod_target();
                    self.mod_catalog_loaded = false;
                    self.mod_catalog.clear();
                }
                self.busy = true;
                self.last_error = false;
                self.progress = 0.;
                self.status = "Procesando…".into();
            }
            Err(_) => {
                self.status = "No se puede comunicar con el motor. Reinicia el launcher.".into();
                self.last_error = true;
            }
        }
    }

    fn configured(&mut self, action: &str) {
        self.send(
            json!({"action":action,"client_id":self.client,"java":self.java,
            "ram":self.ram,"version":self.version,"snapshots":self.snapshots,
             "offline":self.offline,"offline_name":self.offline_name,
             "local_profile":self.local_profile,"skin_path":self.skin_path,
             "loader":LOADERS[self.loader].0,
             "variant":if self.slim {"slim"} else {"classic"}}),
        );
    }

    /// A published URL belongs to the previous PNG, so never offer it for a new one.
    fn clear_skin_publish(&mut self) {
        self.skin_publish_url.clear();
        self.skin_publish_notice.clear();
    }

    fn preview(&mut self, ctx: &egui::Context) {
        self.clear_skin_publish();
        match load_skin_texture(ctx, &self.skin_path, "skin") {
            Some(texture) => {
                if texture.size()[1] == 32 {
                    self.slim = false;
                }
                self.texture = Some(texture);
                self.last_error = false;
                self.status = "Textura cargada. La vista previa no modifica tu cuenta.".into();
            }
            _ => {
                self.texture = None;
                self.status = "Selecciona un PNG de skin de 64×64 o 64×32.".into();
                self.last_error = true;
            }
        }
    }

    fn drain(&mut self, ctx: &egui::Context) {
        while let Ok(v) = self.events.try_recv() {
            let text = |key: &str| v[key].as_str().unwrap_or_default().to_owned();
            match v["event"].as_str().unwrap_or_default() {
                "session_state" => {
                    self.session_notice = text("text");
                    self.session_remembered = v["remembered"].as_bool().unwrap_or(false);
                    self.session_can_restore = v["can_restore"].as_bool().unwrap_or(false);
                    self.session_can_forget = v["can_forget"].as_bool().unwrap_or(false);
                }
                "local_profiles" => {
                    self.local_profiles = v["profiles"].as_array().cloned().unwrap_or_default();
                }
                "local_profile_selected" => {
                    let p = &v["value"];
                    self.local_profile = p["id"].as_str().unwrap_or_default().into();
                    self.offline_name = p["name"].as_str().unwrap_or_default().into();
                    if let Some(version) = p["version"].as_str() {
                        self.version = version.into();
                    }
                    self.offline = true;
                    self.skin_path = p["skin_path"].as_str().unwrap_or_default().into();
                    self.slim = p["variant"].as_str() == Some("slim");
                    self.confirmed_skin = Some((self.skin_path.clone(), self.slim));
                    self.clear_skin_publish();
                    if self.skin_path.is_empty() {
                        self.texture = None;
                    } else {
                        self.preview(ctx);
                    }
                }
                "skin_catalog" => {
                    self.catalog_loaded = true;
                    self.catalog_after = text("after");
                    self.skin_catalog = v["skins"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|p| {
                            let id = p["id"].as_str().unwrap_or_default().to_owned();
                            let texture = p["path"]
                                .as_str()
                                .and_then(|path| load_skin_texture(ctx, path, &id));
                            CatalogSkin {
                                id,
                                name: p["name"].as_str().unwrap_or("Skin").into(),
                                texture,
                            }
                        })
                        .collect();
                }
                "done" => {
                    if !self.last_error && self.pending_action == "skin_upload" {
                        self.confirmed_skin = Some((self.skin_path.clone(), self.slim));
                    }
                    if !self.last_error && self.pending_action == "skin_reset" {
                        self.confirmed_skin = None;
                        self.skin_path.clear();
                        self.texture = None;
                        self.clear_skin_publish();
                    }
                    self.pending_action.clear();
                    self.busy = false;
                }
                "mod_catalog" => {
                    self.mod_catalog = v["items"].as_array().cloned().unwrap_or_default();
                    self.mod_catalog_loaded = true;
                    self.mod_catalog_context = format!(
                        "Minecraft {} · {}",
                        text("minecraft_version"),
                        text("loader")
                    );
                }
                "mods" => self.mods = v["mods"].as_array().cloned().unwrap_or_default(),
                "selected_version" => self.version = text("version"),
                "status" => {
                    self.status = text("text");
                    self.last_error = false;
                }
                "error" => {
                    self.status = text("text");
                    self.last_error = true;
                    self.busy = false;
                }
                "progress" => self.progress = v["value"].as_f64().unwrap_or(0.) as f32,
                "profile" => {
                    self.confirmed_skin = None;
                    self.account = text("name");
                    self.login_url.clear();
                    self.response_url.clear();
                }
                "login_url" => {
                    self.login_url = text("url");
                    self.status = "Completa el acceso en el navegador y pega la URL final.".into();
                    if webbrowser::open(&self.login_url).is_err() {
                        self.status = "Abre el enlace Microsoft que aparece abajo.".into();
                    }
                }
                "versions" => {
                    self.versions = v["versions"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|x| {
                            Some((x["id"].as_str()?.into(), x["type"].as_str()?.into()))
                        })
                        .collect();
                    if self.version.is_empty() {
                        self.version = self
                            .versions
                            .iter()
                            .find(|x| x.1 == "release")
                            .map(|x| x.0.clone())
                            .unwrap_or_default();
                    }
                }
                "installed" => {
                    self.installed = v["versions"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|x| x.as_str().map(str::to_owned))
                        .collect()
                }
                "settings" => {
                    let s = &v["value"];
                    self.client = s["client_id"].as_str().unwrap_or_default().into();
                    self.java = s["java"].as_str().unwrap_or_default().into();
                    self.version = s["version"].as_str().unwrap_or_default().into();
                    self.ram = (s["ram"].as_u64().unwrap_or(4) as u32).clamp(1, 64);
                    self.snapshots = s["snapshots"].as_bool().unwrap_or(false);
                    self.offline = s["offline"].as_bool().unwrap_or(false);
                    self.offline_name = s["offline_name"].as_str().unwrap_or_default().into();
                    self.local_profile = s["local_profile"].as_str().unwrap_or_default().into();
                }
                "skin" => {
                    self.skin_path = text("path");
                    self.slim = text("variant") == "slim";
                    self.preview(ctx);
                    if self.pending_action == "catalog_choose" {
                        self.skin_store_open = false;
                    }
                    if self.pending_action == "skin_current" {
                        self.confirmed_skin = Some((self.skin_path.clone(), self.slim));
                    }
                }
                "skin_published" => {
                    self.skin_publish_url = text("url");
                    self.slim = text("variant") == "slim";
                    self.skin_publish_notice = "Publicada en MineSkin. Copia el comando de SkinsRestorer y ejecútalo en el servidor.".into();
                }
                _ => {}
            }
        }
    }

    fn play_blocker(&self) -> Option<&'static str> {
        if self.busy {
            Some("Espera a que termine la operación o cierra la partida en curso.")
        } else if self.version.is_empty() {
            Some("Selecciona una versión para jugar.")
        } else if self.offline && !valid_local_name(&self.offline_name) {
            Some("Escribe un nombre local de 3–16 letras, números o guiones bajos.")
        } else if !self.offline && self.account.is_empty() {
            Some("Inicia sesión con Microsoft o elige el modo local en Cuenta.")
        } else {
            None
        }
    }

    fn skin_support_missing(&self) -> bool {
        self.offline
            && self.local_profiles.iter().any(|p| {
                p["id"].as_str() == Some(&self.local_profile)
                    && !p["skin"].as_str().unwrap_or_default().is_empty()
                    && p["skin_support_version"].as_str() != Some(&self.version)
            })
    }

    /// CustomSkinLoader can hide the skin a server sends, so publishing with
    /// SkinsRestorer needs a warning while the local override is active.
    fn local_skin_active(&self) -> bool {
        self.offline
            && self.local_profiles.iter().any(|p| {
                p["id"].as_str() == Some(&self.local_profile)
                    && !p["skin"].as_str().unwrap_or_default().is_empty()
                    && p["skin_support_version"].as_str() == Some(&self.version)
            })
    }

    fn profile_controls(&mut self, ui: &mut egui::Ui) {
        let has_profiles = !self.local_profiles.is_empty();
        let selected = self
            .local_profiles
            .iter()
            .find(|p| p["id"].as_str() == Some(&self.local_profile));
        let label = selected
            .map(|p| p["name"].as_str().unwrap_or("Perfil"))
            .unwrap_or("Nuevo perfil local")
            .to_owned();
        let mut requested = None;
        if has_profiles {
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt("local_profile")
                    .width(250.)
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        for p in &self.local_profiles {
                            let id = p["id"].as_str().unwrap_or_default();
                            let text = format!(
                                "{} · {}",
                                p["name"].as_str().unwrap_or("Perfil"),
                                p["version"].as_str().unwrap_or("")
                            );
                            if ui
                                .selectable_label(id == self.local_profile, text)
                                .clicked()
                            {
                                requested = Some(id.to_owned());
                            }
                        }
                    });
                if theme::button_sized(
                    ui,
                    "Nuevo perfil",
                    theme::Tone::Quiet,
                    egui::vec2(120., 38.),
                )
                .clicked()
                {
                    self.local_profile.clear();
                    self.offline_name.clear();
                    self.skin_path.clear();
                    self.texture = None;
                    self.send(json!({"action":"mods"}));
                }
            });
            if let Some(id) = requested {
                self.send(json!({"action":"profile_select", "id":id}));
            }
            ui.add_space(4.);
        }
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Nombre de jugador")
                    .size(13.)
                    .color(theme::MUTED),
            );
            theme::help(
                ui,
                "Es tu identidad en el juego, no una cuenta. Cada perfil guarda nombre, versión y skin, con sus propios mundos y mods.",
            );
            if !self.version.is_empty() {
                theme::badge_icon(
                    ui,
                    icons::Icon::Cube,
                    &version_label(&self.version),
                    theme::STONE,
                );
            }
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.offline_name)
                .hint_text("Por ejemplo: Steve")
                .desired_width(250.)
                .margin(egui::Margin::symmetric(10, 8))
                .char_limit(16),
        );
        if !self.offline_name.is_empty() && !valid_local_name(&self.offline_name) {
            ui.label(
                egui::RichText::new("Usa de 3 a 16 letras A–Z, números o guiones bajos.")
                    .size(13.)
                    .color(theme::REDSTONE),
            );
        }
        ui.horizontal_wrapped(|ui| {
            if theme::button_icon_state(
                ui,
                Some(if self.local_profile.is_empty() {
                    icons::Icon::Plus
                } else {
                    icons::Icon::Save
                }),
                if self.local_profile.is_empty() {
                    "Crear perfil"
                } else {
                    "Guardar cambios"
                },
                theme::Tone::Quiet,
                egui::vec2(168., 38.),
                valid_local_name(&self.offline_name) && !self.version.is_empty(),
            )
            .clicked()
            {
                self.configured("profile_save");
            }
            if !self.local_profile.is_empty()
                && theme::button_icon_state(
                    ui,
                    Some(icons::Icon::Trash),
                    "Eliminar perfil",
                    theme::Tone::Danger,
                    egui::vec2(140., 38.),
                    true,
                )
                .clicked()
            {
                self.profile_delete = Some(self.local_profile.clone());
            }
        });
    }

    fn catalog_ui(&mut self, ui: &mut egui::Ui) {
        theme::caption(ui, "Elige una skin para probarla. No se guarda ni se aplica hasta que lo confirmes en Mi skin.");
        ui.horizontal_wrapped(|ui| {
            if theme::button_icon_state(
                ui,
                Some(icons::Icon::Refresh),
                if self.catalog_loaded {
                    "Actualizar catálogo"
                } else {
                    "Cargar catálogo"
                },
                theme::Tone::Quiet,
                egui::vec2(180., 38.),
                true,
            )
            .clicked()
            {
                self.send(json!({"action":"skin_catalog"}));
            }
            if theme::button_icon_state(
                ui,
                Some(icons::Icon::Play),
                "Página siguiente",
                theme::Tone::Quiet,
                egui::vec2(160., 38.),
                !self.catalog_after.is_empty(),
            )
            .clicked()
            {
                self.send(json!({"action":"skin_catalog","after":self.catalog_after}));
            }
            ui.hyperlink_to("Abrir MineSkin", "https://mineskin.org/");
            theme::help(
                ui,
                "Catálogo público de MineSkin, contenido de terceros sin curar por SkyLauncher y sin pagos. Consultarlo envía tu dirección IP a MineSkin y al servidor de texturas de Minecraft.",
            );
        });
        ui.add_space(12.);
        if self.skin_catalog.is_empty() {
            theme::caption(
                ui,
                if self.busy && self.pending_action == "skin_catalog" {
                    "Cargando skins de MineSkin…"
                } else if self.catalog_loaded {
                    "No hay skins en esta página. Actualiza para volver al inicio."
                } else {
                    "Carga el catálogo para ver las skins."
                },
            );
        }
        let columns = if ui.available_width() >= 720. { 4 } else { 2 };
        let mut chosen = None;
        ui.columns(columns, |uis| {
            for (index, skin) in self.skin_catalog.iter().enumerate() {
                let ui = &mut uis[index % columns];
                theme::panel().inner_margin(12).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if let Some(texture) = &skin.texture {
                        ui.vertical_centered(|ui| theme::skin_preview(ui, texture, false, 128.));
                    } else {
                        ui.label("Vista previa no disponible.");
                    }
                    ui.add(egui::Label::new(egui::RichText::new(&skin.name).strong()).truncate())
                        .on_hover_text(&skin.name);
                    if theme::button(ui, "Probar skin", theme::Tone::Quiet).clicked() {
                        chosen = Some(skin.id.clone());
                    }
                    ui.hyperlink_to(
                        "Ver en MineSkin",
                        format!("https://mineskin.org/{}", skin.id),
                    );
                });
                ui.add_space(12.);
            }
        });
        if let Some(id) = chosen {
            self.send(json!({"action":"catalog_choose", "id":id}));
        }
        theme::caption(
            ui,
            "Vistas rápidas con brazos Classic; al elegir se consulta el modelo real.",
        );
    }

    fn skinsrestorer_ui(&mut self, ui: &mut egui::Ui) {
        theme::block(
            ui,
            theme::SKY_BLUE,
            "Usar en un servidor con SkinsRestorer",
            "Publica el PNG y aplica la textura con el plugin del servidor.",
            |ui| {
                theme::caption(
                    ui,
                    "La publicación es anónima. La URL de textura será pública y cualquiera que la conozca podrá descargarla; no publiques una skin que quieras mantener privada.",
                );
                if self.local_skin_active() {
                    ui.add_space(4.);
                    theme::caption(
                        ui,
                        "Este perfil tiene activada una skin local con CustomSkinLoader. Ese mod puede ocultar la skin que envía el servidor: desactívalo o quítalo en Mods y vuelve a entrar para ver la de SkinsRestorer.",
                    );
                    if theme::button_sized(
                        ui,
                        "Abrir Mods",
                        theme::Tone::Quiet,
                        egui::vec2(130., 32.),
                    )
                    .clicked()
                    {
                        self.tab = 3;
                        self.mod_store_open = false;
                    }
                }
                let can_publish =
                    self.texture.is_some() && !self.skin_path.is_empty() && !self.busy;
                if theme::button_state(
                    ui,
                    "Publicar para SkinsRestorer",
                    theme::Tone::Primary,
                    egui::vec2(230., 38.),
                    can_publish,
                )
                .clicked()
                {
                    self.skin_publish_notice.clear();
                    let variant = if self.slim { "slim" } else { "classic" };
                    self.send(json!({"action":"skin_publish","skin_path":self.skin_path,"variant":variant}));
                }
                if self.texture.is_none() || self.skin_path.is_empty() {
                    theme::caption(
                        ui,
                        "Carga o elige una skin en «Textura» para poder publicarla.",
                    );
                }
                if !self.skin_publish_notice.is_empty() {
                    ui.add_space(4.);
                    theme::caption(ui, &self.skin_publish_notice);
                }
                ui.add_space(8.);
                ui.label(egui::RichText::new("URL de textura").font(theme::display(14.)));
                ui.add(
                    egui::TextEdit::singleline(&mut self.skin_publish_url)
                        .desired_width(f32::INFINITY)
                        .hint_text("https://textures.minecraft.net/texture/…")
                        .margin(egui::Margin::symmetric(10, 8)),
                );
                theme::caption(
                    ui,
                    "Se rellena al publicar. También puedes pegar una URL de textura existente.",
                );
                ui.add_space(6.);
                ui.label(
                    egui::RichText::new("Nombre del skin personalizado").font(theme::display(14.)),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.skin_publish_name)
                        .desired_width(f32::INFINITY)
                        .hint_text("3–16 letras, números o guiones bajos")
                        .margin(egui::Margin::symmetric(10, 8)),
                );
                let url_valid = valid_texture_url(&self.skin_publish_url);
                let custom_command =
                    skin_custom_command(&self.skin_publish_name, &self.skin_publish_url, self.slim);
                let named_command = skin_named_command(&self.skin_publish_name);
                ui.add_space(8.);
                ui.horizontal_wrapped(|ui| {
                    if theme::button_state(
                        ui,
                        "Copiar /skin url",
                        theme::Tone::Quiet,
                        egui::vec2(150., 32.),
                        url_valid,
                    )
                    .clicked()
                    {
                        ui.ctx()
                            .copy_text(skin_url_command(&self.skin_publish_url, self.slim));
                    }
                    if theme::button_state(
                        ui,
                        "Copiar /sr createcustom",
                        theme::Tone::Quiet,
                        egui::vec2(190., 32.),
                        custom_command.is_some(),
                    )
                    .clicked()
                    {
                        if let Some(command) = custom_command.as_ref() {
                            ui.ctx().copy_text(command.clone());
                        }
                    }
                    if theme::button_state(
                        ui,
                        "Copiar /skin <nombre>",
                        theme::Tone::Quiet,
                        egui::vec2(175., 32.),
                        named_command.is_some(),
                    )
                    .clicked()
                    {
                        if let Some(command) = named_command.as_ref() {
                            ui.ctx().copy_text(command.clone());
                        }
                    }
                });
                if url_valid {
                    ui.add_space(4.);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(skin_url_command(
                                &self.skin_publish_url,
                                self.slim,
                            ))
                            .monospace()
                            .size(12.),
                        )
                        .truncate(),
                    )
                    .on_hover_text("Pega este comando en el chat del servidor.");
                }
                theme::caption(
                    ui,
                    "Permisos: skinsrestorer.command.set.url para /skin url; skinsrestorer.admincommand.createcustom para /sr createcustom. /skin <nombre> usa el permiso de cambio de skin del servidor.",
                );
            },
        );
    }

    fn mod_catalog_ui(&mut self, ui: &mut egui::Ui) {
        if self.mod_catalog_version != self.mod_target() {
            self.mod_install = None;
        }
        if self.version.is_empty()
            || self
                .versions
                .iter()
                .any(|(id, kind)| id == &self.version && kind != "local")
        {
            theme::caption(ui, "Elige una versión con loader para buscar mods.");
            if theme::button(ui, "Instalar un loader", theme::Tone::Primary).clicked() {
                self.mod_store_open = false;
            }
            return;
        }
        theme::caption(ui, "Busca en Modrinth para tu versión y loader instalados. La instalación descarga código de terceros que se ejecutará al jugar.");
        ui.horizontal_wrapped(|ui| {
            let response = ui.add(egui::TextEdit::singleline(&mut self.mod_catalog_query)
                .hint_text("Nombre o función: Sodium, mapas…")
                .margin(egui::Margin::symmetric(10, 8))
                .desired_width(260.));
            let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if theme::button_state(ui, "Buscar mods", theme::Tone::Primary, egui::vec2(140., 36.),
                !self.version.is_empty()).clicked() || enter {
                self.send(json!({"action":"mod_catalog_search","query":self.mod_catalog_query,"version":self.version}));
            }
        });
        if self.busy && self.pending_action == "mod_catalog_search" {
            theme::caption(ui, "Buscando versiones de mods en Modrinth…");
        } else if self.last_error {
            theme::caption(ui, "No se completó la operación. Revisa el mensaje inferior y pulsa Buscar mods para reintentar.");
        } else if !self.mod_catalog_loaded {
            let loader = version_loader(&self.version).unwrap_or("tu loader");
            theme::panel().inner_margin(22).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new("Encuentra mods para tu versión")
                            .font(theme::display(18.)),
                    );
                    theme::caption(
                        ui,
                        &format!(
                            "Catálogo de Modrinth filtrado por Minecraft y {loader}. Busca por nombre o explora sin escribir nada."
                        ),
                    );
                    if theme::button_sized(
                        ui,
                        "Ver mods populares",
                        theme::Tone::Quiet,
                        egui::vec2(190., 38.),
                    )
                    .clicked()
                    {
                        self.send(json!({"action":"mod_catalog_search","query":"","version":self.version}));
                    }
                });
            });
        } else if self.mod_catalog_version != self.mod_target() {
            theme::caption(
                ui,
                "Has cambiado de perfil o versión. Vuelve a buscar antes de instalar.",
            );
        } else {
            theme::caption(
                ui,
                &format!(
                    "{} resultados · {}",
                    self.mod_catalog.len(),
                    self.mod_catalog_context
                ),
            );
            if self.mod_catalog.is_empty() {
                theme::caption(ui, "No hay resultados para esta búsqueda y configuración. Prueba otro nombre o cambia de versión.");
            }
        }
        let can_install = self.mod_catalog_loaded
            && self.mod_catalog_version == self.mod_target()
            && (!self.offline || !self.local_profile.is_empty());
        if let Some(item) = self.mod_install.clone() {
            ui.add_space(12.);
            theme::panel().inner_margin(14).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    egui::RichText::new(format!(
                        "Instalar {} en esta biblioteca?",
                        item["title"].as_str().unwrap_or("mod")
                    ))
                    .strong(),
                );
                theme::caption(ui, "También se instalarán sus dependencias obligatorias. Si el autor aún no publicó una versión estable para tu Minecraft, se usará la última beta o alfa. Los mods se ejecutan al iniciar Minecraft.");
                ui.horizontal_wrapped(|ui| {
                    if theme::button_state(
                        ui,
                        "Confirmar instalación",
                        theme::Tone::Primary,
                        egui::vec2(190., 38.),
                        can_install,
                    )
                    .clicked()
                    {
                        self.mod_install = None;
                        self.send(json!({"action":"mod_catalog_install","project_id":item["project_id"],"version":self.version}));
                    }
                    if theme::button(ui, "Cancelar", theme::Tone::Quiet).clicked() {
                        self.mod_install = None;
                    }
                });
            });
        }
        for item in self.mod_catalog.clone() {
            ui.add_space(12.);
            ui.separator();
            ui.label(egui::RichText::new(item["title"].as_str().unwrap_or("Mod")).strong());
            ui.add(egui::Label::new(item["description"].as_str().unwrap_or_default()).wrap());
            theme::caption(
                ui,
                &format!(
                    "Por {} · {} descargas",
                    item["author"].as_str().unwrap_or("autor sin indicar"),
                    downloads_label(item["downloads"].as_u64().unwrap_or(0))
                ),
            );
            ui.horizontal_wrapped(|ui| {
                if theme::button_state(
                    ui,
                    "Instalar mod",
                    theme::Tone::Quiet,
                    egui::vec2(140., 36.),
                    can_install,
                )
                .clicked()
                {
                    self.mod_install = Some(item.clone());
                }
                ui.hyperlink_to(
                    "Detalles y dependencias",
                    format!(
                        "https://modrinth.com/mod/{}",
                        item["project_id"].as_str().unwrap_or_default()
                    ),
                );
            });
        }
        if self.offline && self.local_profile.is_empty() {
            theme::caption(ui, "Crea un perfil en Jugar antes de instalar mods.");
            if theme::button(ui, "Ir a mi perfil", theme::Tone::Quiet).clicked() {
                self.tab = 0;
            }
        }
    }

    fn mod_target(&self) -> String {
        format!(
            "{}:{}:{}",
            self.offline,
            if self.offline {
                &self.local_profile
            } else {
                &self.account
            },
            self.version
        )
    }
}

/// Back-facing atlas view. Legacy skins reuse the right limbs, as in Minecraft.
fn skin_back_preview(ui: &mut egui::Ui, texture: &egui::TextureHandle, slim: bool, height: f32) {
    let scale = height / 32.;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(20. * scale, height + 12.), egui::Sense::hover());
    let origin = rect.left_top() + egui::vec2(2. * scale, 4.);
    let modern = texture.size()[1] == 64;
    let arm = if slim && modern { 3. } else { 4. };
    let part = |x: f32, y: f32, w: f32, h: f32, u: f32, v: f32| {
        ui.painter().image(
            texture.id(),
            egui::Rect::from_min_size(origin + egui::vec2(x, y) * scale, egui::vec2(w, h) * scale),
            egui::Rect::from_min_size(
                egui::pos2(u / 64., v / texture.size()[1] as f32),
                egui::vec2(w / 64., h / texture.size()[1] as f32),
            ),
            egui::Color32::WHITE,
        );
    };
    part(4., 0., 8., 8., 24., 8.);
    part(4., 8., 8., 12., 32., 20.);
    part(12., 8., arm, 12., 48. + arm, 20.);
    part(
        4. - arm,
        8.,
        arm,
        12.,
        if modern { 40. + arm } else { 48. + arm },
        if modern { 52. } else { 20. },
    );
    part(8., 20., 4., 12., 12., 20.);
    part(
        4.,
        20.,
        4.,
        12.,
        if modern { 28. } else { 12. },
        if modern { 52. } else { 20. },
    );
    if modern {
        part(4., 8., 8., 12., 32., 36.);
        part(12., 8., arm, 12., 48. + arm, 36.);
        part(4. - arm, 8., arm, 12., 56. + arm, 52.);
        part(8., 20., 4., 12., 12., 36.);
        part(4., 20., 4., 12., 12., 52.);
    }
    part(4., 0., 8., 8., 56., 8.);
}

fn load_skin_texture(ctx: &egui::Context, path: &str, id: &str) -> Option<egui::TextureHandle> {
    let file = std::fs::metadata(path).ok()?;
    if file.len() > 2 * 1024 * 1024 {
        return None;
    }
    let (width, height) = image::ImageReader::open(path)
        .ok()?
        .into_dimensions()
        .ok()?;
    if width != 64 || !matches!(height, 32 | 64) {
        return None;
    }
    let image = image::open(path).ok()?;
    let rgba = image.to_rgba8();
    Some(ctx.load_texture(
        id,
        egui::ColorImage::from_rgba_unmultiplied([64, image.height() as usize], rgba.as_raw()),
        egui::TextureOptions::NEAREST,
    ))
}

fn valid_local_name(name: &str) -> bool {
    let name = name.trim();
    (3..=16).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// A texture URL SkinsRestorer can download, matching the shape MineSkin returns.
fn valid_texture_url(url: &str) -> bool {
    const PREFIX: &str = "https://textures.minecraft.net/texture/";
    let Some(hash) = url.strip_prefix(PREFIX) else {
        return false;
    };
    (32..=64).contains(&hash.len())
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn skin_url_command(url: &str, slim: bool) -> String {
    format!(
        "/skin url \"{url}\" {}",
        if slim { "slim" } else { "classic" }
    )
}

/// Named custom skin command. Returns `None` when the name is invalid, so a
/// command never copies a name the server would reject.
fn skin_custom_command(name: &str, url: &str, slim: bool) -> Option<String> {
    let name = name.trim();
    if !valid_texture_url(url) || !valid_local_name(name) {
        return None;
    }
    Some(format!(
        "/sr createcustom {name} \"{url}\" {}",
        if slim { "slim" } else { "classic" }
    ))
}

/// Applies a named custom skin previously created with `/sr createcustom`.
fn skin_named_command(name: &str) -> Option<String> {
    let name = name.trim();
    valid_local_name(name).then(|| format!("/skin {name}"))
}

impl Launcher {
    fn ui(&mut self, ctx: &egui::Context) {
        self.drain(ctx);

        if !ctx.wants_keyboard_input()
            && self.delete.is_none()
            && self.mod_delete.is_none()
            && self.profile_delete.is_none()
        {
            for (index, key) in [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
            ]
            .iter()
            .enumerate()
            {
                if ctx.input(|i| i.modifiers.command && i.key_pressed(*key)) {
                    self.tab = index;
                }
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.delete = None;
            self.mod_delete = None;
            self.profile_delete = None;
        }

        theme::background(ctx);
        egui::SidePanel::left("navigation")
            .exact_width(220.)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .stroke(egui::Stroke::new(1_f32, theme::BORDER))
                    .inner_margin(egui::Margin::symmetric(14, 16)),
            )
            .show(ctx, |ui| {
                ui.add_space(8.);
                theme::brand(ui);
                ui.add_space(20.);
                for (index, name) in ["Jugar", "Versiones", "Skins", "Mods", "Ajustes"]
                    .iter()
                    .enumerate()
                {
                    if theme::nav(ui, index, self.tab == index, name) {
                        self.tab = index;
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(10.);
                    theme::panel()
                        .inner_margin(12)
                        .corner_radius(4)
                        .show(ui, |ui| {
                            ui.set_min_width(154.);
                            let name = if self.offline {
                                self.offline_name.trim()
                            } else {
                                &self.account
                            };
                            ui.horizontal(|ui| {
                                theme::icon(
                                    ui,
                                    if self.offline {
                                        icons::Icon::User
                                    } else {
                                        icons::Icon::Microsoft
                                    },
                                    16.,
                                    if self.offline {
                                        theme::MUTED
                                    } else {
                                        theme::SKY_BLUE
                                    },
                                );
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(if name.is_empty() {
                                            "Sin jugador"
                                        } else {
                                            name
                                        })
                                        .font(theme::display(14.))
                                        .color(theme::TEXT),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(name);
                            });
                            ui.label(
                                egui::RichText::new(if self.offline {
                                    "Identidad local"
                                } else if self.account.is_empty() {
                                    "Sin sesión Microsoft"
                                } else {
                                    "Microsoft conectado"
                                })
                                .size(11.)
                                .color(theme::MUTED),
                            );
                        });
                });
            });
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .stroke(egui::Stroke::new(1_f32, theme::BORDER))
                    .inner_margin(egui::Margin::symmetric(22, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let color = if self.busy {
                        theme::GOLD
                    } else if self.last_error {
                        theme::REDSTONE
                    } else {
                        theme::GRASS
                    };
                    theme::icon(
                        ui,
                        if self.busy {
                            icons::Icon::Refresh
                        } else if self.last_error {
                            icons::Icon::Warning
                        } else {
                            icons::Icon::Check
                        },
                        14.,
                        color,
                    );
                    ui.add(
                        egui::Label::new(egui::RichText::new(&self.status).size(13.).color(
                            if self.last_error {
                                theme::REDSTONE.lerp_to_gamma(theme::TEXT, 0.6)
                            } else {
                                theme::MUTED
                            },
                        ))
                        .wrap(),
                    );
                    if self.busy {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spinner();
                        });
                    }
                });
                if self.busy {
                    ui.add_space(6.);
                    theme::progress(ui, self.progress);
                }
            });
        if self.tab == 0 {
            egui::TopBottomPanel::bottom("launch_controls")
                .frame(
                    egui::Frame::new()
                        .fill(theme::PANEL)
                        .inner_margin(egui::Margin::symmetric(26, 16)),
                )
                .show(ctx, |ui| {
                    let blocker = self.play_blocker();
                    let installed = self.installed.contains(&self.version);
                    ui.horizontal(|ui| {
                        let button_width = 190.;
                        let details_width = (ui.available_width() - button_width - 16.).max(100.);
                        ui.allocate_ui_with_layout(
                            egui::Vec2::new(details_width, 48.),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                ui.set_min_width(details_width);
                                let version = if self.version.is_empty() {
                                    "Sin versión seleccionada".to_owned()
                                } else {
                                    version_label(&self.version)
                                };
                                let mode = if self.offline {
                                    format!(
                                        "Local · {}",
                                        if self.offline_name.trim().is_empty() {
                                            "sin nombre"
                                        } else {
                                            self.offline_name.trim()
                                        }
                                    )
                                } else if self.account.is_empty() {
                                    "Microsoft · sin conectar".to_owned()
                                } else {
                                    format!("Microsoft · {}", self.account)
                                };
                                let summary =
                                    format!("{version}  ·  {mode}  ·  {} GB de RAM", self.ram);
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&summary).strong().size(14.),
                                    )
                                    .truncate(),
                                )
                                .on_hover_text(&summary);
                                theme::caption(
                                    ui,
                                    blocker.unwrap_or(if self.skin_support_missing() {
                                        "Puedes jugar; tu skin guardada no se mostrará sin activarla en Skins."
                                    } else if installed {
                                        "Versión instalada. Lista para iniciar."
                                    } else {
                                        "Se descargará la versión antes de iniciar."
                                    }),
                                );
                            },
                        );
                        if theme::button_icon_state(
                            ui,
                            Some(icons::Icon::Play),
                            if self.busy {
                                "En curso…"
                            } else if installed {
                                "Jugar"
                            } else {
                                "Instalar y jugar"
                            },
                            theme::Tone::Primary,
                            egui::Vec2::new(button_width, 48.),
                            blocker.is_none(),
                        )
                        .clicked()
                        {
                            self.configured("play");
                        }
                    });
                });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(egui::Margin {
                left: 26,
                right: 26,
                top: 20,
                bottom: 14,
            }))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(("page_scroll", self.tab))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let busy = self.busy;
                        ui.add_enabled_ui(!busy, |ui| match self.tab {
                            0 => {
                                theme::hero(ui);
                                theme::block(
                                    ui,
                                    theme::GRASS,
                                    "Versión",
                                    "Minecraft oficial o una versión con Fabric, Forge, NeoForge o Quilt. Gestiona instalaciones y snapshots en Versiones. La primera descarga necesita Internet y puede ocupar varios GB.",
                                    |ui| {
                                        let selected_version = if self.version.is_empty() {
                                            "Elige una versión".to_owned()
                                        } else {
                                            version_label(&self.version)
                                        };
                                        ui.horizontal_wrapped(|ui| {
                                            egui::ComboBox::from_id_salt("version")
                                                .width(ui.available_width().min(320.))
                                                .selected_text(selected_version)
                                                .show_ui(ui, |ui| {
                                                    for (id, kind) in &self.versions {
                                                        if kind == "release" || self.snapshots || self.installed.contains(id) {
                                                            ui.selectable_value(
                                                                &mut self.version,
                                                                id.clone(),
                                                                id,
                                                            );
                                                        }
                                                    }
                                                    for id in &self.installed {
                                                        if !self
                                                            .versions
                                                            .iter()
                                                            .any(|v| &v.0 == id)
                                                        {
                                                            ui.selectable_value(
                                                                &mut self.version,
                                                                id.clone(),
                                                                id,
                                                            );
                                                        }
                                                    }
                                                });
                                            if self.installed.contains(&self.version) {
                                                theme::badge_icon(ui, icons::Icon::Check, "Instalada", theme::GRASS);
                                            }
                                            if self.snapshots {
                                                theme::badge_icon(ui, icons::Icon::Star, "Snapshots", theme::GOLD);
                                            }
                                        });
                                    },
                                );
                                ui.add_space(12.);
                                theme::block(
                                    ui,
                                    theme::SKY_BLUE,
                                    "Cuenta",
                                    "Elige cómo jugar; puedes cambiar cuando quieras. Microsoft permite servidores oficiales, skins de cuenta y logros. El perfil local no necesita cuenta ni configuración.",
                                    |ui| {
                                        ui.columns(2, |columns| {
                                            if theme::option_card(
                                                &mut columns[0],
                                                icons::Icon::Microsoft,
                                                !self.offline,
                                                "Cuenta Microsoft",
                                                "Servidores oficiales, skins y logros.",
                                            ) && self.offline
                                             {
                                                 self.offline = false;
                                                 self.confirmed_skin = None;
                                                 self.skin_path.clear();
                                                 self.texture = None;
                                                if self.account.is_empty() {
                                                    if self.client.trim().is_empty() {
                                                        self.send(json!({"action":"mods"}));
                                                    } else {
                                                        self.send(json!({"action":"session_restore","client_id":self.client}));
                                                    }
                                                } else {
                                                    self.send(json!({"action":"mods"}));
                                                }
                                            }
                                            if theme::option_card(
                                                &mut columns[1],
                                                icons::Icon::User,
                                                self.offline,
                                                "Perfil local",
                                                "Sin cuenta. Servidores compatibles.",
                                            ) && !self.offline
                                            {
                                                self.offline = true;
                                                if !self.local_profile.is_empty() {
                                                    self.send(json!({"action":"profile_select", "id":self.local_profile}));
                                                } else {
                                                    self.skin_path.clear();
                                                    self.texture = None;
                                                    self.send(json!({"action":"mods"}));
                                                }
                                            }
                                        });
                                        ui.add_space(10.);
                                        if self.offline {
                                            self.profile_controls(ui);
                                            ui.horizontal_wrapped(|ui| {
                                                if theme::button_icon_state(ui, Some(icons::Icon::User), "Elegir skin", theme::Tone::Quiet, egui::vec2(150., 38.), true).clicked() {
                                                    self.tab = 2;
                                                }
                                                theme::help(ui, "El nombre es tu identidad en el juego; no es una cuenta. La skin se elige en la pestaña Skins.");
                                            });
                                        } else if self.account.is_empty() {
                                            if self.client.trim().is_empty() {
                                                ui.horizontal_wrapped(|ui| {
                                                    if theme::button_icon_state(
                                                        ui,
                                                        Some(icons::Icon::Sliders),
                                                        "Abrir ajustes avanzados",
                                                        theme::Tone::Quiet,
                                                        egui::Vec2::new(220., 38.),
                                                        true,
                                                    ).clicked() {
                                                        self.tab = 4;
                                                    }
                                                    theme::help(ui, "Microsoft necesita un Client ID de tu aplicación de Azure, en Ajustes → avanzado. El modo local no requiere configuración.");
                                                });
                                            } else {
                                                if theme::button_icon_state(
                                                    ui,
                                                    Some(icons::Icon::Key),
                                                    "Iniciar sesión en el navegador",
                                                    theme::Tone::Quiet,
                                                    egui::Vec2::new(250., 38.),
                                                    true,
                                                )
                                                .clicked()
                                                {
                                                    self.send(json!({"action":"login", "client_id":self.client}));
                                                }
                                                if self.session_can_restore && theme::button_icon_state(
                                                    ui, Some(icons::Icon::Refresh), "Restaurar sesión guardada", theme::Tone::Quiet, egui::vec2(250., 38.), true,
                                                ).clicked() {
                                                    self.send(json!({"action":"session_restore","client_id":self.client}));
                                                }
                                                if self.session_can_forget && theme::button_icon_state(
                                                    ui, Some(icons::Icon::Trash), "Olvidar sesión guardada", theme::Tone::Quiet, egui::vec2(240., 38.), true,
                                                ).clicked() {
                                                    self.send(json!({"action":"logout"}));
                                                }
                                            }
                                        } else {
                                            ui.horizontal_wrapped(|ui| {
                                                theme::badge_icon(ui, icons::Icon::Check, "Sesión activa", theme::GRASS);
                                                if theme::button_icon_state(
                                                    ui,
                                                    Some(icons::Icon::Close),
                                                    "Cerrar sesión y olvidar",
                                                    theme::Tone::Quiet,
                                                    egui::Vec2::new(220., 38.),
                                                    true,
                                                )
                                                .clicked()
                                                {
                                                    self.send(json!({"action":"logout"}));
                                                }
                                                if !self.session_remembered && theme::button_icon_state(
                                                    ui, Some(icons::Icon::Save), "Guardar sesión", theme::Tone::Quiet, egui::vec2(160., 38.), true,
                                                ).clicked() {
                                                    self.send(json!({"action":"session_remember","client_id":self.client}));
                                                }
                                            });
                                        }
                                        if !self.offline && !self.session_notice.is_empty() {
                                            theme::caption(ui, &self.session_notice);
                                        }
                                        if !self.login_url.is_empty() {
                                            ui.add_space(8.);
                                            ui.horizontal(|ui| {
                                                ui.hyperlink_to(
                                                    "Abrir la página de Microsoft",
                                                    &self.login_url,
                                                );
                                                theme::help(
                                                    ui,
                                                    "Pega aquí la URL final completa, la que incluye code y state.",
                                                );
                                            });
                                            ui.add(
                                                egui::TextEdit::singleline(
                                                    &mut self.response_url,
                                                )
                                                .password(true)
                                                .margin(egui::Margin::symmetric(10, 8))
                                                .desired_width(f32::INFINITY),
                                            );
                                            if theme::button_icon_state(
                                                ui,
                                                Some(icons::Icon::Check),
                                                "Completar acceso",
                                                theme::Tone::Primary,
                                                egui::Vec2::new(200., 36.),
                                                !self.response_url.trim().is_empty(),
                                            )
                                            .clicked()
                                            {
                                                let url =
                                                    std::mem::take(&mut self.response_url);
                                                self.send(json!({"action":"complete_login", "url":url}));
                                            }
                                        }
                                    },
                                );
                            }
                            1 => {
                                theme::header(
                                    ui,
                                    "Versiones",
                                    "Instala y elige la versión base. Los loaders se añaden en Mods.",
                                );
                                theme::panel().inner_margin(14).show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.search)
                                                .hint_text("Buscar versión")
                                                .margin(egui::Margin::symmetric(10, 8))
                                                .desired_width(230.),
                                        );
                                        ui.checkbox(
                                            &mut self.snapshots,
                                            "Incluir snapshots e históricas",
                                        );
                                        ui.checkbox(&mut self.installed_only, "Solo instaladas");
                                                if theme::button_icon_state(
                                                    ui,
                                                    Some(icons::Icon::Refresh),
                                                    "Actualizar lista",
                                                    theme::Tone::Quiet,
                                                    egui::Vec2::new(150., 32.),
                                                    true,
                                                )
                                                .clicked()
                                                {
                                                    self.send(json!({"action":"versions"}));
                                                }
                                    });
                                });
                                ui.add_space(12.);
                                let mut versions = self.versions.clone();
                                for id in &self.installed {
                                    if !versions.iter().any(|v| &v.0 == id) {
                                        versions.push((id.clone(), "local".into()));
                                    }
                                }
                                let needle = self.search.to_lowercase();
                                let rows: Vec<(String, String, bool)> = versions
                                    .into_iter()
                                    .filter(|(id, kind)| {
                                        (self.snapshots
                                            || kind == "release"
                                            || kind == "local"
                                            || self.installed.contains(id))
                                            && (!self.installed_only || self.installed.contains(id))
                                            && id.to_lowercase().contains(&needle)
                                    })
                                    .map(|(id, kind)| {
                                        let installed = self.installed.contains(&id);
                                        (id, kind, installed)
                                    })
                                    .collect();
                                theme::caption(ui, &format!("{} {} · {} en este equipo", rows.len(), if rows.len() == 1 { "versión" } else { "versiones" }, self.installed.len()));
                                if rows.is_empty() {
                                    theme::panel().inner_margin(24).show(ui, |ui| {
                                        ui.vertical_centered(|ui| {
                                            ui.label(
                                                egui::RichText::new(
                                                    "Ninguna versión coincide.",
                                                )
                                                .font(theme::display(16.)),
                                            );
                                            theme::caption(
                                                ui,
                                                "Prueba otra búsqueda, desmarca Solo instaladas o activa los snapshots.",
                                            );
                                        });
                                    });
                                }
                                        for (id, kind, installed) in rows {
                                            theme::panel()
                                                .inner_margin(10)
                                                .corner_radius(4)
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        let label_width = (ui.available_width() - 260.).max(100.);
                                                        if ui.add_sized(
                                                            [label_width, 38.],
                                                            egui::Button::new(
                                                                egui::RichText::new(&id).strong(),
                                                            ).selected(self.version == id).truncate(),
                                                        ).on_hover_text(&id).clicked() {
                                                            self.version = id.clone();
                                                        }
                                                        let (label, color) = match kind.as_str()
                                                        {
                                                            "release" => {
                                                                ("Release", theme::GRASS)
                                                            }
                                                            "snapshot" => {
                                                                ("Snapshot", theme::GOLD)
                                                            }
                                                            "local" => ("Local", theme::STONE),
                                                            _ => ("Histórica", theme::MUTED),
                                                        };
                                                        theme::badge(ui, label, color);
                                                        ui.with_layout(
                                                            egui::Layout::right_to_left(
                                                                egui::Align::Center,
                                                            ),
                                                            |ui| {
                                                                if installed {
                                                                    if theme::button_sized(
                                                                        ui,
                                                                        "Desinstalar",
                                                                        theme::Tone::Danger,
                                                                        egui::Vec2::new(
                                                                            104., 30.,
                                                                        ),
                                                                    )
                                                                    .clicked()
                                                                    {
                                                                        self.delete =
                                                                            Some(id.clone());
                                                                    }
                                                                    theme::badge(
                                                                        ui,
                                                                        "Instalada",
                                                                        theme::GRASS,
                                                                    );
                                                                } else if theme::button_sized(
                                                                    ui,
                                                                    "Instalar",
                                                                    theme::Tone::Quiet,
                                                                    egui::Vec2::new(90., 30.),
                                                                )
                                                                .clicked()
                                                                {
                                                                    self.version = id.clone();
                                                                    self.configured("install");
                                                                }
                                                            },
                                                        );
                                                    });
                                                });
                                            ui.add_space(6.);
                                        }
                                theme::caption(
                                    ui,
                                    "Desinstalar elimina solo esa versión: los mundos y las bibliotecas compartidas se conservan.",
                                );
                            }
                            2 => {
                                 theme::header(
                                     ui,
                                     "Skins",
                                     "Tu personaje, con Microsoft o con un perfil local.",
                                 );
                                 ui.horizontal_wrapped(|ui| {
                                     if theme::chip(ui, !self.skin_store_open, "Mi skin") {
                                         self.skin_store_open = false;
                                     }
                                      if theme::chip(ui, self.skin_store_open, "Explorar skins") {
                                         self.skin_store_open = true;
                                     }
                                 });
                                 if self.offline {
                                     theme::caption(ui, &format!("Perfil local: {} · {}", if self.offline_name.is_empty() { "sin nombre" } else { &self.offline_name }, version_label(&self.version)));
                                 } else {
                                      theme::caption(ui, &format!("Cuenta Microsoft: {}", if self.account.is_empty() { "sin sesión" } else { &self.account }));
                                  }
                                  if theme::button_sized(ui, "Cambiar perfil o cuenta", theme::Tone::Quiet, egui::Vec2::new(190., 38.)).clicked() { self.tab = 0; }
                                 ui.add_space(8.);
                                 if self.skin_store_open {
                                     self.catalog_ui(ui);
                                     return;
                                 }
                                theme::block(
                                    ui,
                                    theme::SKY_BLUE,
                                    "Textura",
                                    "Archivos PNG de 64×64 o 64×32 píxeles.",
                                    |ui| {
                                        ui.horizontal(|ui| {
                                            if ui.add(
                                                egui::TextEdit::singleline(&mut self.skin_path)
                                                    .desired_width(
                                                        (ui.available_width() - 166.).max(120.),
                                                    )
                                                    .hint_text("Ruta del PNG")
                                                    .margin(egui::Margin::symmetric(10, 8)),
                                            ).on_hover_text("Elige un archivo o pega su ruta y pulsa Previsualizar").changed() {
                                                self.texture = None;
                                                self.clear_skin_publish();
                                            }
                                            if theme::button_sized(
                                                ui,
                                                "Elegir PNG",
                                                theme::Tone::Quiet,
                                                egui::Vec2::new(120., 32.),
                                            )
                                            .clicked()
                                            {
                                                if let Some(path) = rfd::FileDialog::new()
                                                    .add_filter("Skin PNG", &["png"])
                                                    .pick_file()
                                                {
                                                    self.skin_path =
                                                        path.display().to_string();
                                                    self.preview(ctx);
                                                }
                                            }
                                        });
                                        ui.horizontal_wrapped(|ui| {
                                            if theme::chip(ui, !self.slim, "Classic") {
                                                self.slim = false;
                                            }
                                            if theme::chip(ui, self.slim, "Slim") {
                                                self.slim = true;
                                            }
                                            if theme::button_sized(
                                                ui,
                                                "Previsualizar",
                                                theme::Tone::Quiet,
                                                egui::Vec2::new(130., 32.),
                                            )
                                            .clicked()
                                            {
                                                self.preview(ctx);
                                            }
                                            if theme::button_state(
                                                ui,
                                                "Ver skin actual",
                                                theme::Tone::Quiet,
                                                egui::Vec2::new(130., 32.),
                                                !self.offline && !self.account.is_empty(),
                                            )
                                            .clicked()
                                            {
                                                self.send(json!({"action":"skin_current"}));
                                            }
                                        });
                                    },
                                );
                                ui.add_space(12.);
                                match &self.texture {
                                    Some(texture) => {
                                        theme::panel().inner_margin(16).show(ui, |ui| {
                                            ui.horizontal_wrapped(|ui| {
                                                if self.skin_back {
                                                    skin_back_preview(ui, texture, self.slim, 160.);
                                                } else {
                                                    theme::skin_preview(ui, texture, self.slim, 160.);
                                                }
                                                ui.vertical(|ui| {
                                                    ui.horizontal_wrapped(|ui| {
                                                        if theme::chip(ui, !self.skin_back, "Frente") { self.skin_back = false; }
                                                        if theme::chip(ui, self.skin_back, "Espalda") { self.skin_back = true; }
                                                    });
                                                    ui.label(
                                                        egui::RichText::new(if self.slim {
                                                            "Modelo Slim"
                                                        } else {
                                                            "Modelo Classic"
                                                        })
                                                        .font(theme::display(15.)),
                                                    );
                                                    theme::caption(
                                                        ui,
                                                        if self.confirmed_skin.as_ref() == Some(&(self.skin_path.clone(), self.slim)) {
                                                            if self.offline {
                                                                if self.local_profiles.iter().any(|p| p["id"].as_str() == Some(&self.local_profile) && p["skin_support_version"].as_str() == Some(&self.version)) {
                                                                    "Guardada · soporte activado para esta versión."
                                                                } else { "Guardada · sin activar en esta versión." }
                                                            } else { "Confirmada en esta cuenta." }
                                                        } else { "Seleccionada · pendiente de guardar o aplicar." },
                                                    );
                                                    ui.add(egui::Label::new(std::path::Path::new(&self.skin_path).file_name().unwrap_or_default().to_string_lossy()).truncate()).on_hover_text(&self.skin_path);
                                                });
                                            });
                                        });
                                    }
                                    None => {
                                        theme::panel().inner_margin(26).show(ui, |ui| {
                                            ui.vertical_centered(|ui| {
                                                ui.label(
                                                    egui::RichText::new(
                                                        "Todavía no hay textura cargada.",
                                                    )
                                                    .font(theme::display(16.)),
                                                );
                                                theme::caption(
                                                    ui,
                                                    "Importa un PNG o descubre el catálogo en Explorar skins.",
                                                );
                                            });
                                        });
                                    }
                                }
                                ui.add_space(12.);
                                 if self.offline {
                                     ui.horizontal_wrapped(|ui| {
                                         let valid = valid_local_name(&self.offline_name) && !self.version.is_empty();
                                          if theme::button_state(ui, "Guardar y activar", theme::Tone::Primary,
                                              egui::vec2(170., 38.), valid && self.texture.is_some()).clicked() {
                                             self.configured("local_skin_enable");
                                         }
                                         if theme::button_state(ui, "Solo guardar", theme::Tone::Quiet,
                                              egui::vec2(130., 38.), valid && self.texture.is_some()).clicked() {
                                             self.configured("profile_save");
                                         }
                                         if theme::button_state(ui, "Quitar skin", theme::Tone::Quiet,
                                             egui::vec2(110., 38.), valid && !self.local_profile.is_empty()).clicked() {
                                             self.skin_path.clear();
                                             self.texture = None;
                                             self.clear_skin_publish();
                                             self.configured("profile_save");
                                         }
                                     });
                                      theme::caption(ui, "«Guardar y activar» guarda el perfil e instala CustomSkinLoader si tu versión y loader son compatibles. La skin se ve solo en tu cliente; los demás jugadores no la verán.");
                                      if !valid_local_name(&self.offline_name) || self.version.is_empty() {
                                          theme::caption(ui, "Primero elige una versión y un nombre válido en Jugar.");
                                      }
                                     theme::caption(ui, "Sin loader, usa «Solo guardar»: la skin queda en el perfil pero el juego no la mostrará.");
                                 } else {
                                 ui.horizontal_wrapped(|ui| {
                                    if theme::button_state(
                                        ui,
                                        "Aplicar a mi cuenta",
                                        theme::Tone::Primary,
                                        egui::Vec2::new(210., 38.),
                                        !self.account.is_empty() && self.texture.is_some(),
                                    )
                                    .clicked()
                                    {
                                        self.preview(ctx);
                                        if self.texture.is_some() {
                                            self.send(json!({"action":"skin_upload","path":self.skin_path,"variant":if self.slim {"slim"} else {"classic"}}));
                                        }
                                    }
                                    if theme::button_state(
                                        ui,
                                        "Restaurar predeterminada",
                                        theme::Tone::Quiet,
                                        egui::Vec2::new(210., 38.),
                                        !self.account.is_empty(),
                                    )
                                    .clicked()
                                    {
                                        self.send(json!({"action":"skin_reset"}));
                                    }
                                });
                                 if self.account.is_empty() {
                                    theme::caption(
                                        ui,
                                        "Necesitas una cuenta Microsoft con Minecraft Java para cambiar la skin.",
                                    );
                                 }
                                 }
                                 ui.add_space(12.);
                                 self.skinsrestorer_ui(ui);
                            }
                            3 => {
                                theme::header(
                                    ui,
                                    "Mods",
                                    "Organiza tu biblioteca y descubre mods para tu configuración.",
                                );
                                let shown_version = if self.version.is_empty() {
                                    "sin elegir".to_owned()
                                } else {
                                    version_label(&self.version)
                                };
                                theme::caption(ui, &format!("{} · Versión: {}", if self.offline { format!("Perfil local: {}", self.offline_name) } else { "Instalación Microsoft".into() }, shown_version));
                                ui.horizontal_wrapped(|ui| {
                                    if theme::chip(ui, !self.mod_store_open, "Biblioteca") { self.mod_store_open = false; }
                                    if theme::chip(ui, self.mod_store_open, "Descubrir en Modrinth") { self.mod_store_open = true; }
                                    if theme::button_sized(ui, "Cambiar versión", theme::Tone::Quiet, egui::Vec2::new(150., 38.)).clicked() { self.tab = 0; }
                                });
                                ui.add_space(8.);
                                if self.mod_store_open {
                                    self.mod_catalog_ui(ui);
                                    return;
                                }
                                theme::panel().inner_margin(14).show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        let active = self
                                            .mods
                                            .iter()
                                            .filter(|m| {
                                                m["enabled"].as_bool().unwrap_or(false)
                                            })
                                            .count();
                                        theme::badge(
                                            ui,
                                            &format!("{} instalados", self.mods.len()),
                                            theme::STONE,
                                        );
                                        theme::badge(
                                            ui,
                                            &format!("{} activos", active),
                                            theme::GRASS,
                                        );
                                                if theme::button_sized(
                                                    ui,
                                                    "Recargar lista",
                                                    theme::Tone::Quiet,
                                                    egui::Vec2::new(110., 32.),
                                                )
                                                .clicked()
                                                {
                                                    self.send(json!({"action":"mods"}));
                                                }
                                                if theme::button_state(
                                                    ui,
                                                    "Importar .jar",
                                                    theme::Tone::Quiet,
                                                    egui::Vec2::new(140., 32.),
                                                    !self.offline || !self.local_profile.is_empty(),
                                                )
                                                .clicked()
                                                {
                                                    if let Some(path) = rfd::FileDialog::new()
                                                        .add_filter("Mod Java", &["jar"])
                                                        .pick_file()
                                                    {
                                                        self.send(json!({"action":"mod_import", "path":path.display().to_string()}));
                                                    }
                                                }
                                    });
                                });
                                if self.offline && self.local_profile.is_empty() {
                                    theme::caption(ui, "Guarda primero tu perfil en Jugar para importar mods en su propia carpeta.");
                                }
                                theme::caption(ui, "Los .jar importados no se verifican: comprueba su versión, loader y dependencias antes de jugar.");
                                ui.add_space(12.);
                                let active_loader = version_loader(&self.version);
                                if let (Some(loader), false) = (active_loader, self.loader_setup_open) {
                                    theme::panel().inner_margin(12).show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal_wrapped(|ui| {
                                            theme::badge_icon(
                                                ui,
                                                icons::Icon::Check,
                                                &format!("{loader} activo"),
                                                theme::GRASS,
                                            );
                                            theme::caption(
                                                ui,
                                                "Los mods de esta biblioteca se cargan al iniciar esta versión.",
                                            );
                                            if theme::button_sized(
                                                ui,
                                                "Cambiar loader",
                                                theme::Tone::Quiet,
                                                egui::Vec2::new(132., 32.),
                                            )
                                            .clicked()
                                            {
                                                self.loader_setup_open = true;
                                            }
                                        });
                                    });
                                } else {
                                    theme::block(
                                        ui,
                                        theme::GOLD,
                                        "Instalar un loader",
                                        "Fabric, Forge, NeoForge o Quilt para la versión elegida. Cada mod debe ser compatible con su loader.",
                                        |ui| {
                                            let compatible_selection =
                                                self.versions.iter().any(|v| v.0 == self.version) || self.installed.contains(&self.version);
                                            egui::ComboBox::from_id_salt("mod_loader")
                                                .selected_text(LOADERS[self.loader].1)
                                                .show_ui(ui, |ui| {
                                                    for (index, (_, label)) in LOADERS.iter().enumerate() {
                                                        ui.selectable_value(&mut self.loader, index, *label);
                                                    }
                                                });
                                            ui.horizontal_wrapped(|ui| {
                                                let shown_version = if self.version.is_empty() {
                                                    "sin elegir".to_owned()
                                                } else {
                                                    version_label(&self.version)
                                                };
                                                theme::caption(
                                                    ui,
                                                    &format!("Versión actual: {shown_version}"),
                                                );
                                                if theme::button_state(
                                                    ui,
                                                    &format!("Instalar {}", LOADERS[self.loader].1),
                                                    theme::Tone::Quiet,
                                                    egui::Vec2::new(150., 32.),
                                                    compatible_selection,
                                                )
                                                .clicked()
                                                {
                                                    self.configured("loader_install");
                                                }
                                            });
                                            if !compatible_selection {
                                                theme::caption(
                                                    ui,
                                                    "Elige antes una versión oficial en Jugar o Versiones.",
                                                );
                                            }
                                            theme::caption(
                                                ui,
                                                "Se comprueba la compatibilidad durante la instalación. Algunos loaders necesitan Java. Importa las dependencias de cada mod por separado.",
                                            );
                                        },
                                    );
                                }
                                ui.add_space(12.);
                                ui.horizontal_wrapped(|ui| {
                                    for (filter, label) in [(ModFilter::All, "Todos"), (ModFilter::Enabled, "Activos"), (ModFilter::Disabled, "Inactivos")] {
                                        if theme::chip(ui, self.mod_filter == filter, label) { self.mod_filter = filter; }
                                    }
                                });
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.mod_search)
                                        .hint_text("Buscar en tus mods")
                                        .margin(egui::Margin::symmetric(10, 8))
                                        .desired_width(f32::INFINITY),
                                );
                                theme::caption(
                                    ui,
                                    if self.offline && !self.local_profile.is_empty() {
                                        "Biblioteca del perfil local seleccionado. Los cambios se aplican al próximo inicio."
                                    } else {
                                        "Biblioteca de la instalación original. Guarda un perfil local para tener sus propios mods."
                                    },
                                );
                                ui.add_space(8.);
                                if self.mods.is_empty() {
                                    theme::panel().inner_margin(26).show(ui, |ui| {
                                        ui.vertical_centered(|ui| {
                                            ui.label(
                                                egui::RichText::new(
                                                    "Tu biblioteca está vacía.",
                                                )
                                                .font(theme::display(16.)),
                                            );
                                            theme::caption(
                                                ui,
                                                "Descubre mods en Modrinth o importa un .jar. Después podrás activarlos o desactivarlos aquí.",
                                            );
                                        });
                                    });
                                }
                                let filtered: Vec<_> = self
                                    .mods
                                    .iter()
                                    .filter(|m| mod_matches(m, &self.mod_search, self.mod_filter))
                                    .cloned()
                                    .collect();
                                if filtered.is_empty() && !self.mods.is_empty() {
                                    theme::caption(
                                        ui,
                                        "Ningún mod coincide con estos filtros.",
                                    );
                                    if theme::button(ui, "Limpiar filtros", theme::Tone::Quiet).clicked() {
                                        self.mod_search.clear();
                                        self.mod_filter = ModFilter::All;
                                    }
                                }
                                for entry in filtered {
                                    let name = entry["name"].as_str().unwrap_or_default();
                                    let enabled =
                                        entry["enabled"].as_bool().unwrap_or(false);
                                    let label = name.trim_end_matches(".disabled");
                                    theme::panel()
                                        .inner_margin(10)
                                        .corner_radius(4)
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.vertical(|ui| {
                                                ui.add(egui::Label::new(
                                                    egui::RichText::new(label).strong()
                                                        .color(theme::TEXT),
                                                ).truncate()).on_hover_text(label);
                                                ui.horizontal_wrapped(|ui| {
                                                    theme::badge(
                                                        ui,
                                                        if enabled {
                                                            "Activo"
                                                        } else {
                                                            "Desactivado"
                                                        },
                                                        if enabled {
                                                            theme::GRASS
                                                        } else {
                                                            theme::STONE
                                                        },
                                                    );
                                                    theme::caption(
                                                        ui,
                                                        &format!(
                                                            "{:.1} MB",
                                                            entry["size"].as_f64().unwrap_or(0.)
                                                                / 1_048_576.
                                                        ),
                                                    );
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(egui::Align::Center),
                                                        |ui| {
                                                            if theme::button_sized(
                                                                ui,
                                                                if enabled {
                                                                    "Desactivar"
                                                                } else {
                                                                    "Activar"
                                                                },
                                                                theme::Tone::Quiet,
                                                                egui::Vec2::new(112., 30.),
                                                            )
                                                            .clicked()
                                                            {
                                                                self.send(json!({"action":"mod_toggle", "name":name}));
                                                            }
                                                            if theme::button_sized(
                                                                ui,
                                                                "Eliminar",
                                                                theme::Tone::Danger,
                                                                egui::Vec2::new(94., 30.),
                                                            )
                                                            .clicked()
                                                            {
                                                                self.mod_delete =
                                                                    Some(name.into());
                                                            }
                                                        },
                                                    );
                                                });
                                            });
                                        });
                                    ui.add_space(6.);
                                }
                            }
                            _ => {
                                theme::header(
                                    ui,
                                    "Ajustes",
                                    "Memoria, cuenta y motor. Se guardan en este equipo.",
                                );
                                theme::block(
                                    ui,
                                    theme::GRASS,
                                    "Memoria",
                                    "RAM asignada a Minecraft. Deja memoria para el sistema.",
                                    |ui| {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(
                                                egui::RichText::new(format!("{} GB", self.ram))
                                                    .font(theme::display(30.))
                                                    .color(theme::GRASS),
                                            );
                                            for amount in [2, 4, 8, 16] {
                                                if theme::chip(
                                                    ui,
                                                    self.ram == amount,
                                                    &format!("{amount} GB"),
                                                ) {
                                                    self.ram = amount;
                                                }
                                            }
                                        });
                                        ui.add_space(2.);
                                        ui.horizontal_wrapped(|ui| {
                                            if theme::button_sized(
                                                ui,
                                                "− 1 GB",
                                                theme::Tone::Quiet,
                                                egui::Vec2::new(86., 30.),
                                            )
                                            .clicked()
                                            {
                                                self.ram = self.ram.saturating_sub(1).max(1);
                                            }
                                            if theme::button_sized(
                                                ui,
                                                "+ 1 GB",
                                                theme::Tone::Quiet,
                                                egui::Vec2::new(86., 30.),
                                            )
                                            .clicked()
                                            {
                                                self.ram = (self.ram + 1).min(64);
                                            }
                                            theme::caption(
                                                ui,
                                                "Entre 1 y 64 GB; deja memoria libre para el sistema.",
                                            );
                                        });
                                    },
                                );
                                theme::block(
                                    ui,
                                    theme::SKY_BLUE,
                                    "Cuenta Microsoft (avanzado)",
                                    "Solo necesario para servidores oficiales y skins de cuenta.",
                                    |ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.client)
                                                .desired_width(f32::INFINITY)
                                                .margin(egui::Margin::symmetric(10, 8))
                                                .hint_text("Client ID de tu aplicación de Azure"),
                                        );
                                        theme::caption(
                                            ui,
                                            "Si no tienes uno, juega en modo local: no requiere cuenta ni configuración. La sesión se guarda en el llavero del sistema, nunca en estos ajustes.",
                                        );
                                    },
                                );
                                theme::block(
                                    ui,
                                    theme::GOLD,
                                    "Java",
                                    "Automático para jugar; Java instalado puede ser necesario para los loaders.",
                                    |ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut self.java)
                                                .desired_width(f32::INFINITY)
                                                .margin(egui::Margin::symmetric(10, 8))
                                                .hint_text("Automático (recomendado)"),
                                        );
                                        ui.horizontal_wrapped(|ui| {
                                            if theme::button_sized(
                                                ui,
                                                "Elegir ejecutable",
                                                theme::Tone::Quiet,
                                                egui::Vec2::new(160., 32.),
                                            )
                                            .clicked()
                                            {
                                                if let Some(path) =
                                                    rfd::FileDialog::new().pick_file()
                                                {
                                                    self.java = path.display().to_string();
                                                }
                                            }
                                            if theme::button_sized(
                                                ui,
                                                "Usar automático",
                                                theme::Tone::Quiet,
                                                egui::Vec2::new(150., 32.),
                                            )
                                            .clicked()
                                            {
                                                self.java.clear();
                                            }
                                        });
                                    },
                                );
                                theme::block(
                                    ui,
                                    theme::STONE,
                                    "Versiones y datos",
                                    "Snapshots y archivos guardados en tu equipo.",
                                    |ui| {
                                        ui.checkbox(
                                            &mut self.snapshots,
                                            "Mostrar snapshots y versiones históricas",
                                        );
                                        theme::caption(
                                            ui,
                                            "Datos en tu carpeta de usuario (.mi-minecraft-launcher). La última partida queda en game.log.",
                                        );
                                    },
                                );
                                ui.add_space(6.);
                                if theme::button(ui, "Guardar ajustes", theme::Tone::Primary)
                                    .clicked()
                                {
                                    self.configured("save");
                                }
                                ui.add_space(8.);
                                theme::caption(
                                    ui,
                                    "SkyLauncher 0.3 · by T4skor · proyecto independiente, no oficial de Mojang ni Microsoft.",
                                );
                            }
                        });
                    });
            });
        if let Some(id) = self.delete.clone() {
            egui::Modal::new(egui::Id::new("delete_version"))
                .frame(theme::panel())
                .show(ctx, |ui| {
                    ui.set_width(360.);
                    ui.heading("Desinstalar versión");
                    ui.label(format!("Se eliminarán los archivos de {id}."));
                    theme::caption(
                        ui,
                        "Los mundos, los recursos compartidos y las demás versiones se conservan.",
                    );
                    ui.add_space(8.);
                    ui.horizontal(|ui| {
                        if theme::button_sized(
                            ui,
                            "Desinstalar",
                            theme::Tone::Danger,
                            egui::Vec2::new(140., 38.),
                        )
                        .clicked()
                        {
                            self.send(json!({"action":"delete","version":id}));
                            self.delete = None;
                        }
                        if theme::button_sized(
                            ui,
                            "Cancelar",
                            theme::Tone::Quiet,
                            egui::Vec2::new(120., 38.),
                        )
                        .clicked()
                        {
                            self.delete = None;
                        }
                    });
                });
        }
        if let Some(name) = self.mod_delete.clone() {
            egui::Modal::new(egui::Id::new("delete_mod"))
                .frame(theme::panel())
                .show(ctx, |ui| {
                    ui.set_width(360.);
                    ui.heading("Eliminar mod");
                    ui.label(format!("¿Eliminar {name} de tu biblioteca?"));
                    theme::caption(ui, "El archivo original importado se conserva.");
                    ui.add_space(8.);
                    ui.horizontal(|ui| {
                        if theme::button_sized(
                            ui,
                            "Eliminar mod",
                            theme::Tone::Danger,
                            egui::Vec2::new(140., 38.),
                        )
                        .clicked()
                        {
                            self.send(json!({"action":"mod_delete","name":name}));
                            self.mod_delete = None;
                        }
                        if theme::button_sized(
                            ui,
                            "Cancelar",
                            theme::Tone::Quiet,
                            egui::Vec2::new(120., 38.),
                        )
                        .clicked()
                        {
                            self.mod_delete = None;
                        }
                    });
                });
        }
        if let Some(id) = self.profile_delete.clone() {
            egui::Modal::new(egui::Id::new("delete_profile"))
                .frame(theme::panel())
                .show(ctx, |ui| {
                    ui.set_width(360.);
                    ui.heading("Eliminar perfil local");
                    ui.label("Se eliminará el perfil guardado, no sus archivos.");
                    theme::caption(ui, "Sus mundos, mods y skins se conservan en la carpeta de datos. Esta acción no borra una cuenta Microsoft.");
                    ui.horizontal(|ui| {
                        if theme::button_sized(ui, "Cancelar", theme::Tone::Quiet, egui::vec2(120., 38.)).clicked() {
                            self.profile_delete = None;
                        }
                        if theme::button_sized(ui, "Eliminar perfil", theme::Tone::Danger, egui::vec2(150., 38.)).clicked() {
                            self.send(json!({"action":"profile_delete","id":id}));
                            self.profile_delete = None;
                        }
                    });
                });
        }
        if self.busy {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.ui(ctx);
    }
}

impl Drop for Launcher {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

const BACKEND_SOURCE: &str = include_str!("../backend.py");

fn python_path() -> std::ffi::OsString {
    if let Some(python) = std::env::var_os("MI_LAUNCHER_PYTHON") {
        return python;
    }
    // Distribution packages must never use a build machine's virtual environment.
    if let Some(python) = option_env!("SKYLAUNCHER_SYSTEM_PYTHON") {
        return python.into();
    }
    let exe_name = if cfg!(windows) {
        "python.exe"
    } else {
        "python"
    };
    let venv_bin = if cfg!(windows) { "Scripts" } else { "bin" };
    let mut roots = vec![std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))];
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
    {
        roots.insert(0, dir);
    }
    for root in roots {
        for relative in [
            std::path::Path::new("python").join(exe_name),
            std::path::Path::new(".venv").join(venv_bin).join(exe_name),
        ] {
            let candidate = root.join(relative);
            if candidate.is_file() {
                return candidate.into_os_string();
            }
        }
    }
    if cfg!(windows) {
        "python".into()
    } else {
        "python3".into()
    }
}

fn runtime_cache_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(base) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(base).join("SkyLauncher").join("runtime");
    }
    #[cfg(not(windows))]
    {
        if let Some(base) = std::env::var_os("XDG_CACHE_HOME") {
            return PathBuf::from(base).join("skylauncher").join("runtime");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".cache")
                .join("skylauncher")
                .join("runtime");
        }
    }
    std::env::temp_dir().join("skylauncher").join("runtime")
}

fn backend_script() -> io::Result<PathBuf> {
    backend_script_in(&runtime_cache_dir(), BACKEND_SOURCE)
}

/// Windows rejects a command line over 32767 characters, so the engine is
/// cached as a file instead of being passed through `python -c`.
fn backend_script_in(directory: &Path, source: &str) -> io::Result<PathBuf> {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    fs::create_dir_all(directory)?;
    let path = directory.join(format!("backend-{hash:016x}.py"));
    let is_file = || match fs::symlink_metadata(&path) {
        Ok(metadata) => Ok(metadata.file_type().is_file()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    };
    if is_file()? {
        if fs::read(&path)?.as_slice() == source.as_bytes() {
            return Ok(path);
        }
        fs::remove_file(&path)?;
    }
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => {
            file.write_all(source.as_bytes())?;
            Ok(path)
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists && is_file()? => Ok(path),
        Err(error) => Err(error),
    }
}

fn main() -> eframe::Result {
    let python = python_path();
    let script =
        backend_script().expect("No se pudo preparar el motor Python en la caché del usuario.");
    let mut command = Command::new(python);
    command
        .arg("-u")
        .arg(script)
        .env("PYTHONIOENCODING", "utf-8")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let child = command.spawn().expect(
        "No se pudo iniciar Python. Instala requirements.txt y configura MI_LAUNCHER_PYTHON.",
    );
    eframe::run_native(
        "SkyLauncher · by T4skor",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_app_id("skylauncher")
                .with_inner_size([1120., 820.])
                .with_min_inner_size([800., 600.]),
            ..Default::default()
        },
        Box::new(move |cc| Ok(Box::new(Launcher::new(cc, child)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_filters_combine_state_and_case_insensitive_query() {
        let enabled = json!({"name":"Sodium.jar","enabled":true});
        let disabled = json!({"name":"Sodium.jar.disabled","enabled":false});
        assert!(mod_matches(&enabled, " SOD ", ModFilter::Enabled));
        assert!(!mod_matches(&disabled, "sod", ModFilter::Enabled));
        assert!(mod_matches(&disabled, "", ModFilter::Disabled));
        assert!(!mod_matches(&enabled, "", ModFilter::Disabled));
        assert!(!mod_matches(&enabled, "map", ModFilter::All));
    }

    #[test]
    fn catalog_target_changes_with_version_profile_and_account() {
        let mut app = fixture();
        let initial = app.mod_target();
        app.version = "fabric-loader-0.16.14-1.21.5".into();
        assert_ne!(initial, app.mod_target());
        let version = app.mod_target();
        app.local_profile = "second".into();
        assert_ne!(version, app.mod_target());
        let local = app.mod_target();
        app.offline = false;
        app.account = "Alex".into();
        assert_ne!(local, app.mod_target());
    }

    #[test]
    fn saved_skin_without_support_does_not_block_vanilla() {
        let mut app = fixture();
        app.local_profile = "p1".into();
        app.local_profiles = vec![json!({"id":"p1","skin":"saved.png","skin_support_version":""})];
        assert!(app.play_blocker().is_none());
    }

    #[test]
    fn failed_skin_upload_never_confirms_selection() {
        let mut app = fixture();
        let (tx, rx) = mpsc::channel();
        app.events = rx;
        app.pending_action = "skin_upload".into();
        app.skin_path = "draft.png".into();
        tx.send(json!({"event":"error","text":"No se pudo aplicar"}))
            .unwrap();
        tx.send(json!({"event":"done"})).unwrap();
        app.drain(&egui::Context::default());
        assert!(app.confirmed_skin.is_none());
        app.pending_action = "skin_upload".into();
        app.last_error = false;
        tx.send(json!({"event":"done"})).unwrap();
        app.drain(&egui::Context::default());
        assert_eq!(app.confirmed_skin, Some(("draft.png".into(), false)));
    }

    #[test]
    fn skinsrestorer_commands_validate_texture_urls_and_names() {
        let url = "https://textures.minecraft.net/texture/1a4af718455d4aab528e7a61f86fa25e6a369d1768dcb13f7df319a713eb810b";
        assert!(valid_texture_url(url));
        assert!(!valid_texture_url("http://textures.minecraft.net/texture/1a4af718455d4aab528e7a61f86fa25e6a369d1768dcb13f7df319a713eb810b"));
        assert!(!valid_texture_url(
            "https://textures.minecraft.net/texture/not-a-hash"
        ));
        assert!(!valid_texture_url("https://example.com/texture/1a4af718455d4aab528e7a61f86fa25e6a369d1768dcb13f7df319a713eb810b"));
        assert_eq!(
            skin_url_command(url, false),
            format!("/skin url \"{url}\" classic")
        );
        assert_eq!(
            skin_url_command(url, true),
            format!("/skin url \"{url}\" slim")
        );
        assert_eq!(
            skin_custom_command("Alex", url, true).unwrap(),
            format!("/sr createcustom Alex \"{url}\" slim")
        );
        assert!(skin_custom_command("a b", url, false).is_none());
        assert!(skin_custom_command("Alex", "https://example.com/x.png", false).is_none());
        assert_eq!(skin_named_command("Alex").unwrap(), "/skin Alex");
        assert!(skin_named_command("a b").is_none());
    }

    #[test]
    fn skinsrestorer_warns_when_local_skin_overrides_server_skin() {
        let mut app = fixture();
        app.local_profile = "p1".into();
        app.local_profiles =
            vec![json!({"id":"p1","skin":"skin.png","skin_support_version":"1.21.5"})];
        assert!(app.local_skin_active());
        app.version = "1.20.1".into();
        assert!(!app.local_skin_active());

        app.version = "1.21.5".into();
        app.tab = 2;
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut output = None;
        for _ in 0..3 {
            output = Some(render(&mut app, &ctx, egui::vec2(1120., 2000.), vec![]));
        }
        let texts: Vec<String> = output
            .unwrap()
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|text| text.contains("CustomSkinLoader")));
    }

    #[test]
    fn skin_published_event_stores_url_variant_and_notice() {
        let mut app = fixture();
        let (tx, rx) = mpsc::channel();
        app.events = rx;
        app.pending_action = "skin_publish".into();
        app.slim = false;
        let url = "https://textures.minecraft.net/texture/1a4af718455d4aab528e7a61f86fa25e6a369d1768dcb13f7df319a713eb810b";
        tx.send(json!({"event":"skin_published","url":url,"variant":"slim"}))
            .unwrap();
        tx.send(json!({"event":"done"})).unwrap();
        app.drain(&egui::Context::default());
        assert_eq!(app.skin_publish_url, url);
        assert!(app.slim);
        assert!(!app.skin_publish_notice.is_empty());
        assert!(!app.busy);
    }

    #[test]
    fn mod_catalog_and_back_preview_fit_supported_widths() {
        let mut app = fixture();
        app.version = "fabric-loader-0.16.14-1.21.5".into();
        app.mod_store_open = true;
        app.mod_catalog_loaded = true;
        app.mod_catalog_version = app.mod_target();
        app.mod_catalog = vec![
            json!({"project_id":"sodium","title":"A long mod title ".repeat(10),"description":"A useful mod description. ".repeat(20),"author":"author","downloads":1234}),
        ];
        for width in [800., 1120.] {
            let ctx = egui::Context::default();
            theme::install(&ctx);
            app.tab = 3;
            for _ in 0..3 {
                let _ = render(&mut app, &ctx, egui::vec2(width, 600.), vec![]);
            }
            assert!(ctx.used_rect().right() <= width + 1.);
            for height in [32, 64] {
                let texture = ctx.load_texture(
                    "back",
                    egui::ColorImage::new([64, height], egui::Color32::WHITE),
                    egui::TextureOptions::NEAREST,
                );
                let _ = ctx.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| skin_back_preview(ui, &texture, true, 160.));
                });
            }
        }
    }

    #[test]
    fn backend_engine_is_cached_as_a_file_for_windows_command_limits() {
        let directory =
            std::env::temp_dir().join(format!("skylauncher-backend-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let first = backend_script_in(&directory, "print('motor')").unwrap();
        let second = backend_script_in(&directory, "print('motor')").unwrap();
        assert_eq!(first, second);
        assert_eq!(std::fs::read_to_string(&first).unwrap(), "print('motor')");
        assert!(std::fs::symlink_metadata(&first)
            .unwrap()
            .file_type()
            .is_file());
        let other = backend_script_in(&directory, "print('otro')").unwrap();
        assert_ne!(first, other);
        assert_eq!(std::fs::read_to_string(&other).unwrap(), "print('otro')");
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn packaged_python_uses_explicit_override_or_absolute_system_path() {
        if let Some(system) = option_env!("SKYLAUNCHER_SYSTEM_PYTHON") {
            assert!(std::path::Path::new(system).is_absolute());
            let expected = std::env::var_os("MI_LAUNCHER_PYTHON").unwrap_or_else(|| system.into());
            assert_eq!(python_path(), expected);
        }
    }

    fn fixture() -> Launcher {
        // A private inert child keeps UI tests independent of accounts, network and user data.
        let mut child = Command::new(python_path())
            .args(["-c", "import sys; sys.stdin.read()"])
            .stdin(Stdio::piped())
            .spawn()
            .expect("Python is required for the launcher");
        let input = child.stdin.take().unwrap();
        let (_, events) = mpsc::channel();
        Launcher {
            child,
            input,
            events,
            tab: 0,
            busy: false,
            status: "Catálogo actualizado.".into(),
            progress: 0.,
            client: String::new(),
            java: String::new(),
            ram: 4,
            version: "1.21.5".into(),
            snapshots: false,
            versions: vec![("1.21.5".into(), "release".into())],
            installed: vec!["fabric-loader-0.16.14-1.21.5".into()],
            search: String::new(),
            installed_only: false,
            account: String::new(),
            session_notice: String::new(),
            session_remembered: false,
            session_can_restore: false,
            session_can_forget: false,
            offline: true,
            offline_name: "Alex".into(),
            login_url: String::new(),
            response_url: String::new(),
            skin_path: String::new(),
            slim: false,
            texture: None,
            delete: None,
            mods: vec![
                json!({"name":format!("{}.jar", "long-mod-name-".repeat(12)), "enabled":true, "size":1048576}),
            ],
            mod_search: String::new(),
            mod_delete: None,
            last_error: false,
            local_profiles: vec![],
            local_profile: String::new(),
            profile_delete: None,
            loader: 0,
            skin_catalog: vec![],
            catalog_after: String::new(),
            catalog_loaded: false,
            skin_store_open: false,
            skin_back: false,
            confirmed_skin: None,
            skin_publish_url: String::new(),
            skin_publish_name: String::new(),
            skin_publish_notice: String::new(),
            pending_action: String::new(),
            mod_filter: ModFilter::All,
            mod_store_open: false,
            mod_catalog: vec![],
            mod_catalog_query: String::new(),
            mod_catalog_version: String::new(),
            mod_catalog_context: String::new(),
            mod_catalog_loaded: false,
            mod_install: None,
            loader_setup_open: false,
        }
    }

    fn render(
        app: &mut Launcher,
        ctx: &egui::Context,
        size: egui::Vec2,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                events,
                ..Default::default()
            },
            |ctx| app.ui(ctx),
        )
    }

    #[test]
    fn local_names_match_backend_constraints() {
        for name in ["Alex", "  Steve_123  ", "123", "abcdefghijklmnop"] {
            assert!(valid_local_name(name), "{name}");
        }
        for name in ["", "ab", "a".repeat(17).as_str(), "álex", "a b", "abc/"] {
            assert!(!valid_local_name(name), "{name}");
        }
    }

    #[test]
    fn launch_requires_version_identity_and_idle_backend() {
        let mut app = fixture();
        assert!(app.play_blocker().is_none());
        app.offline_name = "ab".into();
        assert!(app.play_blocker().is_some());
        app.offline = false;
        assert!(app.play_blocker().is_some());
        app.account = "Alex".into();
        assert!(app.play_blocker().is_none());
        app.busy = true;
        assert!(app.play_blocker().is_some());
        app.busy = false;
        app.version.clear();
        assert!(app.play_blocker().is_some());
    }

    #[test]
    fn all_tabs_fit_supported_window_widths() {
        let mut app = fixture();
        for size in [
            egui::vec2(800., 600.),
            egui::vec2(1120., 820.),
            egui::vec2(1440., 900.),
        ] {
            for tab in 0..5 {
                let ctx = egui::Context::default();
                theme::install(&ctx);
                app.tab = tab;
                for _ in 0..3 {
                    let _ = render(&mut app, &ctx, size, vec![]);
                }
                assert!(
                    ctx.used_rect().right() <= size.x + 1.,
                    "tab {tab}, size {size:?}: {:?}",
                    ctx.used_rect()
                );
            }
        }
    }

    #[test]
    fn launch_action_stays_visible_in_small_window() {
        let mut app = fixture();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let size = egui::vec2(800., 600.);
        let _ = render(&mut app, &ctx, size, vec![]);
        let output = render(&mut app, &ctx, size, vec![]);
        let button = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "Instalar y jugar" => {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                _ => None,
            })
            .expect("launch button must be rendered");
        assert!(egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(button));
        assert!(button.top() > 400.);
        assert!(
            button.left() > 550.,
            "launch action should align to the right: {button:?}"
        );
    }

    #[test]
    fn loaded_skin_and_long_profile_fit_small_window() {
        let mut app = fixture();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        app.account = "A_long_account_name".repeat(10);
        app.skin_path = format!("/home/player/{}/skin.png", "long-directory/".repeat(12));
        app.texture = Some(ctx.load_texture(
            "test_skin",
            egui::ColorImage::new([64, 64], egui::Color32::WHITE),
            egui::TextureOptions::NEAREST,
        ));
        app.offline = false;
        let size = egui::vec2(800., 600.);
        for tab in [0, 2, 3, 4] {
            app.tab = tab;
            for _ in 0..3 {
                let _ = render(&mut app, &ctx, size, vec![]);
            }
            assert!(
                ctx.used_rect().right() <= size.x + 1.,
                "tab {tab}: {:?}",
                ctx.used_rect()
            );
        }
    }

    #[test]
    fn account_choice_and_launch_summary_are_visible() {
        let mut app = fixture();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let size = egui::vec2(800., 600.);
        let mut output = None;
        for _ in 0..3 {
            output = Some(render(&mut app, &ctx, size, vec![]));
        }
        let texts: Vec<String> = output
            .unwrap()
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        for expected in ["Cuenta Microsoft", "Perfil local"] {
            assert!(
                texts.iter().any(|text| text == expected),
                "missing {expected}: {texts:?}"
            );
        }
        assert!(
            texts.iter().any(|text| text.contains("Local · Alex")),
            "launch summary should name the local player: {texts:?}"
        );
        assert!(texts.iter().any(|text| text.contains("4 GB de RAM")));
    }

    #[test]
    fn first_run_shows_create_profile_without_picker() {
        let mut app = fixture();
        app.local_profiles.clear();
        app.local_profile.clear();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let size = egui::vec2(800., 600.);
        let mut texts = vec![];
        for _ in 0..3 {
            let output = render(&mut app, &ctx, size, vec![]);
            texts = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                    _ => None,
                })
                .collect();
        }
        assert!(texts.iter().any(|text| text == "Crear perfil"));
        assert!(!texts.iter().any(|text| text == "Nuevo perfil"));
        assert!(!texts.iter().any(|text| text == "Guardar cambios"));
    }

    #[test]
    fn existing_profile_shows_picker_and_save_label() {
        let mut app = fixture();
        app.local_profiles = vec![json!({"id":"p1","name":"Alex","version":"1.21.5"})];
        app.local_profile = "p1".into();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let size = egui::vec2(800., 600.);
        let mut texts = vec![];
        for _ in 0..3 {
            let output = render(&mut app, &ctx, size, vec![]);
            texts = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                    _ => None,
                })
                .collect();
        }
        assert!(texts.iter().any(|text| text == "Nuevo perfil"));
        assert!(texts.iter().any(|text| text == "Guardar cambios"));
        assert!(texts.iter().any(|text| text == "Eliminar perfil"));
    }

    #[test]
    fn installed_filter_and_empty_search_show_correct_results() {
        let mut app = fixture();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        app.tab = 1;
        app.installed_only = true;
        let size = egui::vec2(800., 600.);
        let _ = render(&mut app, &ctx, size, vec![]);
        let output = render(&mut app, &ctx, size, vec![]);
        let has_text = |output: &egui::FullOutput, needle: &str| {
            output.shapes.iter().any(|shape| {
                matches!(
                    &shape.shape, egui::Shape::Text(text) if text.galley.text().contains(needle)
                )
            })
        };
        assert!(has_text(&output, "1 versión"));
        assert!(has_text(&output, "fabric-loader"));
        app.search = "no-such-version".into();
        let output = render(&mut app, &ctx, size, vec![]);
        assert!(has_text(&output, "Ninguna versión coincide."));
    }

    #[test]
    fn loader_detection_and_download_labels_are_user_facing() {
        assert_eq!(
            version_loader("fabric-loader-0.16.14-1.21.5"),
            Some("Fabric")
        );
        assert_eq!(version_loader("1.21.5-forge-57.0.1"), Some("Forge"));
        assert_eq!(version_loader("neoforge-21.1.80"), Some("NeoForge"));
        assert_eq!(version_loader("quilt-loader-0.27.0-1.21.5"), Some("Quilt"));
        assert_eq!(version_loader("1.21.5"), None);
        assert_eq!(
            version_label("fabric-loader-0.16.14-1.21.5"),
            "Fabric 1.21.5"
        );
        assert_eq!(version_label("1.21.5-forge-57.0.1"), "Forge 1.21.5");
        assert_eq!(version_label("neoforge-21.1.80"), "NeoForge 21.1.80");
        assert_eq!(version_label("1.21.5"), "1.21.5");
        assert_eq!(downloads_label(92000000), "92 M");
        assert_eq!(downloads_label(1500000), "1,5 M");
        assert_eq!(downloads_label(8450), "8 mil");
        assert_eq!(downloads_label(42), "42");
    }

    #[test]
    fn active_loader_collapses_the_installer_until_requested() {
        let mut app = fixture();
        app.version = "fabric-loader-0.16.14-1.21.5".into();
        app.tab = 3;
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let size = egui::vec2(800., 600.);
        let mut output = None;
        for _ in 0..3 {
            output = Some(render(&mut app, &ctx, size, vec![]));
        }
        let texts: Vec<String> = output
            .unwrap()
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|text| text == "Fabric activo"));
        assert!(!texts.iter().any(|text| text == "Instalar un loader"));
        app.loader_setup_open = true;
        let output = render(&mut app, &ctx, size, vec![]);
        assert!(output.shapes.iter().any(|shape| matches!(
            &shape.shape,
            egui::Shape::Text(text) if text.galley.text() == "Instalar un loader"
        )));
    }

    #[test]
    fn skin_catalog_fits_supported_windows_with_real_atlas_previews() {
        let mut app = fixture();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        app.tab = 2;
        app.skin_store_open = true;
        app.catalog_loaded = true;
        app.catalog_after = "a".repeat(32);
        app.skin_catalog = (0..8)
            .map(|index| CatalogSkin {
                id: format!("{index:032x}"),
                name: format!("Skin {index} {}", "long name ".repeat(10)),
                texture: Some(ctx.load_texture(
                    format!("catalog-{index}"),
                    egui::ColorImage::new([64, 64], egui::Color32::WHITE),
                    egui::TextureOptions::NEAREST,
                )),
            })
            .collect();
        for size in [
            egui::vec2(800., 600.),
            egui::vec2(1120., 820.),
            egui::vec2(1440., 900.),
        ] {
            for _ in 0..3 {
                let _ = render(&mut app, &ctx, size, vec![]);
            }
            assert!(
                ctx.used_rect().right() <= size.x + 1.,
                "{size:?}: {:?}",
                ctx.used_rect()
            );
        }
    }

    #[test]
    fn saved_profile_skin_warns_without_blocking_selected_version() {
        let mut app = fixture();
        app.local_profile = "a".repeat(32);
        app.local_profiles = vec![json!({
            "id":app.local_profile, "name":"Alex", "version":app.version,
            "skin":"skin.png", "skin_support_version":""
        })];
        assert!(app.play_blocker().is_none());
        assert!(app.skin_support_missing());
        app.local_profiles[0]["skin_support_version"] = json!(app.version);
        assert!(app.play_blocker().is_none());
        assert!(!app.skin_support_missing());
        app.version = "other-version".into();
        assert!(app.play_blocker().is_none());
        assert!(app.skin_support_missing());
        app.local_profiles[0]["skin"] = json!("");
        assert!(app.play_blocker().is_none());
    }

    #[test]
    fn selecting_profile_restores_ui_without_losing_microsoft_session() {
        let mut app = fixture();
        app.account = "MicrosoftPlayer".into();
        let (sender, receiver) = mpsc::channel();
        app.events = receiver;
        sender
            .send(json!({"event":"local_profile_selected", "value":{
                "id":"a".repeat(32), "name":"Steve", "version":"forge-1.21.1",
                "skin_path":"", "variant":"slim"
            }}))
            .unwrap();
        app.drain(&egui::Context::default());
        assert_eq!(app.offline_name, "Steve");
        assert_eq!(app.version, "forge-1.21.1");
        assert_eq!(app.local_profile, "a".repeat(32));
        assert!(app.offline && app.slim);
        assert_eq!(app.account, "MicrosoftPlayer");
        assert!(app.texture.is_none());
    }

    #[test]
    fn session_notices_and_retry_controls_fit_small_window() {
        let mut app = fixture();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        app.offline = false;
        app.client = "client".into();
        app.session_notice = "No se pudo abrir el llavero. Desbloquea tu almacén de credenciales y pulsa Restaurar sesión guardada.".into();
        app.session_can_restore = true;
        app.session_can_forget = true;
        for account in ["", "Player"] {
            app.account = account.into();
            for _ in 0..3 {
                let _ = render(&mut app, &ctx, egui::vec2(800., 600.), vec![]);
            }
            assert!(ctx.used_rect().right() <= 801., "{:?}", ctx.used_rect());
        }
    }

    #[test]
    fn session_state_events_update_only_public_status() {
        let mut app = fixture();
        let (sender, receiver) = mpsc::channel();
        app.events = receiver;
        sender
            .send(json!({
                "event":"session_state", "text":"Sesión guardada de forma segura.",
                "remembered":true, "can_restore":true, "can_forget":true
            }))
            .unwrap();
        app.drain(&egui::Context::default());
        assert!(app.session_remembered && app.session_can_restore && app.session_can_forget);
        assert_eq!(app.session_notice, "Sesión guardada de forma segura.");
        assert!(app.account.is_empty());
    }

    #[test]
    fn keyboard_switches_tabs_and_dismisses_confirmation() {
        let mut app = fixture();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let modifiers = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        let event = egui::Event::Key {
            key: egui::Key::Num5,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800., 600.),
            )),
            modifiers,
            events: vec![event],
            ..Default::default()
        };
        let _ = ctx.run(input.clone(), |ctx| app.ui(ctx));
        assert_eq!(app.tab, 4);
        app.delete = Some("1.21.5".into());
        app.mod_delete = Some("example.jar".into());
        input.events = vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        }];
        let _ = ctx.run(input, |ctx| app.ui(ctx));
        assert!(app.delete.is_none() && app.mod_delete.is_none());
    }
}

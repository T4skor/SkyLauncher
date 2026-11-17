use crate::icons::{self, Icon};
use eframe::egui::{
    self, Align2, Color32, FontData, FontFamily, FontId, Pos2, Rect, Response, RichText, Sense,
    Stroke, StrokeKind, Ui, Vec2,
};
use std::sync::Arc;

pub const NIGHT: Color32 = Color32::from_rgb(11, 17, 32);
pub const PANEL: Color32 = Color32::from_rgb(23, 31, 51);
pub const PANEL_HI: Color32 = Color32::from_rgb(31, 41, 66);
pub const BORDER: Color32 = Color32::from_rgb(51, 64, 92);
const CONTROL_BORDER: Color32 = Color32::from_rgb(112, 129, 157);
pub const TEXT: Color32 = Color32::from_rgb(233, 239, 247);
pub const MUTED: Color32 = Color32::from_rgb(170, 183, 204);
pub const GRASS: Color32 = Color32::from_rgb(116, 201, 107);
pub const GRASS_DARK: Color32 = Color32::from_rgb(63, 122, 57);
pub const GOLD: Color32 = Color32::from_rgb(242, 193, 78);
pub const SKY_BLUE: Color32 = Color32::from_rgb(122, 170, 226);
pub const STONE: Color32 = Color32::from_rgb(124, 135, 155);
pub const REDSTONE: Color32 = Color32::from_rgb(226, 105, 92);
pub const INK: Color32 = Color32::from_rgb(12, 19, 34);

pub fn display(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("display".into()))
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "tektur".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/Tektur-Medium.ttf"
        ))),
    );
    let mut chain = vec!["tektur".to_owned()];
    if let Some(fallback) = fonts.families.get(&FontFamily::Proportional) {
        chain.extend(fallback.iter().cloned());
    }
    fonts
        .families
        .insert(FontFamily::Name("display".into()), chain);
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.visuals.window_fill = PANEL;
    style.visuals.window_stroke = Stroke::new(1_f32, BORDER);
    style.visuals.window_corner_radius = 6.into();
    style.visuals.extreme_bg_color = Color32::from_rgb(9, 14, 26);
    style.visuals.faint_bg_color = Color32::from_white_alpha(4);
    style.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(116, 201, 107, 60);
    style.visuals.selection.stroke = Stroke::new(1_f32, GRASS);
    style.visuals.hyperlink_color = GRASS;
    for (widget, fill) in [
        (
            &mut style.visuals.widgets.noninteractive,
            Color32::from_rgb(18, 25, 42),
        ),
        (&mut style.visuals.widgets.inactive, PANEL_HI),
        (
            &mut style.visuals.widgets.hovered,
            Color32::from_rgb(39, 50, 78),
        ),
        (
            &mut style.visuals.widgets.active,
            Color32::from_rgb(45, 58, 88),
        ),
        (&mut style.visuals.widgets.open, PANEL_HI),
    ] {
        widget.corner_radius = 4.into();
        widget.bg_fill = fill;
        widget.weak_bg_fill = fill;
        widget.bg_stroke = Stroke::new(1_f32, CONTROL_BORDER);
        widget.fg_stroke = Stroke::new(1_f32, TEXT);
        widget.expansion = 0.;
    }
    style.visuals.widgets.noninteractive.bg_stroke =
        Stroke::new(1_f32, Color32::from_white_alpha(10));
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1_f32, STONE);
    // egui uses the active visuals for keyboard-focused built-in controls.
    style.visuals.widgets.active.bg_stroke = Stroke::new(2_f32, GOLD);
    style.visuals.widgets.open.bg_stroke = Stroke::new(1_f32, GRASS);
    style.spacing.item_spacing = Vec2::new(10., 10.);
    style.spacing.button_padding = Vec2::new(14., 9.);
    style.spacing.interact_size = Vec2::new(44., 38.);
    style.spacing.text_edit_width = 320.;
    style.spacing.slider_width = 220.;
    style.spacing.icon_width = 16.;
    style.spacing.icon_width_inner = 10.;
    style.spacing.icon_spacing = 8.;
    style.animation_time = 0.14;
    style
        .text_styles
        .insert(egui::TextStyle::Heading, display(26.));
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(15.));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(14.));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(13.));
    ctx.set_style(style);
}

fn gradient(painter: &egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(egui::Shape::mesh(mesh));
}

/// Cubo isométrico: tres caras planas, sin degradados, para el aire de bloque.
fn iso_cube(painter: &egui::Painter, center: Pos2, half: f32, accent: Color32, depth: f32) {
    let lift = half * 0.55;
    let top = center + Vec2::new(0., -lift);
    let right = center + Vec2::new(half, 0.);
    let bottom = center + Vec2::new(0., lift);
    let left = center + Vec2::new(-half, 0.);
    let down = Vec2::new(0., depth);
    painter.add(egui::Shape::convex_polygon(
        vec![top, right, bottom, left],
        accent,
        Stroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![left, bottom, bottom + down, left + down],
        accent.gamma_multiply(0.45),
        Stroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![bottom, right, right + down, bottom + down],
        accent.gamma_multiply(0.65),
        Stroke::NONE,
    ));
}

/// Bloque de hierba: cara superior verde y laterales de tierra con su borde.
fn iso_grass(painter: &egui::Painter, center: Pos2, half: f32, depth: f32) {
    let lift = half * 0.55;
    let top = center + Vec2::new(0., -lift);
    let right = center + Vec2::new(half, 0.);
    let bottom = center + Vec2::new(0., lift);
    let left = center + Vec2::new(-half, 0.);
    let down = Vec2::new(0., depth);
    let dirt = Color32::from_rgb(139, 98, 63);
    painter.add(egui::Shape::convex_polygon(
        vec![top, right, bottom, left],
        GRASS,
        Stroke::NONE,
    ));
    for (index, (a, b)) in [(left, bottom), (bottom, right)].iter().enumerate() {
        let band = (depth * 0.3).clamp(2., 6.);
        let strip = Vec2::new(0., band);
        painter.add(egui::Shape::convex_polygon(
            vec![*a, *b, *b + down, *a + down],
            GRASS.gamma_multiply(0.7),
            Stroke::NONE,
        ));
        painter.add(egui::Shape::convex_polygon(
            vec![*a + strip, *b + strip, *b + down, *a + down],
            if index == 0 {
                dirt.gamma_multiply(0.75)
            } else {
                dirt
            },
            Stroke::NONE,
        ));
    }
}

pub fn background(ctx: &egui::Context) {
    let rect = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    gradient(&painter, rect, NIGHT, Color32::from_rgb(21, 29, 54));
    // Cielo nocturno fijo: las mismas estrellas en cada arranque, sin ruido.
    for (x, y, size, alpha) in [
        (0.06, 0.12, 1., 80),
        (0.13, 0.31, 2., 55),
        (0.19, 0.08, 1., 70),
        (0.24, 0.53, 1., 45),
        (0.31, 0.22, 2., 70),
        (0.37, 0.68, 1., 50),
        (0.42, 0.14, 2., 60),
        (0.48, 0.44, 1., 40),
        (0.54, 0.90, 1., 45),
        (0.58, 0.29, 1., 65),
        (0.63, 0.61, 2., 40),
        (0.69, 0.19, 2., 80),
        (0.74, 0.47, 1., 55),
        (0.79, 0.77, 1., 40),
        (0.84, 0.34, 1., 60),
        (0.89, 0.09, 2., 70),
        (0.93, 0.56, 1., 45),
        (0.97, 0.24, 1., 55),
        (0.03, 0.86, 1., 40),
        (0.16, 0.94, 1., 45),
    ] {
        let center = rect.left_top() + Vec2::new(rect.width() * x, rect.height() * y);
        painter.rect_filled(
            Rect::from_center_size(center, Vec2::splat(size)),
            0.,
            Color32::from_white_alpha(alpha / 2),
        );
    }
}

pub fn panel() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1_f32, BORDER))
        .corner_radius(5)
        .inner_margin(18)
}

pub fn header(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).font(display(27.)).color(TEXT));
    ui.add_space(2.);
    ui.label(RichText::new(subtitle).size(14.).color(MUTED));
    ui.add_space(14.);
}

fn block_icon(ui: &mut Ui, accent: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let center = rect.center() + Vec2::new(0., size * 0.08);
    if accent == GRASS {
        iso_grass(ui.painter(), center, size * 0.34, size * 0.26);
    } else {
        iso_cube(ui.painter(), center, size * 0.34, accent, size * 0.26);
    }
}

/// Marca de ayuda discreta: el texto largo vive aquí, no en la página.
pub fn help(ui: &mut Ui, text: &str) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(22.), Sense::click());
    if response.clicked() {
        response.request_focus();
    }
    if response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        response.surrender_focus();
    }
    let painter = ui.painter();
    painter.circle_stroke(rect.center(), 7., Stroke::new(1.2_f32, MUTED));
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        "?",
        FontId::proportional(11.),
        MUTED,
    );
    if response.has_focus() {
        painter.rect_stroke(
            rect.expand(1.),
            4.,
            Stroke::new(2_f32, GOLD),
            StrokeKind::Outside,
        );
        response.show_tooltip_text(text);
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            format!("Ayuda: {text}"),
        )
    });
    response
        .on_hover_text(text)
        .on_hover_cursor(egui::CursorIcon::Help);
}

pub fn icon(ui: &mut Ui, icon: Icon, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    icons::paint(ui.painter(), rect, icon, color, 1.7);
}

pub fn block<R>(
    ui: &mut Ui,
    accent: Color32,
    title: &str,
    description: &str,
    contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    panel().inner_margin(16).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            block_icon(ui, accent, 22.);
            ui.label(RichText::new(title).font(display(16.)).color(TEXT));
            if !description.is_empty() {
                help(ui, description);
            }
        });
        ui.add_space(8.);
        contents(ui)
    })
}

pub fn caption(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(13.).color(MUTED));
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Primary,
    Quiet,
    Danger,
}

fn tone_colors(tone: Tone) -> (Color32, Color32, Color32, Color32) {
    match tone {
        Tone::Primary => (GRASS, Color32::from_rgb(138, 217, 127), GRASS_DARK, INK),
        Tone::Quiet => (
            PANEL_HI,
            Color32::from_rgb(40, 52, 80),
            Color32::from_rgb(13, 20, 35),
            TEXT,
        ),
        Tone::Danger => (
            Color32::from_rgb(90, 42, 51),
            Color32::from_rgb(116, 54, 62),
            Color32::from_rgb(42, 18, 24),
            Color32::from_rgb(255, 217, 212),
        ),
    }
}

pub fn button_sized(ui: &mut Ui, text: &str, tone: Tone, size: Vec2) -> Response {
    button_state(ui, text, tone, size, ui.is_enabled())
}

/// Botón con estado explícito: permite desactivarlo sin que egui difumine todo el `Ui`.
pub fn button_state(ui: &mut Ui, text: &str, tone: Tone, size: Vec2, enabled: bool) -> Response {
    button_icon_state(ui, None, text, tone, size, enabled)
}

/// Igual que `button_state`, con un icono vectorial a la izquierda del texto.
pub fn button_icon_state(
    ui: &mut Ui,
    icon: Option<Icon>,
    text: &str,
    tone: Tone,
    size: Vec2,
    enabled: bool,
) -> Response {
    let enabled = enabled && ui.is_enabled();
    let height = size.y.max(38.);
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(size.x, height),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let hover = if enabled {
        ui.ctx().animate_bool(response.id, response.hovered())
    } else {
        0.
    };
    let pressed = enabled && response.is_pointer_button_down_on();
    let (base, hover_fill, edge, ink) = tone_colors(tone);
    let (face, edge, ink, border) = if enabled {
        (
            base.lerp_to_gamma(hover_fill, hover * 0.7),
            edge,
            ink,
            if tone == Tone::Primary {
                edge
            } else {
                CONTROL_BORDER
            },
        )
    } else {
        (
            Color32::from_rgb(32, 42, 62),
            Color32::from_rgb(18, 25, 40),
            MUTED,
            BORDER,
        )
    };
    let lift = if pressed { 2. } else { 0. };
    let painter = ui.painter();
    let face_rect = Rect::from_min_max(
        rect.min + Vec2::new(0., lift),
        rect.max - Vec2::new(0., if pressed { 1. } else { 3. }),
    );
    painter.rect(
        rect,
        5.,
        edge,
        Stroke::new(1_f32, border),
        StrokeKind::Inside,
    );
    painter.rect_filled(face_rect, 5., face);
    painter.rect_stroke(
        face_rect,
        5.,
        Stroke::new(1_f32, border),
        StrokeKind::Inside,
    );
    if response.has_focus() {
        painter.rect_stroke(
            rect.expand(2.),
            6.,
            Stroke::new(2_f32, GOLD),
            StrokeKind::Outside,
        );
    }
    let color = if enabled { ink } else { MUTED };
    draw_label(painter, face_rect, icon, text, color);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, text));
    if enabled {
        response
            .on_hover_text(text)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

/// Preserve a readable type size; long labels truncate rather than shrink.
fn draw_label(painter: &egui::Painter, rect: Rect, icon: Option<Icon>, text: &str, color: Color32) {
    let painter = painter.with_clip_rect(painter.clip_rect().intersect(rect.shrink(4.)));
    let icon = icon.filter(|_| rect.width() >= 110.);
    let icon_size = 17.;
    let gap = 8.;
    let base = FontId::proportional(14.);
    let margin = if icon.is_some() { 28. } else { 20. };
    let room = (rect.width() - margin).max(1.) - if icon.is_some() { icon_size + gap } else { 0. };
    let mut job = egui::text::LayoutJob::simple(text.to_owned(), base, color, room.max(1.));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = painter.layout_job(job);
    let group = if icon.is_some() {
        icon_size + gap + galley.size().x
    } else {
        galley.size().x
    };
    let left = rect.center().x - group / 2.;
    if let Some(icon) = icon {
        let slot = Rect::from_center_size(
            Pos2::new(left + icon_size / 2., rect.center().y),
            Vec2::splat(icon_size),
        );
        icons::paint(&painter, slot, icon, color, 1.6);
        painter.galley(
            Pos2::new(
                left + icon_size + gap,
                rect.center().y - galley.size().y / 2.,
            ),
            galley,
            color,
        );
    } else {
        painter.galley(rect.center() - galley.size() / 2., galley, color);
    }
}

pub fn button(ui: &mut Ui, text: &str, tone: Tone) -> Response {
    let width = ui.available_width();
    button_sized(ui, text, tone, Vec2::new(width, 44.))
}

// Full-width variant retained as part of the shared component vocabulary.
#[allow(dead_code)]
pub fn button_icon(ui: &mut Ui, icon: Icon, text: &str, tone: Tone) -> Response {
    let width = ui.available_width();
    button_icon_state(
        ui,
        Some(icon),
        text,
        tone,
        Vec2::new(width, 44.),
        ui.is_enabled(),
    )
}

pub fn chip(ui: &mut Ui, selected: bool, text: &str) -> bool {
    let font = FontId::proportional(14.);
    let galley =
        ui.fonts(|fonts| fonts.layout_no_wrap(text.to_owned(), font, Color32::PLACEHOLDER));
    let size = Vec2::new(galley.size().x + 30., 36.);
    let enabled = ui.is_enabled();
    let (rect, response) = ui.allocate_exact_size(
        size,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let hover = if enabled {
        ui.ctx().animate_bool(response.id, response.hovered())
    } else {
        0.
    };
    let painter = ui.painter();
    let (fill, stroke, color) = if selected {
        (
            Color32::from_rgba_unmultiplied(116, 201, 107, 46),
            Stroke::new(1_f32, GRASS),
            GRASS,
        )
    } else {
        (
            PANEL_HI.lerp_to_gamma(Color32::from_rgb(42, 54, 82), hover),
            Stroke::new(1_f32, CONTROL_BORDER),
            if hover > 0.5 { TEXT } else { MUTED },
        )
    };
    painter.rect(rect, 4., fill, stroke, StrokeKind::Inside);
    if selected {
        painter.line_segment(
            [
                Pos2::new(rect.left() + 12., rect.bottom() - 4.),
                Pos2::new(rect.right() - 12., rect.bottom() - 4.),
            ],
            Stroke::new(1_f32, GRASS),
        );
    }
    painter.galley(
        rect.center() - galley.size() / 2.,
        galley,
        if enabled {
            color
        } else {
            color.gamma_multiply(0.6)
        },
    );
    if response.has_focus() {
        painter.rect_stroke(
            rect.expand(2.),
            5.,
            Stroke::new(2_f32, GOLD),
            StrokeKind::Outside,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, enabled, selected, text)
    });
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
        && enabled
}

/// Tarjeta de elección exclusiva: toda la superficie es clicable y el estado
/// seleccionado no depende solo del color (círculo relleno + borde + icono).
pub fn option_card(
    ui: &mut Ui,
    icon: Icon,
    selected: bool,
    title: &str,
    description: &str,
) -> bool {
    let width = ui.available_width();
    let text_width = (width - 88.).max(1.);
    let title_galley = ui.painter().layout(
        title.to_owned(),
        FontId::proportional(15.),
        TEXT,
        text_width,
    );
    let description_galley = ui.painter().layout(
        description.to_owned(),
        FontId::proportional(13.),
        MUTED,
        text_width,
    );
    let height = (28. + title_galley.size().y + 6. + description_galley.size().y).max(78.);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let hover = if ui.is_enabled() {
        ui.ctx().animate_bool(response.id, response.hovered())
    } else {
        0.
    };
    let painter = ui.painter();
    let fill = if selected {
        Color32::from_rgb(30, 49, 43)
    } else {
        PANEL_HI.lerp_to_gamma(Color32::from_rgb(40, 52, 80), hover)
    };
    painter.rect(
        rect,
        6.,
        fill,
        Stroke::new(
            if selected { 1.5_f32 } else { 1.0_f32 },
            if selected { GRASS } else { CONTROL_BORDER },
        ),
        StrokeKind::Inside,
    );
    let slot = Rect::from_center_size(
        Pos2::new(rect.left() + 28., rect.center().y),
        Vec2::splat(34.),
    );
    icons::paint(
        painter,
        slot,
        icon,
        if selected { GRASS } else { MUTED },
        1.8,
    );
    let dot = Pos2::new(rect.right() - 22., rect.top() + 22.);
    painter.circle_stroke(
        dot,
        7.,
        Stroke::new(1.5_f32, if selected { GRASS } else { STONE }),
    );
    if selected {
        painter.circle_filled(dot, 3.5, GRASS);
    }
    let text_x = rect.left() + 54.;
    let title_pos = Pos2::new(text_x, rect.top() + 14.);
    let description_pos = title_pos + Vec2::new(0., title_galley.size().y + 6.);
    painter.galley(title_pos, title_galley, TEXT);
    painter.galley(description_pos, description_galley, MUTED);
    if response.has_focus() {
        painter.rect_stroke(
            rect.expand(2.),
            7.,
            Stroke::new(2_f32, GOLD),
            StrokeKind::Outside,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::RadioButton,
            ui.is_enabled(),
            selected,
            title,
        )
    });
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
        && ui.is_enabled()
}

pub fn badge(ui: &mut Ui, text: &str, color: Color32) {
    let font = FontId::proportional(12.);
    let galley = ui.fonts(|fonts| fonts.layout_no_wrap(text.to_owned(), font, color));
    let (rect, _) = ui.allocate_exact_size(Vec2::new(galley.size().x + 16., 20.), Sense::hover());
    let painter = ui.painter();
    painter.rect(
        rect,
        3.,
        color.gamma_multiply(0.22),
        Stroke::new(1_f32, color.gamma_multiply(0.55)),
        StrokeKind::Inside,
    );
    painter.galley(rect.center() - galley.size() / 2., galley, color);
}

/// Insignia con icono: se usa para estados que conviene reconocer de un vistazo.
pub fn badge_icon(ui: &mut Ui, icon: Icon, text: &str, color: Color32) {
    let font = FontId::proportional(12.);
    let galley = ui.fonts(|fonts| fonts.layout_no_wrap(text.to_owned(), font, color));
    let width = galley.size().x + 38.;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 20.), Sense::hover());
    let painter = ui.painter();
    painter.rect(
        rect,
        3.,
        color.gamma_multiply(0.22),
        Stroke::new(1_f32, color.gamma_multiply(0.55)),
        StrokeKind::Inside,
    );
    let slot = Rect::from_center_size(
        Pos2::new(rect.left() + 13., rect.center().y),
        Vec2::splat(11.),
    );
    icons::paint(painter, slot, icon, color, 1.4);
    painter.galley(
        Pos2::new(rect.left() + 23., rect.center().y - galley.size().y / 2.),
        galley,
        color,
    );
}

pub fn progress(ui: &mut Ui, value: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 12.), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 3., Color32::from_black_alpha(120));
    painter.rect_stroke(rect, 3., Stroke::new(1_f32, BORDER), StrokeKind::Inside);
    let segments = 40;
    let gap = 2.;
    let cell = (rect.width() - gap) / segments as f32;
    let filled = (value.clamp(0., 1.) * segments as f32).round() as usize;
    for index in 0..filled {
        let x = rect.left() + index as f32 * cell + gap * 0.5;
        let segment = Rect::from_min_max(
            Pos2::new(x, rect.top() + 3.),
            Pos2::new(x + cell - gap, rect.bottom() - 3.),
        );
        painter.rect_filled(segment, 0., GRASS);
    }
}

fn nav_icon(painter: &egui::Painter, center: Pos2, index: usize, color: Color32) {
    let icon = match index {
        0 => Icon::Play,
        1 => Icon::Grid,
        2 => Icon::User,
        3 => Icon::Cube,
        _ => Icon::Sliders,
    };
    icons::paint(
        painter,
        Rect::from_center_size(center, Vec2::splat(18.)),
        icon,
        color,
        1.8,
    );
}

pub fn nav(ui: &mut Ui, index: usize, selected: bool, name: &str) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.), Sense::click());
    let response = response.on_hover_text(format!("Atajo: Ctrl+{}", index + 1));
    let hover = ui
        .ctx()
        .animate_bool(response.id, response.hovered() && !selected);
    let painter = ui.painter();
    let fill = if selected {
        Color32::from_rgb(37, 57, 51)
    } else {
        PANEL.lerp_to_gamma(PANEL_HI, hover)
    };
    painter.rect(
        rect,
        4.,
        fill,
        if selected {
            Stroke::new(1_f32, GRASS.gamma_multiply(0.65))
        } else {
            Stroke::NONE
        },
        StrokeKind::Inside,
    );
    let color = if selected || hover > 0.5 { TEXT } else { MUTED };
    let slot = Rect::from_center_size(rect.left_center() + Vec2::new(24., 0.), Vec2::splat(28.));
    nav_icon(
        painter,
        slot.center(),
        index,
        if selected { GRASS } else { color },
    );
    painter.text(
        rect.left_center() + Vec2::new(48., 0.),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(15.),
        color,
    );
    if response.has_focus() {
        painter.rect_stroke(
            rect.expand(1.),
            5.,
            Stroke::new(2_f32, GOLD),
            StrokeKind::Outside,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            name,
        )
    });
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

pub fn brand(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(52., 40.), Sense::hover());
    let painter = ui.painter();
    iso_grass(painter, rect.center() + Vec2::new(-8., 0.), 11., 9.);
    iso_cube(painter, rect.center() + Vec2::new(10., -7.), 6., GOLD, 5.);
    ui.label(RichText::new("SkyLauncher").font(display(21.)).color(TEXT));
    ui.label(RichText::new("by T4skor").size(12.).color(MUTED));
}

pub fn hero(ui: &mut Ui) {
    // Real layout rather than absolute text positions: headings wrap at small sizes.
    ui.horizontal(|ui| {
        let artwork = ui.available_width() >= 640.;
        let text_width = ui.available_width() - if artwork { 152. } else { 0. };
        ui.allocate_ui_with_layout(
            Vec2::new(text_width, 110.),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_min_width(text_width);
                ui.add_space(8.);
                ui.label(
                    RichText::new("Minecraft Java Edition")
                        .font(display(30.))
                        .color(TEXT),
                );
                ui.label(
                    RichText::new("Tu Minecraft, listo en un clic.")
                        .size(15.)
                        .color(MUTED),
                );
            },
        );
        if artwork {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(140., 110.), Sense::hover());
            let painter = ui.painter_at(rect);
            let base = rect.center();
            iso_grass(&painter, base + Vec2::new(-25., 0.), 30., 26.);
            iso_cube(&painter, base + Vec2::new(35., 20.), 18., STONE, 16.);
            iso_cube(&painter, base + Vec2::new(22., -32.), 11., GOLD, 10.);
        }
    });
    ui.add_space(8.);
}

/// Front-facing paper doll using the real skin atlas, including the outer layer.
pub fn skin_preview(ui: &mut Ui, texture: &egui::TextureHandle, slim: bool, height: f32) {
    let scale = height / 32.;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(20. * scale, height + 12.), Sense::hover());
    let origin = rect.left_top() + Vec2::new(2. * scale, 4.);
    let modern = texture.size()[1] == 64;
    let arm = if slim && modern { 3. } else { 4. };
    let part = |x: f32, y: f32, w: f32, h: f32, u: f32, v: f32| {
        let target = Rect::from_min_size(origin + Vec2::new(x, y) * scale, Vec2::new(w, h) * scale);
        let uv = Rect::from_min_size(
            Pos2::new(u / 64., v / texture.size()[1] as f32),
            Vec2::new(w / 64., h / texture.size()[1] as f32),
        );
        ui.painter().image(texture.id(), target, uv, Color32::WHITE);
    };
    part(4., 0., 8., 8., 8., 8.);
    part(4., 8., 8., 12., 20., 20.);
    part(4. - arm, 8., arm, 12., 44., 20.);
    part(
        12.,
        8.,
        arm,
        12.,
        if modern { 36. } else { 44. },
        if modern { 52. } else { 20. },
    );
    part(4., 20., 4., 12., 4., 20.);
    part(
        8.,
        20.,
        4.,
        12.,
        if modern { 20. } else { 4. },
        if modern { 52. } else { 20. },
    );
    if modern {
        part(4., 8., 8., 12., 20., 36.);
        part(4. - arm, 8., arm, 12., 44., 36.);
        part(12., 8., arm, 12., 52., 52.);
        part(4., 20., 4., 12., 4., 36.);
        part(8., 20., 4., 12., 4., 52.);
    }
    part(4., 0., 8., 8., 40., 8.);
}

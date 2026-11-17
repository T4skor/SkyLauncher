//! Iconos vectoriales dibujados con el pintor de egui.
//!
//! No se añaden fuentes ni dependencias: cada icono ocupa el rectángulo que se le
//! pasa y usa el color y el grosor indicados. Sirven como apoyo visual; el texto
//! o el tooltip siguen explicando la acción.

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};
use std::f32::consts::PI;

#[derive(Clone, Copy, PartialEq, Eq)]
// Keep the complete authored icon set available to the launcher's shared controls.
#[allow(dead_code)]
pub enum Icon {
    Play,
    Download,
    Check,
    User,
    Microsoft,
    Cube,
    Grid,
    Sliders,
    Search,
    Refresh,
    Trash,
    Plus,
    Folder,
    Eye,
    EyeOff,
    External,
    Info,
    Warning,
    Save,
    Key,
    Chip,
    Java,
    Close,
    Star,
}

pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32, width: f32) {
    let center = rect.center();
    let radius = rect.width().min(rect.height()) * 0.5;
    let stroke = Stroke::new(width.max(1.0), color);
    let at = |x: f32, y: f32| Pos2::new(center.x + x * radius, center.y + y * radius);
    let line = |a: Pos2, b: Pos2| painter.line_segment([a, b], stroke);
    let polyline = |points: Vec<Pos2>| painter.add(Shape::line(points, stroke));
    let polygon = |points: Vec<Pos2>, fill: Color32| {
        painter.add(Shape::convex_polygon(points, fill, Stroke::NONE));
    };

    match icon {
        Icon::Play => polygon(
            vec![at(-0.42, -0.55), at(0.58, 0.0), at(-0.42, 0.55)],
            color,
        ),
        Icon::Download => {
            line(at(0.0, -0.6), at(0.0, 0.18));
            polyline(vec![at(-0.36, -0.12), at(0.0, 0.26), at(0.36, -0.12)]);
            line(at(-0.55, 0.55), at(0.55, 0.55));
        }
        Icon::Check => {
            polyline(vec![at(-0.55, 0.0), at(-0.12, 0.42), at(0.55, -0.45)]);
        }
        Icon::User => {
            painter.circle_stroke(at(0.0, -0.28), 0.26 * radius, stroke);
            polyline(vec![
                at(-0.52, 0.55),
                at(-0.52, 0.24),
                at(-0.32, 0.08),
                at(0.0, 0.02),
                at(0.32, 0.08),
                at(0.52, 0.24),
                at(0.52, 0.55),
            ]);
        }
        Icon::Microsoft => {
            let size = 0.34 * radius;
            for (x, y) in [(-0.2, -0.2), (0.2, -0.2), (-0.2, 0.2), (0.2, 0.2)] {
                painter.rect_filled(
                    Rect::from_center_size(at(x, y), Vec2::splat(size)),
                    1.,
                    color,
                );
            }
        }
        Icon::Cube => {
            let top = vec![
                at(0.0, -0.6),
                at(0.55, -0.28),
                at(0.0, 0.04),
                at(-0.55, -0.28),
            ];
            let right = vec![at(0.0, 0.04), at(0.55, -0.28), at(0.55, 0.24), at(0.0, 0.6)];
            let left = vec![
                at(0.0, 0.04),
                at(-0.55, -0.28),
                at(-0.55, 0.24),
                at(0.0, 0.6),
            ];
            polygon(left, color.gamma_multiply(0.45));
            polygon(right, color.gamma_multiply(0.68));
            polygon(top, color);
        }
        Icon::Grid => {
            let cell = 0.36 * radius;
            for (x, y) in [(-0.27, -0.27), (0.27, -0.27), (-0.27, 0.27), (0.27, 0.27)] {
                painter.rect_stroke(
                    Rect::from_center_size(at(x, y), Vec2::splat(cell)),
                    1.,
                    stroke,
                    StrokeKind::Inside,
                );
            }
        }
        Icon::Sliders => {
            for (y, knob) in [(-0.42, -0.18), (0.0, 0.22), (0.42, -0.04)] {
                line(at(-0.55, y), at(0.55, y));
                painter.circle_filled(at(knob, y), 0.14 * radius, color);
            }
        }
        Icon::Search => {
            painter.circle_stroke(at(-0.1, -0.1), 0.36 * radius, stroke);
            line(at(0.18, 0.18), at(0.56, 0.56));
        }
        Icon::Refresh => {
            let points: Vec<Pos2> = (0..=28)
                .map(|index| {
                    let angle = -0.35 * PI + 1.65 * PI * index as f32 / 28.0;
                    at(0.42 * angle.cos(), 0.42 * angle.sin())
                })
                .collect();
            painter.add(Shape::line(points, stroke));
            let tip = -0.35 * PI;
            let tip_point = at(0.42 * tip.cos(), 0.42 * tip.sin());
            line(tip_point, tip_point + Vec2::new(-0.16, -0.22) * radius);
            line(tip_point, tip_point + Vec2::new(0.24, -0.12) * radius);
        }
        Icon::Trash => {
            line(at(-0.5, -0.38), at(0.5, -0.38));
            line(at(-0.16, -0.38), at(-0.16, -0.54));
            line(at(0.16, -0.38), at(0.16, -0.54));
            polyline(vec![at(-0.38, -0.24), at(-0.3, 0.55)]);
            polyline(vec![at(0.38, -0.24), at(0.3, 0.55)]);
            line(at(-0.3, 0.55), at(0.3, 0.55));
            line(at(-0.12, -0.2), at(-0.1, 0.4));
            line(at(0.12, -0.2), at(0.1, 0.4));
        }
        Icon::Plus => {
            line(at(0.0, -0.55), at(0.0, 0.55));
            line(at(-0.55, 0.0), at(0.55, 0.0));
        }
        Icon::Folder => {
            let points = vec![
                at(-0.55, -0.4),
                at(-0.12, -0.4),
                at(-0.02, -0.2),
                at(0.55, -0.2),
                at(0.55, 0.48),
                at(-0.55, 0.48),
                at(-0.55, -0.4),
            ];
            painter.add(Shape::closed_line(points, stroke));
        }
        Icon::Eye | Icon::EyeOff => {
            let points: Vec<Pos2> = (0..=26)
                .map(|index| {
                    let angle = 2.0 * PI * index as f32 / 26.0;
                    at(0.56 * angle.cos(), 0.32 * angle.sin())
                })
                .collect();
            painter.add(Shape::closed_line(points, stroke));
            painter.circle_filled(center, 0.15 * radius, color);
            if icon == Icon::EyeOff {
                line(at(-0.6, -0.6), at(0.6, 0.6));
            }
        }
        Icon::External => {
            painter.rect_stroke(
                Rect::from_center_size(at(-0.12, 0.12), Vec2::splat(0.72 * radius)),
                1.,
                stroke,
                StrokeKind::Inside,
            );
            line(at(0.06, -0.06), at(0.58, -0.58));
            polyline(vec![at(0.24, -0.58), at(0.58, -0.58), at(0.58, -0.24)]);
        }
        Icon::Info => {
            painter.circle_stroke(center, 0.55 * radius, stroke);
            line(at(0.0, -0.05), at(0.0, 0.32));
            painter.circle_filled(at(0.0, -0.3), 0.06 * radius, color);
        }
        Icon::Warning => {
            painter.add(Shape::closed_line(
                vec![at(0.0, -0.56), at(0.58, 0.48), at(-0.58, 0.48)],
                stroke,
            ));
            line(at(0.0, -0.1), at(0.0, 0.2));
            painter.circle_filled(at(0.0, 0.36), 0.06 * radius, color);
        }
        Icon::Save => {
            painter.rect_stroke(
                Rect::from_center_size(center, Vec2::splat(1.0 * radius)),
                1.,
                stroke,
                StrokeKind::Inside,
            );
            painter.rect_stroke(
                Rect::from_center_size(at(0.0, -0.14), Vec2::new(0.42 * radius, 0.3 * radius)),
                0.,
                stroke,
                StrokeKind::Inside,
            );
            painter.rect_stroke(
                Rect::from_center_size(at(0.0, 0.26), Vec2::new(0.5 * radius, 0.28 * radius)),
                0.,
                stroke,
                StrokeKind::Inside,
            );
        }
        Icon::Key => {
            painter.circle_stroke(at(-0.3, 0.0), 0.24 * radius, stroke);
            line(at(-0.06, 0.0), at(0.56, 0.0));
            line(at(0.28, 0.0), at(0.28, 0.24));
            line(at(0.44, 0.0), at(0.44, 0.18));
        }
        Icon::Chip => {
            painter.rect_stroke(
                Rect::from_center_size(center, Vec2::splat(0.72 * radius)),
                1.,
                stroke,
                StrokeKind::Inside,
            );
            painter.rect_stroke(
                Rect::from_center_size(center, Vec2::splat(0.34 * radius)),
                1.,
                stroke,
                StrokeKind::Inside,
            );
            for offset in [-0.24, 0.0, 0.24] {
                line(at(offset, -0.58), at(offset, -0.38));
                line(at(offset, 0.38), at(offset, 0.58));
                line(at(-0.58, offset), at(-0.38, offset));
                line(at(0.38, offset), at(0.58, offset));
            }
        }
        Icon::Java => {
            painter.rect_stroke(
                Rect::from_min_max(at(-0.34, -0.2), at(0.38, 0.46)),
                3.,
                stroke,
                StrokeKind::Inside,
            );
            painter.circle_stroke(at(0.42, 0.1), 0.2 * radius, stroke);
            for (x, y) in [(-0.16, -0.42), (0.04, -0.5), (0.2, -0.4)] {
                line(at(x, y), at(x + 0.08, y - 0.16));
            }
        }
        Icon::Close => {
            line(at(-0.5, -0.5), at(0.5, 0.5));
            line(at(0.5, -0.5), at(-0.5, 0.5));
        }
        Icon::Star => {
            let points: Vec<Pos2> = (0..10)
                .map(|index| {
                    let angle = -PI / 2.0 + index as f32 * PI / 5.0;
                    let length = if index % 2 == 0 { 0.6 } else { 0.26 };
                    at(length * angle.cos(), length * angle.sin())
                })
                .collect();
            painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
        }
    }
}

//! Lucide icons (<https://lucide.dev>, ISC licence; Feather-derived parts MIT)
//! for the deck panel, drawn in-house: each icon is a handful of polylines on
//! Lucide's 24 x 24 grid, scaled into a rect and stroked in the caller's
//! colour. No SVG parser, no asset files, no dependency.

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Shape, Stroke, pos2};

/// The icons the deck panel uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Icon {
    CircleDot,
    Play,
    Pause,
    LogIn,
    LogOut,
    Repeat,
    Lock,
    Timer,
}

/// One drawable path on the 24 x 24 grid.
pub(crate) struct Path {
    pub(crate) points: Vec<Pos2>,
    pub(crate) closed: bool,
}

fn line(points: &[(f32, f32)]) -> Path {
    Path {
        points: points.iter().map(|&(x, y)| pos2(x, y)).collect(),
        closed: false,
    }
}

/// Points of an arc around `(cx, cy)`, angles in degrees, y pointing down.
fn arc(out: &mut Vec<Pos2>, cx: f32, cy: f32, r: f32, from: f32, to: f32) {
    let steps = (((to - from).abs() / 15.0).ceil() as usize).max(2);
    for i in 0..=steps {
        let a = (from + (to - from) * i as f32 / steps as f32).to_radians();
        out.push(pos2(cx + r * a.cos(), cy + r * a.sin()));
    }
}

fn circle(cx: f32, cy: f32, r: f32) -> Path {
    let mut points = Vec::new();
    arc(&mut points, cx, cy, r, 0.0, 345.0);
    Path {
        points,
        closed: true,
    }
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Path {
    let mut points = Vec::new();
    arc(&mut points, x + w - r, y + r, r, -90.0, 0.0);
    arc(&mut points, x + w - r, y + h - r, r, 0.0, 90.0);
    arc(&mut points, x + r, y + h - r, r, 90.0, 180.0);
    arc(&mut points, x + r, y + r, r, 180.0, 270.0);
    Path {
        points,
        closed: true,
    }
}

impl Icon {
    /// The icon's paths on Lucide's 24 x 24 grid.
    pub(crate) fn paths(self) -> Vec<Path> {
        match self {
            Icon::CircleDot => vec![circle(12.0, 12.0, 10.0), circle(12.0, 12.0, 1.0)],
            Icon::Play => vec![Path {
                points: vec![pos2(6.0, 3.0), pos2(20.0, 12.0), pos2(6.0, 21.0)],
                closed: true,
            }],
            Icon::Pause => vec![
                rounded_rect(14.0, 3.0, 5.0, 18.0, 1.0),
                rounded_rect(5.0, 3.0, 5.0, 18.0, 1.0),
            ],
            Icon::LogIn => {
                let mut bracket = vec![pos2(15.0, 3.0), pos2(19.0, 3.0)];
                arc(&mut bracket, 19.0, 5.0, 2.0, -90.0, 0.0);
                arc(&mut bracket, 19.0, 19.0, 2.0, 0.0, 90.0);
                bracket.push(pos2(15.0, 21.0));
                vec![
                    Path {
                        points: bracket,
                        closed: false,
                    },
                    line(&[(10.0, 17.0), (15.0, 12.0), (10.0, 7.0)]),
                    line(&[(15.0, 12.0), (3.0, 12.0)]),
                ]
            }
            Icon::LogOut => {
                let mut bracket = vec![pos2(9.0, 21.0), pos2(5.0, 21.0)];
                arc(&mut bracket, 5.0, 19.0, 2.0, 90.0, 180.0);
                arc(&mut bracket, 5.0, 5.0, 2.0, 180.0, 270.0);
                bracket.push(pos2(9.0, 3.0));
                vec![
                    Path {
                        points: bracket,
                        closed: false,
                    },
                    line(&[(16.0, 17.0), (21.0, 12.0), (16.0, 7.0)]),
                    line(&[(21.0, 12.0), (9.0, 12.0)]),
                ]
            }
            Icon::Repeat => {
                let mut upper = vec![pos2(3.0, 11.0)];
                arc(&mut upper, 7.0, 10.0, 4.0, 180.0, 270.0);
                upper.push(pos2(21.0, 6.0));
                let mut lower = vec![pos2(21.0, 13.0)];
                arc(&mut lower, 17.0, 14.0, 4.0, 0.0, 90.0);
                lower.push(pos2(3.0, 18.0));
                vec![
                    line(&[(17.0, 2.0), (21.0, 6.0), (17.0, 10.0)]),
                    Path {
                        points: upper,
                        closed: false,
                    },
                    line(&[(7.0, 22.0), (3.0, 18.0), (7.0, 14.0)]),
                    Path {
                        points: lower,
                        closed: false,
                    },
                ]
            }
            Icon::Lock => {
                let mut shackle = vec![pos2(7.0, 11.0), pos2(7.0, 7.0)];
                arc(&mut shackle, 12.0, 7.0, 5.0, 180.0, 360.0);
                shackle.push(pos2(17.0, 11.0));
                vec![
                    rounded_rect(3.0, 11.0, 18.0, 11.0, 2.0),
                    Path {
                        points: shackle,
                        closed: false,
                    },
                ]
            }
            Icon::Timer => vec![
                line(&[(10.0, 2.0), (14.0, 2.0)]),
                line(&[(12.0, 14.0), (15.0, 11.0)]),
                circle(12.0, 14.0, 8.0),
            ],
        }
    }

    /// Stroke width in pixels at `size` (Lucide's 2 on a 24 grid).
    pub(crate) fn stroke_width(size: f32) -> f32 {
        (2.0 * size / 24.0).max(1.25)
    }
}

/// Paint `icon` centred in `rect` (as large as fits) with `colour`.
pub(crate) fn paint(painter: &Painter, icon: Icon, rect: Rect, colour: Color32) {
    let size = rect.width().min(rect.height());
    let scale = size / 24.0;
    let origin = rect.center() - egui::vec2(12.0, 12.0) * scale;
    let stroke = Stroke::new(Icon::stroke_width(size), colour);
    for path in icon.paths() {
        let points: Vec<Pos2> = path
            .points
            .iter()
            .map(|p| origin + p.to_vec2() * scale)
            .collect();
        painter.add(if path.closed {
            Shape::closed_line(points, stroke)
        } else {
            Shape::line(points, stroke)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_stays_inside_the_lucide_grid() {
        for icon in [
            Icon::CircleDot,
            Icon::Play,
            Icon::Pause,
            Icon::LogIn,
            Icon::LogOut,
            Icon::Repeat,
            Icon::Lock,
            Icon::Timer,
        ] {
            let paths = icon.paths();
            assert!(!paths.is_empty(), "{icon:?}");
            for path in paths {
                assert!(path.points.len() >= 2, "{icon:?}");
                for p in path.points {
                    assert!(
                        (0.0..=24.0).contains(&p.x) && (0.0..=24.0).contains(&p.y),
                        "{icon:?} point {p:?} outside the 24 x 24 grid"
                    );
                }
            }
        }
    }
}

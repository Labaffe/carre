//! Formes géométriques 2D génériques et tests d'intersection.
//!
//! Utilisé par tous les modules qui ont besoin d'un test de collision /
//! détection : hitbox de projectile, hitbox d'ennemi/astéroïde, zone de
//! détection du joueur, etc.

use bevy::prelude::*;

/// Forme géométrique 2D : cercle ou rectangle orienté.
#[derive(Clone)]
pub enum Shape {
    /// Cercle simple (rayon).
    Circle(f32),
    /// Rectangle orienté (demi-longueur dans l'axe, demi-largeur perpendiculaire).
    Rect { half_length: f32, half_width: f32 },
}

/// Test d'intersection entre une `Shape` (orientée par `rot`) et un cercle.
///
/// `shape_pos` / `shape_rot` : position et rotation de la `Shape` (typiquement
/// la `Transform` de l'entité qui la porte).
/// `circle_pos` / `circle_radius` : cible.
pub fn shape_hits_circle(
    shape_pos: Vec2,
    shape_rot: Quat,
    shape: &Shape,
    circle_pos: Vec2,
    circle_radius: f32,
) -> bool {
    match shape {
        Shape::Circle(r) => shape_pos.distance(circle_pos) < *r + circle_radius,
        Shape::Rect {
            half_length,
            half_width,
        } => {
            let angle = shape_rot.to_euler(EulerRot::ZYX).0;
            obb_circle_collision(
                shape_pos,
                angle,
                *half_length,
                *half_width,
                circle_pos,
                circle_radius,
            )
        }
    }
}

/// Test d'intersection entre 2 `Shape` arbitraires (orientées par leurs Quat).
/// Dispatch sur les variantes :
/// - Circle ↔ Circle : distance < r₁ + r₂
/// - Circle ↔ Rect / Rect ↔ Circle : utilise `shape_hits_circle`
/// - Rect ↔ Rect : approximation via le bounding circle d'un des deux Rect
///   (cas rare en pratique : un projectile long contre une AOE rectangulaire).
pub fn shapes_overlap(
    pos_a: Vec2,
    rot_a: Quat,
    shape_a: &Shape,
    pos_b: Vec2,
    rot_b: Quat,
    shape_b: &Shape,
) -> bool {
    match (shape_a, shape_b) {
        (Shape::Circle(r_a), Shape::Circle(r_b)) => pos_a.distance(pos_b) < r_a + r_b,
        (Shape::Circle(r), _) => shape_hits_circle(pos_b, rot_b, shape_b, pos_a, *r),
        (_, Shape::Circle(r)) => shape_hits_circle(pos_a, rot_a, shape_a, pos_b, *r),
        (
            Shape::Rect {
                half_length: hl_a,
                half_width: hw_a,
            },
            _,
        ) => {
            let bounding_r_a = (*hl_a).hypot(*hw_a);
            shape_hits_circle(pos_b, rot_b, shape_b, pos_a, bounding_r_a)
        }
    }
}

/// Test OBB (rectangle orienté) vs cercle.
/// Projette le centre du cercle dans le repère local du rectangle, puis trouve
/// le point le plus proche sur le rectangle.
fn obb_circle_collision(
    rect_pos: Vec2,
    rect_angle: f32,
    half_length: f32,
    half_width: f32,
    circle_pos: Vec2,
    circle_radius: f32,
) -> bool {
    let delta = circle_pos - rect_pos;
    let cos = rect_angle.cos();
    let sin = rect_angle.sin();
    let local_x = delta.dot(Vec2::new(cos, sin));
    let local_y = delta.dot(Vec2::new(-sin, cos));

    let cx = local_x.clamp(-half_width, half_width);
    let cy = local_y.clamp(-half_length, half_length);

    (local_x - cx).powi(2) + (local_y - cy).powi(2) <= circle_radius * circle_radius
}

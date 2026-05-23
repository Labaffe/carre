//! Orientation visuelle de sprites — flip horizontal selon la direction
//! du mouvement.
//!
//! Le composant [`FaceMovement`] flippe le `Sprite.flip_x` selon le signe
//! de la vitesse horizontale (delta de position frame-à-frame). Marche pour
//! n'importe quel système qui modifie `Transform.translation`.
//!
//! Exemple : le sprite du kamikaze ou du green_ufo regarde naturellement
//! vers la gauche. Quand l'ennemi se déplace vers la droite (chase joueur),
//! le sprite est flippé pour faire face à droite.

use bevy::prelude::*;

/// Flippe automatiquement le sprite (`flip_x`) selon la direction du
/// mouvement horizontal.
#[derive(Component)]
pub struct FaceMovement {
    /// Direction "naturelle" du sprite tel qu'il est dessiné dans l'asset :
    /// `false` = face à gauche, `true` = face à droite.
    /// Si l'ennemi se déplace dans la direction opposée → flip.
    pub natural_faces_right: bool,
    /// Vitesse horizontale minimale (px/s) pour appliquer le flip. Évite
    /// le clignotement quand vx oscille près de zéro.
    pub min_speed_x: f32,
    /// Position de la frame précédente (interne, mise à jour par le système).
    pub last_position: Option<Vec2>,
}

impl FaceMovement {
    /// Sprite qui regarde naturellement à GAUCHE dans son asset.
    pub fn faces_left() -> Self {
        Self {
            natural_faces_right: false,
            min_speed_x: 5.0,
            last_position: None,
        }
    }

    /// Sprite qui regarde naturellement à DROITE dans son asset.
    pub fn faces_right() -> Self {
        Self {
            natural_faces_right: true,
            min_speed_x: 5.0,
            last_position: None,
        }
    }

    /// Builder : change la vitesse minimale d'application.
    pub fn with_min_speed_x(mut self, min_speed_x: f32) -> Self {
        self.min_speed_x = min_speed_x;
        self
    }
}

/// Met à jour `Sprite.flip_x` selon le signe de la vitesse horizontale.
fn face_movement_system(
    time: Res<Time>,
    mut query: Query<(&Transform, &mut Sprite, &mut FaceMovement)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (transform, mut sprite, mut face) in &mut query {
        let pos = transform.translation.xy();
        if let Some(last) = face.last_position {
            let vx = (pos.x - last.x) / dt;
            if vx.abs() >= face.min_speed_x {
                // moving_right XOR natural_faces_right → flip si on bouge
                // dans la direction opposée à l'orientation naturelle.
                sprite.flip_x = (vx > 0.0) != face.natural_faces_right;
            }
        }
        face.last_position = Some(pos);
    }
}

// ─── Rotation continue vers la direction de mouvement ───────────────

/// Comme `FaceMovement` mais fait une **vraie rotation** du sprite (pas un
/// simple flip) pour l'aligner sur la direction du mouvement. Utile pour
/// les entités qui se déplacent en arc / courbe (ex: simple_ufo qui suit
/// une Bézier) et dont le sprite doit suivre la tangente.
///
/// `natural_facing_rad` = angle (radians) du sprite "au repos" :
/// - `+π/2` (= `FRAC_PI_2`) → sprite naturellement orienté +Y (haut)
/// - `0` → sprite orienté +X (droite)
/// - `π` → sprite orienté -X (gauche)
/// - `-π/2` → sprite orienté -Y (bas)
#[derive(Component)]
pub struct RotateToMovement {
    pub natural_facing_rad: f32,
    /// Vitesse minimale (px/s) pour appliquer la rotation. Évite la
    /// rotation parasite quand la vitesse est quasi nulle.
    pub min_speed: f32,
    /// Position frame précédente (interne).
    pub last_position: Option<Vec2>,
}

impl RotateToMovement {
    /// Sprite naturellement orienté vers le **haut** (+Y). Default.
    pub fn facing_up() -> Self {
        Self {
            natural_facing_rad: std::f32::consts::FRAC_PI_2,
            min_speed: 10.0,
            last_position: None,
        }
    }

    /// Sprite naturellement orienté vers le **bas** (-Y).
    pub fn facing_down() -> Self {
        Self {
            natural_facing_rad: -std::f32::consts::FRAC_PI_2,
            min_speed: 10.0,
            last_position: None,
        }
    }

    /// Sprite naturellement orienté vers la **droite** (+X).
    pub fn facing_right() -> Self {
        Self {
            natural_facing_rad: 0.0,
            min_speed: 10.0,
            last_position: None,
        }
    }

    /// Sprite naturellement orienté vers la **gauche** (-X).
    pub fn facing_left() -> Self {
        Self {
            natural_facing_rad: std::f32::consts::PI,
            min_speed: 10.0,
            last_position: None,
        }
    }
}

/// Met à jour `Transform.rotation` pour aligner le sprite sur la direction
/// du déplacement frame-à-frame. Conserve la dernière rotation appliquée
/// si la vitesse passe sous `min_speed`.
fn rotate_to_movement_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut RotateToMovement)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (mut tf, mut r) in &mut query {
        let pos = tf.translation.xy();
        if let Some(last) = r.last_position {
            let v = (pos - last) / dt;
            if v.length() >= r.min_speed {
                // Angle de la vitesse - angle naturel du sprite =
                // rotation à appliquer.
                let target_angle = v.y.atan2(v.x) - r.natural_facing_rad;
                tf.rotation = Quat::from_rotation_z(target_angle);
            }
        }
        r.last_position = Some(pos);
    }
}

pub struct SpriteOrientPlugin;

impl Plugin for SpriteOrientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (face_movement_system, rotate_to_movement_system));
    }
}

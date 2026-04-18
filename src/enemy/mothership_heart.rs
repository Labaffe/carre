//! MothershipHeart — sprites ennemis attachés au Mothership.
//!
//! Les Hearts sont des ennemis simples (pattern idle permanent) dont la
//! position est synchronisée au Mothership via `MothershipLink`.
//! Toute la logique (spawn, sync, mort) est dans `mothership.rs`.
//!
//! Ce module ré-exporte les types pertinents pour un accès direct.

pub use crate::enemy::mothership::{MothershipHeart, MothershipLink};

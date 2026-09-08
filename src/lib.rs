//! Deterministic rules shared by the game and its tests.
use bevy::prelude::*;

pub const TARGET_Z: f32 = -12.0;
pub const MUZZLE: Vec3 = Vec3::new(0.0, 1.5, 8.0);

#[derive(Clone, Copy, Debug)]
pub struct Difficulty {
    pub speed: f32,
    pub interval: f32,
    pub radius: f32,
    pub quota: u32,
    pub weave: f32,
}

pub fn difficulty(level: u32) -> Difficulty {
    let n = level.saturating_sub(1) as f32;
    Difficulty {
        speed: (1.5 + n * 0.24).min(5.8),
        interval: (2.1 - n * 0.11).max(0.7),
        radius: (0.82 - n * 0.025).max(0.46),
        quota: 6 + level.min(20) * 2,
        weave: (n * 0.13).min(1.0),
    }
}

/// Swept collision: fast bullets cannot tunnel through a target between frames.
pub fn segment_hits_sphere(start: Vec3, end: Vec3, center: Vec3, radius: f32) -> bool {
    let segment = end - start;
    let t = ((center - start).dot(segment) / segment.length_squared().max(1e-8)).clamp(0.0, 1.0);
    (start + segment * t).distance_squared(center) <= radius * radius
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fast_projectile_hits_between_frames() {
        assert!(segment_hits_sphere(
            Vec3::Z * 10.0,
            -Vec3::Z * 10.0,
            Vec3::ZERO,
            0.5
        ));
        assert!(!segment_hits_sphere(
            Vec3::Z * 10.0,
            -Vec3::Z * 10.0,
            Vec3::X,
            0.5
        ));
    }
    #[test]
    fn collision_respects_segment_endpoints() {
        assert!(!segment_hits_sphere(
            Vec3::ZERO,
            Vec3::Z,
            Vec3::Z * 3.0,
            0.5
        ));
        assert!(segment_hits_sphere(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, 0.5));
    }
    #[test]
    fn levels_increase_pressure_with_playable_limits() {
        for level in 1..100 {
            let a = difficulty(level);
            let b = difficulty(level + 1);
            assert!(b.speed >= a.speed && b.interval <= a.interval && b.radius <= a.radius);
            assert!(b.quota >= a.quota && b.weave >= a.weave);
            assert!(b.interval >= 0.7 && b.radius >= 0.46 && b.speed <= 5.8);
        }
    }
}

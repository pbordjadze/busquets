//! A small 2D vector, metres.

use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

pub const fn v2(x: f32, y: f32) -> V2 {
    V2 { x, y }
}

impl V2 {
    pub const ZERO: V2 = v2(0.0, 0.0);
    pub fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    pub fn len2(self) -> f32 {
        self.x * self.x + self.y * self.y
    }
    pub fn norm(self) -> V2 {
        let l = self.len();
        if l < 1e-6 {
            V2::ZERO
        } else {
            self * (1.0 / l)
        }
    }
    pub fn dot(self, o: V2) -> f32 {
        self.x * o.x + self.y * o.y
    }
    pub fn cross(self, o: V2) -> f32 {
        self.x * o.y - self.y * o.x
    }
    pub fn perp(self) -> V2 {
        v2(-self.y, self.x)
    }
    pub fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }
    pub fn from_angle(a: f32) -> V2 {
        v2(a.cos(), a.sin())
    }
    pub fn dist(self, o: V2) -> f32 {
        (self - o).len()
    }
    pub fn lerp(self, o: V2, k: f32) -> V2 {
        self + (o - self) * k
    }
    /// Shortened to at most `m`.
    pub fn clamp_len(self, m: f32) -> V2 {
        let l = self.len();
        if l > m && l > 0.0 {
            self * (m / l)
        } else {
            self
        }
    }
    pub fn arr(self) -> [f32; 2] {
        [self.x, self.y]
    }
}

impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        v2(self.x + o.x, self.y + o.y)
    }
}
impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        v2(self.x - o.x, self.y - o.y)
    }
}
impl Mul<f32> for V2 {
    type Output = V2;
    fn mul(self, k: f32) -> V2 {
        v2(self.x * k, self.y * k)
    }
}
impl Neg for V2 {
    type Output = V2;
    fn neg(self) -> V2 {
        v2(-self.x, -self.y)
    }
}
impl AddAssign for V2 {
    fn add_assign(&mut self, o: V2) {
        *self = *self + o;
    }
}
impl SubAssign for V2 {
    fn sub_assign(&mut self, o: V2) {
        *self = *self - o;
    }
}

/// The closest point to `p` on the segment `a`..`b`, and how far along it (0..1).
pub fn closest_on_segment(p: V2, a: V2, b: V2) -> (V2, f32) {
    let ab = b - a;
    let l2 = ab.len2();
    if l2 < 1e-9 {
        return (a, 0.0);
    }
    let u = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    (a + ab * u, u)
}

/// The signed difference of two angles, in -π..π.
pub fn angle_diff(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(std::f32::consts::TAU);
    if d > std::f32::consts::PI {
        d - std::f32::consts::TAU
    } else {
        d
    }
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A tiny deterministic generator (xorshift*), so a played plan is the same game every time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// 0..1
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// -1..1
    pub fn s(&mut self) -> f32 {
        self.f() * 2.0 - 1.0
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
}

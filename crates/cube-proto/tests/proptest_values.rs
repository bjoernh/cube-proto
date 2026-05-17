//! Property-based round-trip tests for the shared value types (SDS §6.1, §5.4).
//!
//! Strategy:
//! - `Damage`: any u32 quad must round-trip.
//! - `Color`: hex pairs in `#RRGGBB` (alpha=255 implied) and `#RRGGBBAA` must
//!   round-trip and serialize back into the canonical form (uppercase hex,
//!   alpha-suffixed only when α≠0xFF).
//! - `Vec2` / `Vec3`: any pair / triple of finite f32 (no NaN, no ±∞) must
//!   round-trip. We use f32 because the SDS doesn't pin width; the test would
//!   tighten if Vec2/Vec3 are typed differently.
//! - `ParamValue::Int` over i32 range must round-trip (we test i32 — the
//!   storage is i64, so a tighter input range proves coverage of all useful
//!   integers without bumping into encoding edge-cases).

use proptest::prelude::*;
use serde_json::json;

use cube_proto::{Color, Damage, ParamValue, Vec2, Vec3};

proptest! {
    #[test]
    fn damage_roundtrip_sds_6_1(x in any::<u32>(), y in any::<u32>(), w in any::<u32>(), h in any::<u32>()) {
        let d = Damage { x, y, w, h };
        let v = serde_json::to_value(&d).unwrap();
        let d2: Damage = serde_json::from_value(v).unwrap();
        prop_assert_eq!(d, d2);
    }

    #[test]
    fn color_rgb_roundtrip_sds_5_4(r in any::<u8>(), g in any::<u8>(), b in any::<u8>()) {
        let c = Color::from_rgba(r, g, b, 0xFF);
        let v = serde_json::to_value(&c).unwrap();
        let expected = json!(format!("#{:02X}{:02X}{:02X}", r, g, b));
        prop_assert_eq!(v.clone(), expected);
        let c2: Color = serde_json::from_value(v).unwrap();
        prop_assert_eq!(c, c2);
    }

    #[test]
    fn color_rgba_roundtrip_sds_5_4(r in any::<u8>(), g in any::<u8>(), b in any::<u8>(), a in 0u8..=0xFEu8) {
        let c = Color::from_rgba(r, g, b, a);
        let v = serde_json::to_value(&c).unwrap();
        let expected = json!(format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a));
        prop_assert_eq!(v.clone(), expected);
        let c2: Color = serde_json::from_value(v).unwrap();
        prop_assert_eq!(c, c2);
    }

    #[test]
    fn vec2_roundtrip_sds_5_4(x in prop::num::f32::NORMAL | prop::num::f32::POSITIVE | prop::num::f32::NEGATIVE | prop::num::f32::ZERO,
                              y in prop::num::f32::NORMAL | prop::num::f32::POSITIVE | prop::num::f32::NEGATIVE | prop::num::f32::ZERO) {
        let v2 = Vec2 { x, y };
        let v = serde_json::to_value(&v2).unwrap();
        let back: Vec2 = serde_json::from_value(v).unwrap();
        prop_assert_eq!(v2, back);
    }

    #[test]
    fn vec3_roundtrip_sds_5_4(x in prop::num::f32::NORMAL | prop::num::f32::POSITIVE | prop::num::f32::NEGATIVE | prop::num::f32::ZERO,
                              y in prop::num::f32::NORMAL | prop::num::f32::POSITIVE | prop::num::f32::NEGATIVE | prop::num::f32::ZERO,
                              z in prop::num::f32::NORMAL | prop::num::f32::POSITIVE | prop::num::f32::NEGATIVE | prop::num::f32::ZERO) {
        let v3 = Vec3 { x, y, z };
        let v = serde_json::to_value(&v3).unwrap();
        let back: Vec3 = serde_json::from_value(v).unwrap();
        prop_assert_eq!(v3, back);
    }

    #[test]
    fn paramvalue_int_roundtrip_sds_5_4(i in i32::MIN..=i32::MAX) {
        let p = ParamValue::Int(i64::from(i));
        let v = serde_json::to_value(&p).unwrap();
        prop_assert_eq!(v.clone(), json!({"type": "int", "value": i}));
        let p2: ParamValue = serde_json::from_value(v).unwrap();
        prop_assert_eq!(p, p2);
    }

    #[test]
    fn paramvalue_bool_roundtrip_sds_5_4(b in any::<bool>()) {
        let p = ParamValue::Bool(b);
        let v = serde_json::to_value(&p).unwrap();
        let p2: ParamValue = serde_json::from_value(v).unwrap();
        prop_assert_eq!(p, p2);
    }

    #[test]
    fn paramvalue_string_roundtrip_sds_5_4(s in "[a-zA-Z0-9_.-]{0,64}") {
        let p = ParamValue::String(s);
        let v = serde_json::to_value(&p).unwrap();
        let p2: ParamValue = serde_json::from_value(v).unwrap();
        prop_assert_eq!(p, p2);
    }
}

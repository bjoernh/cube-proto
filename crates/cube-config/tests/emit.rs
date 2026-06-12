//! RED contract for cube-config's emission surface (SDS §7.2; cubekit-spec §9.4).
//!
//! The SDK's "agree by construction" promise is that the same crate that *loads*
//! `manifest.toml` / `schema.toml` also *emits* them, so an emitter and loader
//! cannot drift. These tests pin that: `manifest_to_toml` / `schema_to_toml`
//! produce TOML that `load_manifest` / `load_schema` accept and round-trip.

use cube_config::{
    AppSection, Manifest, ManifestCategory, ParamType, PowerSection, RequiresSection, Schema,
    load_manifest, load_schema, manifest_to_toml, schema_to_toml,
};
use cube_proto::ParamValue;
use semver::{Version, VersionReq};

fn write_tmp(name: &str, contents: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    std::fs::write(&path, contents).unwrap();
    (dir, path)
}

// ── manifest ─────────────────────────────────────────────────────────────────

fn sample_manifest() -> Manifest {
    Manifest {
        app: AppSection {
            name: "voxel-sand".to_owned(),
            display_name: "Voxel Sand".to_owned(),
            version: Version::parse("2.0.0").unwrap(),
            category: ManifestCategory::Visualizer,
            icon: None,
        },
        requires: Some(RequiresSection {
            libcube: None,
            cubekit: Some(VersionReq::parse("0.1").unwrap()),
            inputs: vec!["joystick".to_owned()],
            sensors: vec!["imu".to_owned()],
            network: true,
        }),
        power: Some(PowerSection { idle_blank: true }),
    }
}

#[test]
fn manifest_to_toml_round_trips_through_the_loader() {
    let m = sample_manifest();
    let toml = manifest_to_toml(&m);
    let (_dir, path) = write_tmp("manifest.toml", &toml);

    let back = load_manifest(&path).expect("emitted manifest must parse");
    assert_eq!(back.app.name, "voxel-sand");
    assert_eq!(back.app.display_name, "Voxel Sand");
    assert_eq!(back.app.version.to_string(), "2.0.0");
    assert_eq!(back.app.category, ManifestCategory::Visualizer);
    let req = back.requires.expect("requires section");
    assert!(req.network);
    assert!(req.libcube.is_none(), "libcube omitted when None");
    assert!(req.cubekit.is_some(), "cubekit requirement preserved");
    assert!(req.inputs.iter().any(|i| i == "joystick"));
    assert!(req.sensors.iter().any(|s| s == "imu"));
    assert!(back.power.expect("power").idle_blank);
}

#[test]
fn manifest_to_toml_is_stable_and_sorted() {
    // Deterministic output: a second emission is byte-identical, and tables come
    // out key-sorted (so `[app]` keys are alphabetical).
    let m = sample_manifest();
    let a = manifest_to_toml(&m);
    let b = manifest_to_toml(&m);
    assert_eq!(a, b, "emission is deterministic");
    let app_idx = a.find("[app]").expect("has [app]");
    let cat = a[app_idx..].find("category").unwrap();
    let name = a[app_idx..].find("name = ").unwrap();
    assert!(cat < name, "[app] keys are sorted (category before name)");
    // A None Option (icon) must be omitted entirely.
    assert!(!a.contains("icon"), "absent icon is not emitted:\n{a}");
}

// ── schema ───────────────────────────────────────────────────────────────────

fn sample_schema() -> Schema {
    Schema::builder("voxel-sand", 3)
        .int_range("grains", 100, 20_000)
        .float_readonly("load")
        .string_non_shareable("label")
        .build()
}

#[test]
fn schema_to_toml_round_trips_through_the_loader() {
    let s = sample_schema();
    let toml = schema_to_toml(&s);
    let (_dir, path) = write_tmp("schema.toml", &toml);

    let back = load_schema(&path).expect("emitted schema must parse");
    assert_eq!(back.schema_version, 3);
    assert!(back.params.contains_key("grains"));
    assert_eq!(back.params["grains"].ty, ParamType::Int);
    assert!(back.params.contains_key("load"));
    assert!(back.params["load"].readonly, "readonly flag survives the round trip");
    assert!(back.params.contains_key("label"));
}

#[test]
fn schema_to_toml_emits_param_value_scalars() {
    // The emitted schema carries the ParamValue defaults as TOML scalars that the
    // loader reads back to the same value (exercises the ParamValue mapping).
    let s = Schema::builder("vectors", 1).int_range("n", 0, 10).build();
    let toml = schema_to_toml(&s);
    assert!(toml.contains("schema_version = 1"));
    assert!(toml.contains("[params.n]"));
    // The default scalar round-trips as an integer.
    let (_dir, path) = write_tmp("schema.toml", &toml);
    let back = load_schema(&path).unwrap();
    assert_eq!(back.params["n"].default, ParamValue::Int(0));
}

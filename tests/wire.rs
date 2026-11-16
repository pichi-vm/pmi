// SPDX-FileCopyrightText: Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Wire-schema tests: encode the types to real CBOR and assert the per-target
//! schema from `spec/dt.md` — the `dt:dtb`/`dt:dtbo` names, `Version<N>`
//! acceptance, `deny_unknown_fields`, and the optional `dt:dtb` attribute.

use ciborium::value::{Integer, Value};
use flagset::{FlagSet, Flags};

use pmi::sev::Feature;
use pmi::vm::{Action, Fill, FillKind, Load, LoadKind, Spec};
use pmi::x86_64::Profile;
use pmi::Version;

fn to_cbor<T: serde::Serialize>(v: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::into_writer(v, &mut bytes).expect("serialize to CBOR");
    bytes
}

fn value_of<T: serde::Serialize>(v: &T) -> Value {
    ciborium::from_reader(to_cbor(v).as_slice()).expect("decode to Value")
}

fn entries(v: &Value) -> &[(Value, Value)] {
    match v {
        Value::Map(m) => m,
        other => panic!("expected a CBOR map, got {other:?}"),
    }
}

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    let wanted = Value::Text(key.to_owned());
    entries(v)
        .iter()
        .find(|(k, _)| *k == wanted)
        .map(|(_, val)| val)
}

/// The bundled `.pmi.vm` example from `spec/dt.md`: base bundled + loaded, host
/// overlay via `dt:dtbo`.
fn bundled_example() -> Spec<Profile> {
    Spec {
        version: Version::default(),
        cpu_profile: Profile::V2,
        dt_dtb: Some(".dtb".to_owned()),
        actions: vec![
            Action::Load(Load {
                gpa: 0x100000,
                section: ".linux".to_owned(),
                kind: LoadKind::Default,
            }),
            Action::Load(Load {
                gpa: 0x2001000,
                section: ".dtb".to_owned(),
                kind: LoadKind::Default,
            }),
            Action::Fill(Fill {
                gpa: 0x2011000,
                section: ".dtbo".to_owned(),
                kind: FillKind::DtDtbo,
            }),
        ],
    }
}

#[test]
fn bundled_example_round_trips() {
    let bytes = to_cbor(&bundled_example());
    let back: Spec<Profile> = ciborium::from_reader(bytes.as_slice()).expect("decode Spec");

    assert_eq!(back.version, Version::<1>::default());
    assert_eq!(back.dt_dtb.as_deref(), Some(".dtb"));
    assert_eq!(back.actions.len(), 3);
    match &back.actions[2] {
        Action::Fill(f) => assert_eq!(f.kind, FillKind::DtDtbo),
        other => panic!("expected a fill, got {other:?}"),
    }
}

#[test]
fn wire_keys_use_the_dt_prefix() {
    let v = value_of(&bundled_example());

    // Target attribute rename.
    assert_eq!(get(&v, "dt:dtb"), Some(&Value::Text(".dtb".to_owned())));
    assert!(get(&v, "cpu:profile").is_some());

    // The fill action carries the renamed kind; the default loads omit `kind`.
    let actions = match get(&v, "actions") {
        Some(Value::Array(a)) => a,
        other => panic!("expected actions array, got {other:?}"),
    };
    let fill = actions.last().unwrap();
    assert_eq!(get(fill, "kind"), Some(&Value::Text("dt:dtbo".to_owned())));
    assert!(
        get(&actions[0], "kind").is_none(),
        "LoadKind::Default is skipped on the wire"
    );
}

#[test]
fn fill_kind_wire_values() {
    assert_eq!(value_of(&FillKind::DtDtb), Value::Text("dt:dtb".to_owned()));
    assert_eq!(
        value_of(&FillKind::DtDtbo),
        Value::Text("dt:dtbo".to_owned())
    );

    let back: FillKind =
        ciborium::from_reader(to_cbor(&FillKind::DtDtb).as_slice()).expect("decode FillKind");
    assert_eq!(back, FillKind::DtDtb);
}

#[test]
fn detached_mode_omits_the_attribute() {
    let mut spec = bundled_example();
    spec.dt_dtb = None;
    spec.actions[1] = Action::Fill(Fill {
        gpa: 0x2001000,
        section: ".dtb".to_owned(),
        kind: FillKind::DtDtb,
    });

    let v = value_of(&spec);
    assert!(
        get(&v, "dt:dtb").is_none(),
        "absent attribute is skipped, marking detached mode"
    );

    let back: Spec<Profile> = ciborium::from_reader(to_cbor(&spec).as_slice()).expect("decode");
    assert_eq!(back.dt_dtb, None);
    match &back.actions[1] {
        Action::Fill(f) => assert_eq!(f.kind, FillKind::DtDtb),
        other => panic!("expected a dt:dtb fill, got {other:?}"),
    }
}

#[test]
fn wrong_version_is_rejected() {
    let mut v = value_of(&bundled_example());
    for (k, val) in match &mut v {
        Value::Map(m) => m,
        _ => unreachable!(),
    } {
        if *k == Value::Text("version".to_owned()) {
            *val = Value::Integer(Integer::from(2u64));
        }
    }

    let bytes = to_cbor(&v);
    assert!(
        ciborium::from_reader::<Spec<Profile>, _>(bytes.as_slice()).is_err(),
        "Version<1> must reject version 2"
    );
}

#[test]
fn unknown_key_is_rejected() {
    let mut v = value_of(&bundled_example());
    match &mut v {
        Value::Map(m) => m.push((Value::Text("dt:bogus".to_owned()), Value::Null)),
        _ => unreachable!(),
    }

    let bytes = to_cbor(&v);
    assert!(
        ciborium::from_reader::<Spec<Profile>, _>(bytes.as_slice()).is_err(),
        "deny_unknown_fields must reject an unknown target key"
    );
}

#[test]
fn unknown_key_in_a_load_is_rejected() {
    // deny_unknown_fields must reach inside the internally-tagged action enum.
    let mut v = value_of(&bundled_example());
    if let Value::Map(top) = &mut v {
        for (k, val) in top.iter_mut() {
            if *k == Value::Text("actions".to_owned()) {
                if let Value::Array(actions) = val {
                    if let Value::Map(load) = &mut actions[0] {
                        load.push((Value::Text("bogus".to_owned()), Value::Null));
                    }
                }
            }
        }
    }

    let bytes = to_cbor(&v);
    assert!(
        ciborium::from_reader::<Spec<Profile>, _>(bytes.as_slice()).is_err(),
        "an unknown key inside a load action must be rejected"
    );
}

#[test]
fn lbc_bootconfig_kind_on_every_target() {
    fn roundtrip<T: serde::Serialize + serde::de::DeserializeOwned>() {
        let wire = to_cbor(&Value::Map(vec![
            (Value::Text("type".into()), Value::Text("fill".into())),
            (Value::Text("gpa".into()), Value::Integer(0x2100000.into())),
            (Value::Text("section".into()), Value::Text(".bconf".into())),
            (
                Value::Text("kind".into()),
                Value::Text("lbc:bootconfig".into()),
            ),
        ]));
        let action: T = ciborium::from_reader(wire.as_slice()).unwrap();
        assert_eq!(
            get(&value_of(&action), "kind"),
            Some(&Value::Text("lbc:bootconfig".into()))
        );
    }
    roundtrip::<pmi::vm::Action>();
    roundtrip::<pmi::sev::Action>();
    roundtrip::<pmi::tdx::Action>();
}

#[test]
fn unknown_fill_kinds_are_rejected_on_every_target() {
    for kind in ["unknown:kind", "lbc:unknown", "lbc:Bootconfig"] {
        let wire = to_cbor(&Value::Text(kind.into()));
        assert!(ciborium::from_reader::<pmi::vm::FillKind, _>(wire.as_slice()).is_err());
        assert!(ciborium::from_reader::<pmi::sev::FillKind, _>(wire.as_slice()).is_err());
        assert!(ciborium::from_reader::<pmi::tdx::FillKind, _>(wire.as_slice()).is_err());
    }
}

#[test]
fn cpu_profile_values_use_their_spec_names() {
    fn check<P>(profile: P, name: &str)
    where
        P: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        assert_eq!(value_of(&profile), Value::Text(name.to_owned()));
        let back: P = ciborium::from_reader(to_cbor(&profile).as_slice()).expect("decode profile");
        assert_eq!(back, profile);
    }
    check(pmi::x86_64::Profile::V1, "x86-64-v1");
    check(pmi::x86_64::Profile::V4, "x86-64-v4");
    check(pmi::aarch64::Profile::V8_0, "armv8.0-a");
    check(pmi::aarch64::Profile::V9_8, "armv9.8-a");
}

#[test]
fn cpu_profile_values_are_per_architecture() {
    let decode = |name: &str| to_cbor(&Value::Text(name.into()));
    for name in ["x86-64", "x86-64-v5", "armv8.0-a"] {
        assert!(ciborium::from_reader::<pmi::x86_64::Profile, _>(decode(name).as_slice()).is_err());
    }
    for name in ["armv8-a", "armv9.9-a", "x86-64-v1"] {
        assert!(
            ciborium::from_reader::<pmi::aarch64::Profile, _>(decode(name).as_slice()).is_err()
        );
    }
}

#[test]
fn sev_features_is_an_optional_bit_field() {
    let target = |features: Option<u64>| {
        let mut map = vec![
            (Value::Text("version".into()), Value::Integer(1.into())),
            (
                Value::Text("cpu:profile".into()),
                Value::Text("x86-64-v3".into()),
            ),
            (
                Value::Text("actions".into()),
                Value::Array(vec![Value::Map(vec![
                    (Value::Text("type".into()), Value::Text("load".into())),
                    (
                        Value::Text("gpa".into()),
                        Value::Integer(0xFFC0_0000u32.into()),
                    ),
                    (Value::Text("section".into()), Value::Text(".ovmf".into())),
                ])]),
            ),
        ];
        if let Some(bits) = features {
            map.push((
                Value::Text("sev:features".into()),
                Value::Integer(bits.into()),
            ));
        }
        to_cbor(&Value::Map(map))
    };
    let decode = |bytes: Vec<u8>| ciborium::from_reader::<pmi::sev::Spec, _>(bytes.as_slice());

    let absent = decode(target(None)).expect("absent");
    assert!(absent.features.is_empty());
    assert!(get(&value_of(&absent), "sev:features").is_none());

    let spec = decode(target(Some(1 << 5 | 1 << 9))).expect("offered bits");
    assert_eq!(
        spec.features,
        Feature::DebugVirtualization | Feature::SecureTsc
    );
    assert_eq!(
        get(&value_of(&spec), "sev:features"),
        Some(&Value::Integer((1u64 << 5 | 1 << 9).into()))
    );

    // SNPActive (bit 0), vTOM (bit 1), and SecureAvic (bit 16).
    for bit in [0, 1, 16] {
        assert!(decode(target(Some(1 << bit))).is_err());
    }
}

#[test]
fn sev_feature_values_are_their_spec_bits() {
    let table = [(Feature::DebugVirtualization, 5), (Feature::SecureTsc, 9)];
    assert_eq!(
        Feature::LIST,
        table.map(|(feature, _)| feature),
        "Feature lists every spec bit"
    );
    for (feature, bit) in table {
        assert_eq!(FlagSet::from(feature).bits(), 1 << bit);
    }
}

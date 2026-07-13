// SPDX-FileCopyrightText: Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Wire-schema tests: encode the types to real CBOR and assert the per-target
//! schema from `spec/dt.md` — the `dt:dtb`/`dt:dtbo` names, `Version<N>`
//! acceptance, `deny_unknown_fields`, the optional `dt:dtb` attribute, and the
//! `rflags` default/skip behavior.

use ciborium::value::{Integer, Value};

use pmi::vm::vcpu::x86_64::CpuState;
use pmi::vm::{Action, Fill, FillKind, Load, LoadKind, Spec};
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
fn bundled_example() -> Spec<CpuState> {
    Spec {
        version: Version::default(),
        vcpu: CpuState {
            rip: 0x100000,
            rsp: 0x80000,
            ..CpuState::default()
        },
        cpu_profile: "x86-64-v2".into(),
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
    let back: Spec<CpuState> = ciborium::from_reader(bytes.as_slice()).expect("decode Spec");

    assert_eq!(back.version, Version::<1>::default());
    assert_eq!(back.dt_dtb.as_deref(), Some(".dtb"));
    assert_eq!(back.vcpu.rip, 0x100000);
    assert_eq!(
        back.vcpu.rflags.get(),
        0x2,
        "rflags default survives the round trip"
    );
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
    assert!(get(&v, "vm:vcpu").is_some());
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

    let back: Spec<CpuState> = ciborium::from_reader(to_cbor(&spec).as_slice()).expect("decode");
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
        ciborium::from_reader::<Spec<CpuState>, _>(bytes.as_slice()).is_err(),
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
        ciborium::from_reader::<Spec<CpuState>, _>(bytes.as_slice()).is_err(),
        "deny_unknown_fields must reject an unknown target key"
    );
}

#[test]
fn default_vcpu_serializes_empty_and_defaults_rflags() {
    // A default vCPU skips every register (rflags == 0x2 is its own default).
    let v = value_of(&CpuState::default());
    assert!(entries(&v).is_empty(), "default vCPU is the empty map");

    // Decoding the empty map restores the reserved rflags bit.
    let empty = Value::Map(Vec::new());
    let back: CpuState = ciborium::from_reader(to_cbor(&empty).as_slice()).expect("decode vcpu");
    assert_eq!(back.rflags.get(), 0x2);
    assert_eq!(back.rip, 0);
}

#[test]
fn rflags_reserved_bit_is_enforced() {
    // A present rflags that clears bit 1 is rejected on decode.
    let bad = Value::Map(vec![(
        Value::Text("rflags".to_owned()),
        Value::Integer(Integer::from(0u64)),
    )]);
    assert!(
        ciborium::from_reader::<CpuState, _>(to_cbor(&bad).as_slice()).is_err(),
        "rflags with bit 1 clear must be rejected"
    );

    // A value with bit 1 set decodes fine.
    let ok = Value::Map(vec![(
        Value::Text("rflags".to_owned()),
        Value::Integer(Integer::from(0x202u64)),
    )]);
    let s: CpuState = ciborium::from_reader(to_cbor(&ok).as_slice()).expect("valid rflags");
    assert_eq!(s.rflags.get(), 0x202);
}

#[test]
fn reserved_segment_attribute_bits_are_rejected() {
    let bad = Value::Map(vec![(
        Value::Text("attributes".to_owned()),
        Value::Integer(Integer::from(0x1000u64)),
    )]);
    assert!(
        ciborium::from_reader::<pmi::vm::vcpu::x86_64::SegReg, _>(to_cbor(&bad).as_slice())
            .is_err(),
        "segment attribute bits 12-15 must be zero"
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
        ciborium::from_reader::<Spec<CpuState>, _>(bytes.as_slice()).is_err(),
        "an unknown key inside a load action must be rejected"
    );
}

#[test]
fn aarch64_pstate_is_required_and_must_select_el1() {
    use pmi::vm::vcpu::aarch64;

    // Omitting pstate is rejected (it is required).
    let no_pstate = Value::Map(vec![(
        Value::Text("pc".to_owned()),
        Value::Integer(Integer::from(0x100000u64)),
    )]);
    assert!(
        ciborium::from_reader::<aarch64::CpuState, _>(to_cbor(&no_pstate).as_slice()).is_err(),
        "missing pstate must be rejected"
    );

    // A non-EL1 pstate is rejected.
    let el0 = Value::Map(vec![(
        Value::Text("pstate".to_owned()),
        Value::Integer(Integer::from(0u64)),
    )]);
    assert!(ciborium::from_reader::<aarch64::CpuState, _>(to_cbor(&el0).as_slice()).is_err());

    // EL1h decodes.
    let el1 = Value::Map(vec![(
        Value::Text("pstate".to_owned()),
        Value::Integer(Integer::from(0x5u64)),
    )]);
    let s: aarch64::CpuState =
        ciborium::from_reader(to_cbor(&el1).as_slice()).expect("EL1h pstate is valid");
    assert_eq!(s.pstate.get(), 0x5);
}

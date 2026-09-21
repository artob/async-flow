// This is free and unencumbered software released into the public domain.

use async_flow::model::{InputPortId, Inputs, OutputPortId, Outputs, PortId};
use std::collections::BTreeSet;

#[test]
fn signed_constructors_enforce_direction_and_nonzero_ids() {
    for value in [isize::MIN, -97, -1] {
        let input = InputPortId::try_from(value).unwrap();
        assert_eq!(isize::from(input), value);
        assert_eq!(*input.as_ref(), value);
        assert_eq!(input.to_string(), value.to_string());
        assert_eq!(PortId::try_from(value).unwrap(), PortId::Input(input));
        assert!(OutputPortId::try_from(value).is_err());
    }
    for value in [1, 97, isize::MAX] {
        let output = OutputPortId::try_from(value).unwrap();
        assert_eq!(isize::from(output), value);
        assert_eq!(*output.as_ref(), value);
        assert_eq!(output.to_string(), value.to_string());
        assert_eq!(PortId::try_from(value).unwrap(), PortId::Output(output));
        assert!(InputPortId::try_from(value).is_err());
    }
    assert!(InputPortId::try_from(0).is_err());
    assert!(OutputPortId::try_from(0).is_err());
    assert!(PortId::try_from(0).is_err());
}

#[test]
fn unsigned_keys_preserve_direction_and_round_trip_at_boundaries() {
    let mut keys = BTreeSet::new();
    for signed in [isize::MIN, -97, -2, -1, 1, 2, 97, isize::MAX] {
        let id = PortId::try_from(signed).unwrap();
        let encoded = id.as_usize();
        assert_eq!(encoded, signed as usize);
        assert_eq!(usize::from(id), encoded);
        assert_eq!(PortId::from_usize(encoded).unwrap(), id);
        assert_eq!(id.as_isize(), signed);
        assert_eq!(isize::from(id), signed);
        assert_eq!(*id.as_ref(), signed);
        assert_eq!(id.to_string(), signed.to_string());
        assert!(keys.insert(encoded));
    }
    assert!(PortId::from_usize(0).is_err());
    assert_eq!(PortId::from_usize(usize::MAX).unwrap().as_isize(), -1);
    assert_eq!(
        PortId::from_usize(isize::MAX as usize + 1)
            .unwrap()
            .as_isize(),
        isize::MIN
    );
}

#[test]
fn magnitudes_and_ordinals_are_direction_local() {
    for magnitude in [1, 2, 97, isize::MAX] {
        let input = InputPortId::try_from(-magnitude).unwrap();
        let output = OutputPortId::try_from(magnitude).unwrap();
        assert_eq!(input.magnitude(), magnitude as usize);
        assert_eq!(output.magnitude(), magnitude as usize);
        assert_eq!(usize::from(input), input.magnitude());
        assert_eq!(usize::from(output), output.magnitude());
        assert_eq!(PortId::from(input).magnitude(), input.magnitude());
        assert_eq!(PortId::from(output).magnitude(), output.magnitude());
        assert_eq!(input.index(), magnitude as usize - 1);
        assert_eq!(output.index(), magnitude as usize - 1);
        assert_ne!(
            PortId::from(input).as_usize(),
            PortId::from(output).as_usize()
        );
    }
    let minimum = InputPortId::try_from(isize::MIN).unwrap();
    assert_eq!(minimum.magnitude(), isize::MAX as usize + 1);
    assert_eq!(minimum.index(), isize::MAX as usize);
}

#[test]
fn generated_ids_remain_unique_across_types_cardinalities_and_threads() {
    let workers: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                (0..32)
                    .map(|n| match n % 3 {
                        0 => (Inputs::<u8>::default().id(), Outputs::<u8>::default().id()),
                        1 => (
                            Inputs::<String, 1>::default().id(),
                            Outputs::<String, 1>::default().id(),
                        ),
                        _ => (
                            Inputs::<u64, 3, 2>::default().id(),
                            Outputs::<u64, 3, 2>::default().id(),
                        ),
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let mut inputs = BTreeSet::new();
    let mut outputs = BTreeSet::new();
    for worker in workers {
        for (input, output) in worker.join().unwrap() {
            assert!(isize::from(input) < 0);
            assert!(isize::from(output) > 0);
            assert!(inputs.insert(input));
            assert!(outputs.insert(output));
        }
    }
    assert_eq!(inputs.len(), 128);
    assert_eq!(outputs.len(), 128);
}

#[cfg(feature = "serde")]
mod serialization {
    use super::*;
    use serde::{
        Deserialize,
        de::{self, IntoDeserializer, Visitor},
    };
    use std::collections::BTreeMap;

    #[test]
    fn valid_ids_preserve_json_representation_and_round_trip() {
        for signed in [isize::MIN, -97, -1, 1, 97, isize::MAX] {
            let port = PortId::try_from(signed).unwrap();
            let (tag, leaf) = match port {
                PortId::Input(input) => {
                    let json = serde_json::to_string(&input).unwrap();
                    assert_eq!(serde_json::from_str::<InputPortId>(&json).unwrap(), input);
                    ("input", json)
                },
                PortId::Output(output) => {
                    let json = serde_json::to_string(&output).unwrap();
                    assert_eq!(serde_json::from_str::<OutputPortId>(&json).unwrap(), output);
                    ("output", json)
                },
            };
            assert_eq!(leaf, signed.to_string());
            let json = format!("{{\"{tag}\":{signed}}}");
            assert_eq!(serde_json::to_string(&port).unwrap(), json);
            assert_eq!(serde_json::from_str::<PortId>(&json).unwrap(), port);
            let value = serde_json::to_value(port).unwrap();
            assert_eq!(serde_json::from_value::<PortId>(value).unwrap(), port);
        }
    }

    #[test]
    fn leaf_ids_reject_zero_and_wrong_signs() {
        for signed in [0, 1, isize::MAX] {
            let error = serde_json::from_str::<InputPortId>(&signed.to_string()).unwrap_err();
            assert!(error.to_string().contains("negative"));
        }
        for signed in [isize::MIN, -1, 0] {
            let error = serde_json::from_str::<OutputPortId>(&signed.to_string()).unwrap_err();
            assert!(error.to_string().contains("positive"));
        }
        assert!(serde_json::from_str::<InputPortId>("-0").is_err());
        assert!(serde_json::from_str::<OutputPortId>("-0").is_err());
    }

    #[test]
    fn enum_tags_cannot_contradict_the_numeric_direction() {
        for json in [
            r#"{"input":0}"#,
            r#"{"input":1}"#,
            r#"{"output":0}"#,
            r#"{"output":-1}"#,
            r#"{"Input":-1}"#,
            r#"{"unknown":1}"#,
            r#"{"input":-1,"output":1}"#,
        ] {
            assert!(serde_json::from_str::<PortId>(json).is_err(), "{json}");
        }
    }

    #[test]
    fn out_of_range_numbers_are_rejected_instead_of_wrapping() {
        for json in [
            (isize::MIN as i128 - 1).to_string(),
            (isize::MAX as i128 + 1).to_string(),
            usize::MAX.to_string(),
        ] {
            assert!(
                serde_json::from_str::<InputPortId>(&json).is_err(),
                "{json}"
            );
            assert!(
                serde_json::from_str::<OutputPortId>(&json).is_err(),
                "{json}"
            );
            for tag in ["input", "output"] {
                let tagged = format!("{{\"{tag}\":{json}}}");
                assert!(serde_json::from_str::<PortId>(&tagged).is_err(), "{tagged}");
            }
        }
    }

    #[test]
    fn noninteger_payloads_are_rejected() {
        for json in [
            "null",
            "true",
            "false",
            "1.0",
            "-1.0",
            "1e0",
            r#""-1""#,
            r#""1""#,
            "[-1]",
            "{\"value\":1}",
        ] {
            assert!(serde_json::from_str::<InputPortId>(json).is_err(), "{json}");
            assert!(
                serde_json::from_str::<OutputPortId>(json).is_err(),
                "{json}"
            );
            for tag in ["input", "output"] {
                let tagged = format!("{{\"{tag}\":{json}}}");
                assert!(serde_json::from_str::<PortId>(&tagged).is_err(), "{tagged}");
            }
        }
    }

    #[test]
    fn validation_applies_to_nested_values_and_map_keys() {
        assert!(serde_json::from_str::<Vec<InputPortId>>("[-1,0,-2]").is_err());
        assert!(serde_json::from_str::<Vec<OutputPortId>>("[1,-1,2]").is_err());
        assert!(serde_json::from_str::<Vec<PortId>>(r#"[{"input":-1},{"output":-2}]"#).is_err());
        let input = InputPortId::try_from(isize::MIN).unwrap();
        let output = OutputPortId::try_from(isize::MAX).unwrap();
        let inputs = BTreeMap::from([(input, 1u8)]);
        let outputs = BTreeMap::from([(output, 2u8)]);
        assert_eq!(
            serde_json::from_str::<BTreeMap<InputPortId, u8>>(
                &serde_json::to_string(&inputs).unwrap()
            )
            .unwrap(),
            inputs
        );
        assert_eq!(
            serde_json::from_str::<BTreeMap<OutputPortId, u8>>(
                &serde_json::to_string(&outputs).unwrap()
            )
            .unwrap(),
            outputs
        );
        for json in [r#"{"0":1}"#, r#"{"1":1}"#] {
            assert!(serde_json::from_str::<BTreeMap<InputPortId, u8>>(json).is_err());
        }
        for json in [r#"{"0":1}"#, r#"{"-1":1}"#] {
            assert!(serde_json::from_str::<BTreeMap<OutputPortId, u8>>(json).is_err());
        }
    }

    // Unlike JSON, some formats distinguish named newtypes from bare integers.
    struct NamedIdDeserializer {
        expected_name: &'static str,
        signed: isize,
    }

    impl<'de> serde::Deserializer<'de> for NamedIdDeserializer {
        type Error = de::value::Error;

        fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Self::Error> {
            Err(de::Error::custom("expected a named newtype"))
        }

        fn deserialize_newtype_struct<V: Visitor<'de>>(
            self,
            name: &'static str,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            if name != self.expected_name {
                return Err(de::Error::custom("newtype name changed"));
            }
            visitor.visit_newtype_struct(self.signed.into_deserializer())
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct seq tuple tuple_struct map struct
            enum identifier ignored_any
        }
    }

    #[test]
    fn named_newtype_formats_keep_their_representation_and_validation() {
        let input = InputPortId::deserialize(NamedIdDeserializer {
            expected_name: "InputPortId",
            signed: -1,
        })
        .unwrap();
        let output = OutputPortId::deserialize(NamedIdDeserializer {
            expected_name: "OutputPortId",
            signed: 1,
        })
        .unwrap();
        assert_eq!(isize::from(input), -1);
        assert_eq!(isize::from(output), 1);
        assert!(
            InputPortId::deserialize(NamedIdDeserializer {
                expected_name: "InputPortId",
                signed: 0
            })
            .is_err()
        );
        assert!(
            OutputPortId::deserialize(NamedIdDeserializer {
                expected_name: "OutputPortId",
                signed: -1
            })
            .is_err()
        );
    }
}

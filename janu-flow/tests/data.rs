//
// Copyright (c) 2017, 2021 Tawedge.
//
// This program and the accompanying materials are made available under the
// terms of the TAW Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   open
//

use janu_flow::async_std::sync::Arc;
use janu_flow::serde::{Deserialize, Serialize};
use janu_flow::janu_flow_derive::ZFData;
use janu_flow::{Data, Deserializable, ZFData, ZFError, ZFResult};

#[derive(Debug, ZFData, Clone, Serialize, Deserialize)]
struct TestData {
    pub field1: u8,
    pub field2: String,
    pub field3: f64,
}

impl ZFData for TestData {
    fn try_serialize(&self) -> janu_flow::ZFResult<Vec<u8>> {
        Ok(serde_json::to_string(self)
            .map_err(|_| ZFError::SerializationError)?
            .as_bytes()
            .to_vec())
    }
}

impl Deserializable for TestData {
    fn try_deserialize(bytes: &[u8]) -> ZFResult<TestData>
    where
        Self: Sized,
    {
        let json = String::from_utf8_lossy(bytes);
        let data: TestData =
            serde_json::from_str(&json).map_err(|_| ZFError::DeseralizationError)?;
        Ok(data)
    }
}

#[test]
fn data_wrapping_unwrapping() {
    let test_data = TestData {
        field1: 16u8,
        field2: String::from("TestString"),
        field3: 123.456f64,
    };

    let mut wrapped_data = Data::from(test_data.clone());

    let unwrapped_data = wrapped_data.try_get::<TestData>().unwrap();

    assert_eq!(unwrapped_data.field1, test_data.field1);
    assert_eq!(unwrapped_data.field2, test_data.field2);
    assert!((unwrapped_data.field3 - test_data.field3).abs() < f64::EPSILON);

    let arc_data = Arc::new(test_data.clone());

    let mut wrapped_data = Data::from_arc(arc_data);
    let unwrapped_data = wrapped_data.try_get::<TestData>().unwrap();

    assert_eq!(unwrapped_data.field1, test_data.field1);
    assert_eq!(unwrapped_data.field2, test_data.field2);
    assert!((unwrapped_data.field3 - test_data.field3).abs() < f64::EPSILON);

    let serialized_data = test_data.try_serialize().unwrap();

    let mut wrapped_data = Data::from_bytes(serialized_data.clone());

    assert_eq!(
        Arc::from(serialized_data),
        wrapped_data.try_as_bytes().unwrap()
    );

    let unwrapped_data = wrapped_data.try_get::<TestData>().unwrap();

    assert_eq!(unwrapped_data.field1, test_data.field1);
    assert_eq!(unwrapped_data.field2, test_data.field2);
    assert!((unwrapped_data.field3 - test_data.field3).abs() < f64::EPSILON);
}

use lib0::any::Any;

pub fn bytes_from_yrs(value: yrs::types::Value) -> Option<Vec<u8>> {
    if let yrs::types::Value::Any(Any::Buffer(bytes)) = value {
        Some(bytes.to_vec())
    } else {
        None
    }
}

pub fn uint_from_yrs(value: yrs::types::Value) -> Option<u32> {
    if let yrs::types::Value::Any(any) = value {
        uint_from_yrs_any(&any)
    } else {
        None
    }
}

pub fn uint_from_yrs_any(value: &Any) -> Option<u32> {
    if let Any::Number(num) = value {
        Some(*num as u32)
    } else {
        None
    }
}

pub fn int64_from_yrs(value: yrs::types::Value) -> Option<i64> {
    if let yrs::types::Value::Any(Any::BigInt(num)) = value {
        Some(num)
    } else {
        None
    }
}

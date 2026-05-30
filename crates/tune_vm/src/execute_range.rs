use tune_runtime::value::{RangeItemKind, RangeValue, Value};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RangeItem {
    pub index: u64,
    pub value: Value,
}

pub(crate) fn range_len(range: RangeValue) -> Option<u64> {
    let exclusive_end = if range.inclusive {
        range.end.checked_add(1)?
    } else {
        range.end
    };
    u64::try_from(exclusive_end.saturating_sub(range.start).max(0)).ok()
}

pub(crate) fn range_item(range: RangeValue, index: u64) -> Option<RangeItem> {
    if index >= range_len(range)? {
        return None;
    }
    let value = range.start.checked_add(i128::from(index))?;
    Some(RangeItem {
        index,
        value: range_value(range.item, value)?,
    })
}

pub(crate) fn value_range(value: &Value) -> Option<RangeValue> {
    match value {
        Value::Range(range) => Some(*range),
        _ => None,
    }
}

fn range_value(kind: RangeItemKind, value: i128) -> Option<Value> {
    match kind {
        RangeItemKind::Int => i64::try_from(value).ok().map(Value::Int),
        RangeItemKind::Size => u64::try_from(value).ok().map(Value::Size),
    }
}

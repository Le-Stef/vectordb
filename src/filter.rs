use crate::vector::MetadataValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterValue {
    Direct(MetadataValue),
    Operator(FilterOperator),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOperator {
    #[serde(rename = "$ne", skip_serializing_if = "Option::is_none")]
    pub ne: Option<MetadataValue>,
    #[serde(rename = "$in", skip_serializing_if = "Option::is_none")]
    pub in_values: Option<Vec<MetadataValue>>,
    #[serde(rename = "$nin", skip_serializing_if = "Option::is_none")]
    pub nin: Option<Vec<MetadataValue>>,
}

pub type WhereFilter = HashMap<String, FilterValue>;

pub fn matches_filter(metadata: &HashMap<String, MetadataValue>, filter: &WhereFilter) -> bool {
    for (key, filter_value) in filter {
        let meta_val = metadata.get(key);

        match filter_value {
            FilterValue::Direct(expected) => {
                // égalité simple
                match meta_val {
                    Some(val) if val == expected => continue,
                    _ => return false,
                }
            }
            FilterValue::Operator(op) => {
                // opérateurs
                if let Some(ref ne_val) = op.ne {
                    match meta_val {
                        Some(val) if val == ne_val => return false,
                        None => return false,
                        _ => continue,
                    }
                }

                if let Some(ref in_vals) = op.in_values {
                    match meta_val {
                        Some(val) if in_vals.contains(val) => continue,
                        _ => return false,
                    }
                }

                if let Some(ref nin_vals) = op.nin {
                    match meta_val {
                        Some(val) if nin_vals.contains(val) => return false,
                        None => continue,
                        _ => continue,
                    }
                }
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_match() {
        let mut metadata = HashMap::new();
        metadata.insert("status".to_string(), MetadataValue::String("active".to_string()));

        let mut filter = HashMap::new();
        filter.insert(
            "status".to_string(),
            FilterValue::Direct(MetadataValue::String("active".to_string())),
        );

        assert!(matches_filter(&metadata, &filter));
    }

    #[test]
    fn test_ne_operator() {
        let mut metadata = HashMap::new();
        metadata.insert("status".to_string(), MetadataValue::String("active".to_string()));

        let mut filter = HashMap::new();
        filter.insert(
            "status".to_string(),
            FilterValue::Operator(FilterOperator {
                ne: Some(MetadataValue::String("inactive".to_string())),
                in_values: None,
                nin: None,
            }),
        );

        assert!(matches_filter(&metadata, &filter));
    }
}

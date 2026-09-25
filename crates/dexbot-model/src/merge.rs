use crate::{ModelError, Result};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

const DELETE_KEY: &str = "$delete";

/// Layer-name prefix marking a detected-hardware overlay (§6.3).
const DISCOVERY_PREFIX: &str = "discovery:";

/// True for provenance entries recorded by layers that carry explicit user
/// intent: user overlays and programmatic API layers.
fn user_layer(layer: &str) -> bool {
    layer.starts_with("overlay:") || layer.starts_with("api:")
}

/// Finds a user-intent provenance entry at `path` or anywhere beneath it.
/// Discovery layers must not override such values (§6.3): a subtree match
/// covers replacements and deletions that would swallow user-set leaves.
fn user_provenance<'a>(
    provenance: &'a BTreeMap<String, String>,
    path: &str,
) -> Option<(&'a str, &'a str)> {
    if let Some((key, layer)) = provenance.get_key_value(path) {
        if user_layer(layer) {
            return Some((key.as_str(), layer.as_str()));
        }
    }
    let prefix = format!("{path}.");
    provenance
        .range(prefix.clone()..)
        .take_while(|(key, _)| key.starts_with(&prefix))
        .find(|(_, layer)| user_layer(layer))
        .map(|(key, layer)| (key.as_str(), layer.as_str()))
}

/// Enforces the §6.3 discovery-conflict rule for a slot a discovery layer is
/// about to change. `existing`/`incoming` are the current and proposed states
/// of the slot, with `None` meaning absent (a deletion or a fresh insert).
/// Returns `Ok(true)` when the write must be skipped because the incoming
/// state equals the user-set state (idempotent facts keep their original
/// provenance) and an error when it differs — never a silent override.
/// `Ok(false)` means the write is not user-constrained.
fn check_discovery_conflict(
    existing: Option<&Value>,
    incoming: Option<&Value>,
    path: &str,
    layer: &str,
    provenance: &BTreeMap<String, String>,
) -> Result<bool> {
    if !layer.starts_with(DISCOVERY_PREFIX) {
        return Ok(false);
    }
    let Some((user_path, user_layer)) = user_provenance(provenance, path) else {
        return Ok(false);
    };
    if existing == incoming {
        return Ok(true);
    }
    Err(ModelError::DiscoveryConflict {
        path: user_path.to_string(),
        existing: user_layer.to_string(),
        incoming: layer.to_string(),
    })
}

/// True only for the exact deletion directive `{"$delete": true}`.
fn deletion(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.len() == 1 && object.get(DELETE_KEY).and_then(Value::as_bool) == Some(true)
}

/// Any other object shape mentioning `$delete` is a malformed directive and
/// must be rejected rather than merged as ordinary data.
fn malformed_deletion(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|object| object.contains_key(DELETE_KEY))
        && !deletion(value)
}

pub fn merge_layer(
    base: &mut Value,
    overlay: Value,
    layer: &str,
    provenance: &mut BTreeMap<String, String>,
) -> Result<()> {
    merge_value(base, overlay, "", layer, provenance)
}

fn merge_value(
    base: &mut Value,
    overlay: Value,
    path: &str,
    layer: &str,
    provenance: &mut BTreeMap<String, String>,
) -> Result<()> {
    if deletion(&overlay) {
        return Err(ModelError::InvalidDeletion(path.to_string()));
    }
    if malformed_deletion(&overlay) {
        return Err(ModelError::MalformedDeletion(path.to_string()));
    }
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            merge_map(base_map, overlay_map, path, layer, provenance)
        }
        (slot, value) => {
            if check_discovery_conflict(Some(&*slot), Some(&value), path, layer, provenance)? {
                return Ok(());
            }
            *slot = materialize(value, path, layer, provenance)?;
            Ok(())
        }
    }
}

fn merge_map(
    base: &mut Map<String, Value>,
    overlay: Map<String, Value>,
    path: &str,
    layer: &str,
    provenance: &mut BTreeMap<String, String>,
) -> Result<()> {
    for (key, value) in overlay {
        let child = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };
        if malformed_deletion(&value) {
            return Err(ModelError::MalformedDeletion(child));
        }
        if deletion(&value) {
            if check_discovery_conflict(base.get(&key), None, &child, layer, provenance)? {
                continue;
            }
            base.remove(&key);
            provenance.insert(child, format!("{layer} (deleted)"));
            continue;
        }
        match base.get_mut(&key) {
            Some(existing) => merge_value(existing, value, &child, layer, provenance)?,
            None => {
                check_discovery_conflict(None, Some(&value), &child, layer, provenance)?;
                let stored = materialize(value, &child, layer, provenance)?;
                base.insert(key, stored);
            }
        }
    }
    Ok(())
}

/// Converts an overlay value into a stored value with every deletion
/// directive consumed (objects) or rejected (arrays), so no `$delete` can
/// survive into resolved output. The caller has already rejected the case
/// where `value` itself is a (possibly malformed) directive.
fn materialize(
    value: Value,
    path: &str,
    layer: &str,
    provenance: &mut BTreeMap<String, String>,
) -> Result<Value> {
    match value {
        Value::Object(map) => {
            let mut fresh = Map::new();
            merge_map(&mut fresh, map, path, layer, provenance)?;
            if fresh.is_empty() {
                provenance.insert(path.to_string(), layer.to_string());
            }
            Ok(Value::Object(fresh))
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                reject_directives(item, &format!("{path}[{index}]"))?;
            }
            provenance.insert(path.to_string(), layer.to_string());
            Ok(Value::Array(items))
        }
        other => {
            provenance.insert(path.to_string(), layer.to_string());
            Ok(other)
        }
    }
}

/// Arrays replace wholesale and their elements never merge, so a deletion
/// directive inside an array can never take effect — reject it outright.
fn reject_directives(value: &Value, path: &str) -> Result<()> {
    match value {
        Value::Object(map) => {
            if map.contains_key(DELETE_KEY) {
                return Err(ModelError::MalformedDeletion(path.to_string()));
            }
            for (key, child) in map {
                reject_directives(child, &format!("{path}.{key}"))?;
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                reject_directives(item, &format!("{path}[{index}]"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn merge(base: Value, overlay: Value) -> Result<(Value, BTreeMap<String, String>)> {
        let mut merged = base;
        let mut provenance = BTreeMap::new();
        merge_layer(&mut merged, overlay, "test", &mut provenance)?;
        Ok((merged, provenance))
    }

    fn assert_no_delete_keys(value: &Value) {
        match value {
            Value::Object(map) => {
                assert!(!map.contains_key(DELETE_KEY), "leaked $delete in {value}");
                map.values().for_each(assert_no_delete_keys);
            }
            Value::Array(items) => items.iter().for_each(assert_no_delete_keys),
            _ => {}
        }
    }

    #[test]
    fn nested_delete_in_fresh_subtree_is_consumed() {
        let (merged, provenance) = merge(
            json!({}),
            json!({"metadata": {"brand_new": {"inner": {"$delete": true}}}}),
        )
        .expect("merge");
        assert_eq!(merged, json!({"metadata": {"brand_new": {}}}));
        assert_no_delete_keys(&merged);
        assert_eq!(
            provenance
                .get("metadata.brand_new.inner")
                .map(String::as_str),
            Some("test (deleted)")
        );
    }

    #[test]
    fn delete_in_type_conflict_replacement_is_consumed() {
        let (merged, _) = merge(
            json!({"keep": 1}),
            json!({"keep": {"x": {"$delete": true}, "y": 2}}),
        )
        .expect("merge");
        assert_eq!(merged, json!({"keep": {"y": 2}}));
        assert_no_delete_keys(&merged);
    }

    #[test]
    fn directive_with_sibling_keys_is_rejected() {
        let error = merge(json!({}), json!({"nested": {"$delete": true, "b": 2}}))
            .expect_err("must reject");
        assert!(matches!(error, ModelError::MalformedDeletion(path) if path == "nested"));
    }

    #[test]
    fn delete_false_is_rejected() {
        let error =
            merge(json!({"a": 1}), json!({"a": {"$delete": false}})).expect_err("must reject");
        assert!(matches!(error, ModelError::MalformedDeletion(path) if path == "a"));
    }

    #[test]
    fn directive_inside_array_is_rejected() {
        let error = merge(json!({}), json!({"list": [{"$delete": true}]})).expect_err("reject");
        assert!(matches!(error, ModelError::MalformedDeletion(path) if path == "list[0]"));
        let error = merge(json!({}), json!({"list": [{"inner": {"$delete": false}}]}))
            .expect_err("reject nested");
        assert!(matches!(error, ModelError::MalformedDeletion(path) if path == "list[0].inner"));
    }

    #[test]
    fn delete_of_missing_key_is_a_no_op() {
        let (merged, provenance) =
            merge(json!({"a": 1}), json!({"ghost": {"$delete": true}})).expect("merge");
        assert_eq!(merged, json!({"a": 1}));
        assert_eq!(
            provenance.get("ghost").map(String::as_str),
            Some("test (deleted)")
        );
    }

    #[test]
    fn root_deletion_is_rejected() {
        let error = merge(json!({"a": 1}), json!({"$delete": true})).expect_err("reject");
        assert!(matches!(error, ModelError::InvalidDeletion(_)));
    }

    #[test]
    fn discovery_layers_never_silently_override_user_set_values() {
        let user_provenance = || {
            BTreeMap::from([(
                "sensors.cam.enabled".to_string(),
                "overlay:user".to_string(),
            )])
        };
        let expect_conflict = |overlay: Value| {
            let mut base = json!({"sensors": {"cam": {"enabled": false}}});
            let mut provenance = user_provenance();
            let error = merge_layer(&mut base, overlay, "discovery:probe", &mut provenance)
                .expect_err("must conflict");
            assert!(matches!(
                &error,
                ModelError::DiscoveryConflict { path, existing, incoming }
                    if path == "sensors.cam.enabled"
                        && existing == "overlay:user"
                        && incoming == "discovery:probe"
            ));
        };
        // A differing leaf value, a subtree replacement, and a deletion all
        // contradict the user-set value.
        expect_conflict(json!({"sensors": {"cam": {"enabled": true}}}));
        expect_conflict(json!({"sensors": {"cam": 5}}));
        expect_conflict(json!({"sensors": {"cam": {"$delete": true}}}));
        // Restating the identical fact is a no-op that keeps user provenance.
        let mut base = json!({"sensors": {"cam": {"enabled": false}}});
        let mut provenance = user_provenance();
        merge_layer(
            &mut base,
            json!({"sensors": {"cam": {"enabled": false}}}),
            "discovery:probe",
            &mut provenance,
        )
        .expect("idempotent fact");
        assert_eq!(base, json!({"sensors": {"cam": {"enabled": false}}}));
        assert_eq!(
            provenance.get("sensors.cam.enabled").map(String::as_str),
            Some("overlay:user")
        );
        // Re-adding a key the user explicitly deleted is also a conflict.
        let mut base = json!({"sensors": {}});
        let mut provenance = BTreeMap::from([(
            "sensors.cam".to_string(),
            "overlay:user (deleted)".to_string(),
        )]);
        let error = merge_layer(
            &mut base,
            json!({"sensors": {"cam": {"enabled": true}}}),
            "discovery:probe",
            &mut provenance,
        )
        .expect_err("must conflict");
        assert!(matches!(error, ModelError::DiscoveryConflict { .. }));
        // Deleting a key the user already deleted agrees with the user.
        let mut base = json!({"sensors": {}});
        let mut provenance = BTreeMap::from([(
            "sensors.cam".to_string(),
            "overlay:user (deleted)".to_string(),
        )]);
        merge_layer(
            &mut base,
            json!({"sensors": {"cam": {"$delete": true}}}),
            "discovery:probe",
            &mut provenance,
        )
        .expect("agreeing deletion");
        assert_eq!(
            provenance.get("sensors.cam").map(String::as_str),
            Some("overlay:user (deleted)")
        );
    }

    #[test]
    fn deep_merge_and_scalar_replace_record_provenance() {
        let (merged, provenance) = merge(
            json!({"robot": {"model": "vega_1", "namespace": null}}),
            json!({"robot": {"namespace": "dm"}, "extra": {"a": [1, 2]}}),
        )
        .expect("merge");
        assert_eq!(
            merged,
            json!({"robot": {"model": "vega_1", "namespace": "dm"}, "extra": {"a": [1, 2]}})
        );
        assert_eq!(
            provenance.get("robot.namespace").map(String::as_str),
            Some("test")
        );
        assert_eq!(provenance.get("extra.a").map(String::as_str), Some("test"));
        assert!(!provenance.contains_key("robot.model"));
    }
}

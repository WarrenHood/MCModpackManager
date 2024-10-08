use std::{any::Any, default, str::FromStr};

#[derive(Debug, Clone, Copy)]
pub enum FileType {
    Json,
    Yaml,
    Toml,
}

impl FromStr for FileType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        Ok(if s.contains("json") {
            FileType::Json
        } else if s.contains("toml") {
            FileType::Toml
        } else if s.contains("yaml") || s.contains("yml") {
            FileType::Yaml
        } else {
            anyhow::bail!("Unmergable file type: {s}")
        })
    }
}

fn merge_json(
    src: &serde_json::Value,
    dst: &mut serde_json::Value,
    overwrite_existing: bool,
) -> anyhow::Result<()> {
    if src.is_object() && dst.is_object() {
        let src = src.as_object().unwrap();
        let dst = dst.as_object_mut().unwrap();

        for (k, v) in src.iter() {
            if v.is_object() {
                let dst_v = dst.entry(k).or_insert(serde_json::json!({}));
                merge_json(v, dst_v, overwrite_existing)?;
            } else {
                if overwrite_existing || !dst.contains_key(k) {
                    dst.insert(k.to_string(), v.clone());
                }
            }
        }
    } else {
        // TODO: Keep track of path for better errors
        anyhow::bail!("Cannot merge non-objects: {src:#?} and {dst:#?}")
    }
    Ok(())
}

#[test]
fn test_merge_json() {
    let src = serde_json::json!({
        "a": 3,
        "b": {
            "x": {

            },
            "y": {
                "test": "thing"
            }
        },
        "c": {}
    });
    let dst = serde_json::json!({
        "b": {
            "y": {
                "test": "something"
            }
        },
        "c": {
            "foo": "bar"
        }
    });

    let mut merged_overwrite = dst.clone();
    let mut merged_retained = dst.clone();
    merge_json(&src, &mut merged_overwrite, true).unwrap();
    merge_json(&src, &mut merged_retained, false).unwrap();

    assert!(
        merged_overwrite["b"]["y"]["test"] == "thing",
        "//b/y/test wasn't overwritten with \"thing\". src={}, dst={}",
        src,
        merged_overwrite
    );
    assert!(
        merged_overwrite["a"] == 3,
        "//a was not set to 3. src={}, dst={}",
        src,
        merged_overwrite
    );
    assert!(
        merged_overwrite["b"]["x"].is_object(),
        "//b/x is not an object. src={}, dst={}",
        src,
        merged_overwrite
    );
    assert!(
        merged_overwrite["c"]["foo"] == "bar",
        "//c/foo != bar. src={}, dst={}",
        src,
        merged_overwrite
    );

    assert!(
        merged_retained["b"]["y"]["test"] == "something",
        "//b/y/test was overwritten. src={}, dst={}",
        src,
        merged_retained
    );
    assert!(
        merged_retained["a"] == 3,
        "//a was not set to 3. src={}, dst={}",
        src,
        merged_retained
    );
    assert!(
        merged_retained["b"]["x"].is_object(),
        "//b/x is not an object. src={}, dst={}",
        src,
        merged_retained
    );
    assert!(
        merged_retained["c"]["foo"] == "bar",
        "//c/foo != bar. src={}, dst={}",
        src,
        merged_retained
    );
}

/// Merge `src` into `dst` if it is a supported file type
fn merge_files(
    src: &str,
    dst: &str,
    overwrite_existing: bool,
    file_type: FileType,
) -> anyhow::Result<String> {
    Ok(match file_type {
        FileType::Json => {
            let src_val = serde_json::from_str(src)?;
            let mut dst_val = serde_json::from_str(dst)?;
            merge_json(&src_val, &mut dst_val, overwrite_existing)?;
            dst_val.to_string()
        }
        FileType::Yaml => todo!(),
        FileType::Toml => todo!(),
    })
}

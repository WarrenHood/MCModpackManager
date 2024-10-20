use std::str::FromStr;

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
        "//b/y/test wasn't overwritten with \"thing\". src={:#?}, dst={:#?}",
        src,
        merged_overwrite
    );
    assert!(
        merged_overwrite["a"] == 3,
        "//a was not set to 3. src={:#?}, dst={:#?}",
        src,
        merged_overwrite
    );
    assert!(
        merged_overwrite["b"]["x"].is_object(),
        "//b/x is not an object. src={:#?}, dst={:#?}",
        src,
        merged_overwrite
    );
    assert!(
        merged_overwrite["c"]["foo"] == "bar",
        "//c/foo != bar. src={:#?}, dst={:#?}",
        src,
        merged_overwrite
    );

    assert!(
        merged_retained["b"]["y"]["test"] == "something",
        "//b/y/test was overwritten. src={:#?}, dst={:#?}",
        src,
        merged_retained
    );
    assert!(
        merged_retained["a"] == 3,
        "//a was not set to 3. src={:#?}, dst={:#?}",
        src,
        merged_retained
    );
    assert!(
        merged_retained["b"]["x"].is_object(),
        "//b/x is not an object. src={:#?}, dst={:#?}",
        src,
        merged_retained
    );
    assert!(
        merged_retained["c"]["foo"] == "bar",
        "//c/foo != bar. src={:#?}, dst={:#?}",
        src,
        merged_retained
    );
}

fn merge_yaml(
    src: &serde_yaml::Value,
    dst: &mut serde_yaml::Value,
    overwrite_existing: bool,
) -> anyhow::Result<()> {
    if src.is_mapping() && dst.is_mapping() {
        let src = src.as_mapping().unwrap();
        let dst = dst.as_mapping_mut().unwrap();

        for (k, v) in src.iter() {
            if v.is_mapping() {
                let dst_v = dst.entry(k.clone()).or_insert(serde_yaml::from_str("{}")?);
                merge_yaml(v, dst_v, overwrite_existing)?;
            } else {
                if overwrite_existing || !dst.contains_key(k) {
                    dst.insert(k.clone(), v.clone());
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
fn test_merge_yaml() {
    let src = serde_yaml::from_str(
        r#"{
        "a": 3,
        "b": {
            "x": {

            },
            "y": {
                "test": "thing"
            }
        },
        "c": {}
    }"#,
    )
    .unwrap();

    let dst: serde_yaml::Value = serde_yaml::from_str(
        r#"{
        "b": {
            "y": {
                "test": "something"
            }
        },
        "c": {
            "foo": "bar"
        }
    }"#,
    )
    .unwrap();

    let mut merged_overwrite = dst.clone();
    let mut merged_retained = dst.clone();
    merge_yaml(&src, &mut merged_overwrite, true).unwrap();
    merge_yaml(&src, &mut merged_retained, false).unwrap();

    assert!(
        merged_overwrite["b"]["y"]["test"] == "thing",
        "//b/y/test wasn't overwritten with \"thing\". src={:#?}, dst={:#?}",
        src,
        merged_overwrite
    );
    assert!(
        merged_overwrite["a"] == 3,
        "//a was not set to 3. src={:#?}, dst={:#?}",
        src,
        merged_overwrite
    );
    assert!(
        merged_overwrite["b"]["x"].is_mapping(),
        "//b/x is not a mapping. src={:#?}, dst={:#?}",
        src,
        merged_overwrite
    );
    assert!(
        merged_overwrite["c"]["foo"] == "bar",
        "//c/foo != bar. src={:#?}, dst={:#?}",
        src,
        merged_overwrite
    );

    assert!(
        merged_retained["b"]["y"]["test"] == "something",
        "//b/y/test was overwritten. src={:#?}, dst={:#?}",
        src,
        merged_retained
    );
    assert!(
        merged_retained["a"] == 3,
        "//a was not set to 3. src={:#?}, dst={:#?}",
        src,
        merged_retained
    );
    assert!(
        merged_retained["b"]["x"].is_mapping(),
        "//b/x is not a mapping. src={:#?}, dst={:#?}",
        src,
        merged_retained
    );
    assert!(
        merged_retained["c"]["foo"] == "bar",
        "//c/foo != bar. src={:#?}, dst={:#?}",
        src,
        merged_retained
    );
}

fn merge_toml(
    src: &toml::Value,
    dst: &mut toml::Value,
    overwrite_existing: bool,
) -> anyhow::Result<()> {
    if src.is_table() && dst.is_table() {
        let src = src.as_table().unwrap();
        let dst = dst.as_table_mut().unwrap();

        for (k, v) in src.iter() {
            if v.is_table() {
                let dst_v = dst.entry(k.clone()).or_insert(serde_yaml::from_str("{}")?);
                merge_toml(v, dst_v, overwrite_existing)?;
            } else {
                if overwrite_existing || !dst.contains_key(k) {
                    dst.insert(k.clone(), v.clone());
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
fn test_merge_toml() {
    let src: toml::Value = toml::from_str(
        r#"
        a = 3

        [b]
        [b.x]

        [b.y]
        test = "thing"

        [c]
    "#,
    )
    .unwrap();

    let mut dst: toml::Value = toml::from_str(
        r#"
        [b]
        [b.y]
        test = "something"

        [c]
        foo = "bar"
    "#,
    )
    .unwrap();

    let mut merged_overwrite = dst.clone();
    let mut merged_retained = dst.clone();
    merge_toml(&src, &mut merged_overwrite, true).unwrap();
    merge_toml(&src, &mut merged_retained, false).unwrap();

    assert!(
        merged_overwrite["b"]["y"]["test"] == "thing".into(),
        "//b/y/test wasn't overwritten with \"thing\". src={:#?}, dst={:#?}",
        src,
        merged_overwrite
    );
    assert!(
        merged_overwrite["a"] == 3.into(),
        "//a was not set to 3. src={:#?}, dst={:#?}",
        src,
        merged_overwrite
    );
    assert!(
        merged_overwrite["b"]["x"].is_table(),
        "//b/x is not a mapping. src={:#?}, dst={:#?}",
        src,
        merged_overwrite
    );
    assert!(
        merged_overwrite["c"]["foo"] == "bar".into(),
        "//c/foo != bar. src={:#?}, dst={:#?}",
        src,
        merged_overwrite
    );

    assert!(
        merged_retained["b"]["y"]["test"] == "something".into(),
        "//b/y/test was overwritten. src={:#?}, dst={:#?}",
        src,
        merged_retained
    );
    assert!(
        merged_retained["a"] == 3.into(),
        "//a was not set to 3. src={:#?}, dst={:#?}",
        src,
        merged_retained
    );
    assert!(
        merged_retained["b"]["x"].is_table(),
        "//b/x is not a mapping. src={:#?}, dst={:#?}",
        src,
        merged_retained
    );
    assert!(
        merged_retained["c"]["foo"] == "bar".into(),
        "//c/foo != bar. src={:#?}, dst={:#?}",
        src,
        merged_retained
    );
}

/// Merge `src` into `dst` if it is a supported file type
pub fn merge_files(
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
        FileType::Yaml => {
            let src_val = serde_yaml::Value::from(src);
            let mut dst_val = serde_yaml::Value::from(dst);
            merge_yaml(&src_val, &mut dst_val, overwrite_existing)?;
            serde_yaml::to_string(&dst_val)?
        }
        FileType::Toml => {
            let src_val: toml::Value = toml::from_str(src)?;
            let mut dst_val: toml::Value = toml::from_str(dst)?;
            merge_toml(&src_val, &mut dst_val, overwrite_existing)?;
            dst_val.to_string()
        }
    })
}

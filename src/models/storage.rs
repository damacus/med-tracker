use std::collections::BTreeMap;

use serde_json::Value;

pub(crate) const AVATAR_STORE: &str = "avatar";
pub(crate) const DMD_STORE: &str = "dmd";

#[derive(Clone)]
pub struct Operators {
    pub avatar: Option<opendal::Operator>,
    pub dmd: Option<opendal::Operator>,
}

pub fn configured(settings: Option<&Value>) -> Option<(loco_rs::storage::Storage, Operators)> {
    let avatar = avatar_operator(settings, None);
    let dmd = dmd_operator(settings, None);
    let mut stores: BTreeMap<String, Box<dyn loco_rs::storage::drivers::StoreDriver>> =
        BTreeMap::new();
    if let Some(operator) = &avatar {
        stores.insert(
            AVATAR_STORE.to_string(),
            Box::new(
                loco_rs::storage::drivers::opendal_adapter::OpendalAdapter::new(operator.clone()),
            ),
        );
    }
    if let Some(operator) = &dmd {
        stores.insert(
            DMD_STORE.to_string(),
            Box::new(
                loco_rs::storage::drivers::opendal_adapter::OpendalAdapter::new(operator.clone()),
            ),
        );
    }
    if stores.is_empty() {
        return None;
    }
    let primary = if stores.contains_key(AVATAR_STORE) {
        AVATAR_STORE
    } else {
        DMD_STORE
    };
    Some((
        loco_rs::storage::Storage::new(
            stores,
            Box::new(loco_rs::storage::strategies::single::SingleStrategy::new(
                primary,
            )),
        ),
        Operators { avatar, dmd },
    ))
}

pub(crate) fn avatar_operator(
    settings: Option<&Value>,
    bucket: Option<&str>,
) -> Option<opendal::Operator> {
    let section = "avatar_storage";
    let text =
        |key: &str| crate::models::settings::text(settings, &[section, key]).map(str::to_owned);
    let endpoint = text("endpoint")?;
    let configured_bucket = text("bucket")?;
    let region = text("region")?;
    let access_key = text("access_key_id")?;
    let secret_key = text("secret_access_key")?;
    let path_style =
        crate::models::settings::flag(settings, &[section, "force_path_style"]).unwrap_or(true);
    Some(s3_operator(
        &endpoint,
        bucket.unwrap_or(&configured_bucket),
        &region,
        &access_key,
        &secret_key,
        path_style,
    ))
}

fn dmd_operator(settings: Option<&Value>, bucket: Option<&str>) -> Option<opendal::Operator> {
    let section = "dmd_storage";
    let text =
        |key: &str| crate::models::settings::text(settings, &[section, key]).map(str::to_owned);
    let endpoint = text("endpoint")?;
    let configured_bucket = text("bucket")?;
    let access_key = text("access_key_id")?;
    let secret_key = text("secret_access_key")?;
    let region = text("region").unwrap_or_else(|| "us-east-1".to_string());
    Some(s3_operator(
        &endpoint,
        bucket.unwrap_or(&configured_bucket),
        &region,
        &access_key,
        &secret_key,
        true,
    ))
}

fn s3_operator(
    endpoint: &str,
    bucket: &str,
    region: &str,
    access_key: &str,
    secret_key: &str,
    path_style: bool,
) -> opendal::Operator {
    opendal::install_default();
    let mut builder = opendal::services::S3::default()
        .bucket(bucket)
        .endpoint(endpoint)
        .region(region)
        .access_key_id(access_key)
        .secret_access_key(secret_key);
    if !path_style {
        builder = builder.enable_virtual_host_style();
    }
    opendal::Operator::new(builder).expect("S3 operator requires a bucket")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn avatar_settings() -> Value {
        json!({
            "avatar_storage": {
                "endpoint": "http://127.0.0.1:9000",
                "bucket": "avatars",
                "region": "us-east-1",
                "access_key_id": "key",
                "secret_access_key": "secret",
            }
        })
    }

    #[test]
    fn unconfigured_settings_produce_no_stores() {
        assert!(configured(None).is_none());
        assert!(configured(Some(&json!({}))).is_none());
    }

    #[test]
    fn fully_configured_avatar_storage_registers_named_store() {
        let (storage, operators) = configured(Some(&avatar_settings())).unwrap();
        assert!(storage.as_store(AVATAR_STORE).is_some());
        assert!(storage.as_store(DMD_STORE).is_none());
        assert!(operators.avatar.is_some());
        assert!(operators.dmd.is_none());
    }

    #[test]
    fn partially_configured_storage_is_skipped() {
        let mut settings = avatar_settings();
        settings["avatar_storage"]["bucket"] = Value::Null;
        assert!(configured(Some(&settings)).is_none());
    }

    #[test]
    fn dmd_storage_defaults_region_and_registers() {
        let settings = json!({
            "dmd_storage": {
                "endpoint": "http://127.0.0.1:9000",
                "bucket": "dmd",
                "access_key_id": "key",
                "secret_access_key": "secret",
            }
        });
        let (storage, operators) = configured(Some(&settings)).unwrap();
        assert!(storage.as_store(DMD_STORE).is_some());
        assert!(operators.dmd.is_some());
    }

    #[test]
    fn empty_rendered_values_count_as_unconfigured() {
        let settings = json!({
            "avatar_storage": {
                "endpoint": "",
                "bucket": "avatars",
                "region": "us-east-1",
                "access_key_id": "key",
                "secret_access_key": "secret",
            }
        });
        assert!(configured(Some(&settings)).is_none());
    }
}

//! Expand the fixed dashboard defaults for backend deployment verification.
//! Interactive values are interpolated by Grafana with its sqlstring formatter.

pub fn verification_sql(raw: &str) -> Option<String> {
    let mut sql = raw.replace("$$", "$");
    for name in ["os", "runtime", "version", "install", "device", "purpose"] {
        sql = sql.replace(&format!("${{{name}:sqlstring}}"), "'__all'");
    }
    if sql.contains("${") {
        return None;
    }
    Some(sql)
}

#[cfg(test)]
mod tests {
    use super::verification_sql;

    #[test]
    fn backend_verification_resolves_defaults_and_rejects_unknown_variables() {
        assert_eq!(
            verification_sql("SELECT $$__timeInterval(t) WHERE os IN ($${os:sqlstring})"),
            Some("SELECT $__timeInterval(t) WHERE os IN ('__all')".to_owned())
        );
        assert!(verification_sql("SELECT $${unconfigured:sqlstring}").is_none());
        assert!(verification_sql("SELECT $${os:raw}").is_none());
    }
}

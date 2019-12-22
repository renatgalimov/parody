use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Config {
    pub query_for_dir: Vec<String>,
    pub root_dir: PathBuf,
}

impl Config {
    pub fn with_root_dir(mut self, root_dir: PathBuf) -> Self {
        self.root_dir = root_dir;
        self
    }

    pub fn with_query(mut self, query: &str) -> Self {
        match self
            .query_for_dir
            .binary_search_by(|probe: &String| probe.as_str().cmp(query))
        {
            Err(index) => self.query_for_dir.insert(index, query.to_string()),
            _ => {}
        };

        self
    }

    pub fn should_use_query_in_path(&self, query: &str) -> bool {
        self.query_for_dir
            .binary_search_by(|probe| probe.as_str().cmp(query))
            .is_ok()
    }

    pub fn get_root_dir(&self) -> &Path {
        return self.root_dir.as_ref();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_config_with_query_should_insert_query_sorted() {
        let config = Config::default()
            .with_query("foo")
            .with_query("1delta")
            .with_query("bar")
            .with_query("foo");

        assert_eq!(config.query_for_dir, vec!["1delta", "bar", "foo"]);
    }

    #[test]
    fn test_config_with_query_when_has_query_for_dir_should_return_true() {
        assert_eq!(
            Config::default()
                .with_query("foo")
                .should_use_query_in_path("foo"),
            true
        );
    }

    #[test]
    fn test_config_with_query_when_no_query_for_dir_should_return_true() {
        assert_eq!(
            Config::default()
                .with_query("foo")
                .should_use_query_in_path("bar"),
            false
        );
    }
}

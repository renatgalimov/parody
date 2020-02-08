use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum QueryInPath {
    None,
    All,
    Selected(Vec<String>),
}

impl Default for QueryInPath {
    fn default() -> Self {
        QueryInPath::All
    }
}

#[derive(Default, Clone)]
pub struct Config {
    pub query_in_path: QueryInPath,
    pub root_dir: PathBuf,
}

impl Config {
    pub fn set_root_dir(&mut self, root_dir: PathBuf) -> &Self {
        self.root_dir = root_dir;
        self
    }

    pub fn with_root_dir(mut self, root_dir: PathBuf) -> Self {
        self.root_dir = root_dir;
        self
    }

    pub fn use_no_query_path(&mut self) -> &Self {
        self.query_in_path = QueryInPath::None;
        self
    }

    pub fn with_no_query_path(mut self) -> Self {
        self.use_no_query_path();
        self
    }

    pub fn use_all_query_path(&mut self) -> &Self {
        self.query_in_path = QueryInPath::All;
        self
    }

    pub fn with_all_query_path(mut self) -> Self {
        self.use_all_query_path();
        self
    }

    pub fn use_query_path(&mut self, query: &str) -> &Self {
        let query_in_path = match &mut self.query_in_path {
            QueryInPath::Selected(value) => value,
            QueryInPath::All | QueryInPath::None => {
                self.query_in_path = QueryInPath::Selected(vec![query.to_owned()]);
                return self;
            }
        };

        match query_in_path.binary_search_by(|probe: &String| probe.as_str().cmp(query)) {
            Err(index) => query_in_path.insert(index, query.to_string()),
            _ => {}
        };

        self
    }

    pub fn with_query_path(mut self, query: &str) -> Self {
        self.use_query_path(query);
        self
    }

    pub fn is_query_in_path(&self, query: &str) -> bool {
        trace!("Checking if query is in path: {}", query);

        match &self.query_in_path {
            QueryInPath::Selected(queries) => queries
                .binary_search_by(|probe| probe.as_str().cmp(query))
                .is_ok(),
            QueryInPath::All => {
                trace!("All queries accepted in path: {}", query);
                true
            }
            QueryInPath::None => {
                trace!("No queries accepted in path: {}", query);
                false
            }
        }
    }

    pub fn get_root_dir(&self) -> &Path {
        self.root_dir.as_ref()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_config_with_query_should_insert_query_sorted() {
        let config = Config::default()
            .with_query_path("foo")
            .with_query_path("1delta")
            .with_query_path("bar")
            .with_query_path("foo");

        match config.query_in_path {
            QueryInPath::Selected(queries) => assert_eq!(queries, vec!["1delta", "bar", "foo"]),
            _ => panic!(),
        };
    }

    #[test]
    fn test_config_with_query_when_has_query_in_path_should_return_true() {
        assert_eq!(
            Config::default()
                .use_query_path("foo")
                .is_query_in_path("foo"),
            true
        )
    }

    #[test]
    fn test_config_with_query_when_no_query_in_path_should_return_true() {
        assert_eq!(
            Config::default()
                .with_query_path("foo")
                .is_query_in_path("bar"),
            false
        );
    }
}

use crate::{error::Error, request::ParodyRequest, storage, storage::Storage};
use iron::{middleware::BeforeMiddleware, typemap::Key, IronError, IronResult};
use std::path::PathBuf;

#[cfg(test)]
mod test;

impl ParodyRequest for iron::Request<'_, '_> {
    fn get_method(&self) -> std::string::String {
        self.method.as_ref().to_owned()
    }
    fn get_url(&self) -> url::Url {
        self.url.clone().into()
    }
}

pub struct CacheMiddleware {
    storage_config: storage::Config,
}

impl CacheMiddleware {
    pub fn new() -> Self {
        Self {
            storage_config: storage::Config::default(),
        }
    }

    pub fn with_root_dir(mut self, root_dir: PathBuf) -> Self {
        self.storage_config.set_root_dir(root_dir);
        self
    }

    pub fn set_root_dir(&mut self, root_dir: PathBuf) -> &Self {
        self.storage_config.set_root_dir(root_dir);
        self
    }
}

#[derive(Clone, Copy)]
pub struct ResponseCache;
impl Key for ResponseCache {
    type Value = storage::Storage;
}

#[derive(Clone, Copy)]
struct CachedResponse;
impl Key for CachedResponse {
    type Value = iron::Response;
}

impl BeforeMiddleware for CacheMiddleware {
    fn before(&self, req: &mut iron::Request) -> IronResult<()> {
        trace!("Entered BeforeMiddleware::before");

        let storage = Storage::new_with_config(req, self.storage_config.clone()).map_err(
            |error| match error {
                Error::Common(error) => error.into(),
                _ => IronError::new(Box::new(error), iron::status::InternalServerError),
            },
        )?;

        req.extensions.insert::<ResponseCache>(storage);

        Ok(())
    }

    fn catch(&self, _req: &mut iron::Request, err: IronError) -> IronResult<()> {
        trace!("Entered BeforeMiddleware::catch");
        Err(err)
    }
}

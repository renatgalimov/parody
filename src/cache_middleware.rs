use crate::{storage, storage::Storage};
use iron::{
    middleware::{AfterMiddleware, BeforeMiddleware},
    typemap::Key,
    IronError, IronResult,
};

struct CacheMiddleware {
    storage_config: Vec<storage::Config>,
}

impl Key for Storage {
    type Value = Storage;
}

impl BeforeMiddleware for CacheMiddleware {
    /// Saves a storage for the request in the local context
    fn before(&self, req: &mut iron::Request) -> IronResult<()> {
        Ok(())
    }

    fn catch(&self, req: &mut iron::Request, err: IronError) -> IronResult<()> {
        Err(err)
    }
}

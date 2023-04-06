use reqwest::{Client, Request, Response};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Middleware, Next, Result};
use task_local_extensions::Extensions;

use crate::managers::UnsafeAppRef;

pub fn new(app: UnsafeAppRef) -> ClientWithMiddleware {
    let client = Client::builder().build().unwrap();

    ClientBuilder::new(client)
        .with(CacheMiddleware { app })
        .build()
}

struct CacheMiddleware {
    app: UnsafeAppRef,
}

#[async_trait::async_trait]
impl Middleware for CacheMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        // SAFETY: Requests cannot be made before the appref is initialized
        let _app = unsafe { self.app.upgrade() };

        next.run(req, extensions).await
    }
}

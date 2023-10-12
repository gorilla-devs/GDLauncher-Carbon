#[cfg(feature = "production")]
#[inline(always)]
pub fn get_client() -> reqwest_middleware::ClientBuilder {
    iridium::get_client()
}

#[cfg(not(feature = "production"))]
#[inline(always)]
pub fn get_client() -> reqwest_middleware::ClientBuilder {
    use reqwest::{Request, Response};
    use reqwest_middleware::{Middleware, Next};

    struct AddHeaderMiddleware;

    #[async_trait::async_trait]
    impl Middleware for AddHeaderMiddleware {
        async fn handle(
            &self,
            req: Request,
            _extensions: &mut task_local_extensions::Extensions,
            next: Next<'_>,
        ) -> reqwest_middleware::Result<Response> {
            // Add the header to the request.
            // req.headers_mut().insert(
            //     "authentication",
            //     std::env::var("API_AUTH").unwrap().parse().unwrap(),
            // );

            // Continue with the modified request.
            next.run(req, _extensions).await
        }
    }

    let client = reqwest::Client::builder().build().unwrap();
    reqwest_middleware::ClientBuilder::new(client).with(AddHeaderMiddleware)
}

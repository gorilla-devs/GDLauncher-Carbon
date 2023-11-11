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

    use crate::managers::GDL_API_BASE;

    struct AddHeaderMiddleware;

    #[async_trait::async_trait]
    impl Middleware for AddHeaderMiddleware {
        async fn handle(
            &self,
            req: Request,
            _extensions: &mut task_local_extensions::Extensions,
            next: Next<'_>,
        ) -> reqwest_middleware::Result<Response> {
            let gdl_api_base_host = url::Url::parse(GDL_API_BASE).unwrap();

            let opt_auth = option_env!("GDL_AUTH");

            if req.url().host_str() == gdl_api_base_host.host_str() && opt_auth.is_some() {
                let mut req = req;
                req.headers_mut()
                    .insert("GDL-Auth", opt_auth.unwrap().parse().unwrap());
                return next.run(req, _extensions).await;
            }

            // Continue with the modified request.
            next.run(req, _extensions).await
        }
    }

    let client = reqwest::Client::builder()
        .user_agent(format!(
            "{} {}",
            env!("USER_AGENT_PREFIX"),
            env!("APP_VERSION")
        ))
        .build()
        .unwrap();
    reqwest_middleware::ClientBuilder::new(client).with(AddHeaderMiddleware)
}

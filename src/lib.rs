//! Implementation of the [inertia.js] protocol for axum.
//!
//! [inertia.js]: https://inertiajs.com
//!
//! # Getting started
//!
//! The [Inertia] struct is available as an axum [Extractor] and can
//! be used in handlers like so:
//!
//! ```rust
//! use axum::response::IntoResponse;
//! use axum_inertia::Inertia;
//! use serde_json::json;
//!
//! async fn get_posts(i: Inertia) -> impl IntoResponse {
//!     i.render("Posts/Index", json!({ "posts": vec!["post one", "post two"] }))
//! }
//! ```
//! [Extractor]: https://docs.rs/axum/latest/axum/#extractors
//!
//! The extractor requires that you use [Router#with_state] to
//! initialize Inertia. In fact, it won't compile if you don't!
//!
//! The renderer needs to know how to build the initial page load. You
//! can pass a standard development Vite configuration like so:
//!
//! [Router#with_state]: https://docs.rs/axum/latest/axum/struct.Router.html#method.with_state
//!
//! ```rust
//! use axum_inertia::{vite, Inertia};
//! use axum::{Router, routing::get, response::IntoResponse};
//!
//! # async fn get_posts(_i: Inertia) -> impl IntoResponse { "foo" }
//! // Configuration for Inertia using `vite dev`:
//! let inertia = vite::Development::new()
//!     .port(5173)
//!     .main("src/main.ts")
//!     .lang("en")
//!     .title("Tuvu")
//!     .inertia();
//! let app: Router = Router::new()
//!     .route("/", get(get_posts))
//!     .with_state(inertia);
//! ```

use async_trait::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use http::{request::Parts, HeaderMap, HeaderValue, StatusCode};
use page::Page;
use request::Request;
use response::Response;
use serde::Serialize;
use std::sync::Arc;

mod page;
mod request;
mod response;
pub mod vite;

#[derive(Clone)]
pub struct Inertia {
    request: Option<Request>,
    version: Option<String>,
    /// A function from the serialized page props to the initial page
    /// load html.
    layout: Arc<Box<dyn Fn(String) -> String + Send + Sync>>,
}

#[async_trait]
impl<S> FromRequestParts<S> for Inertia
where
    S: Send + Sync,
    Inertia: FromRef<S>,
{
    type Rejection = (StatusCode, HeaderMap<HeaderValue>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let mut inertia = Inertia::from_ref(state);
        let request = Request::from_request_parts(parts, state).await?;

        // Respond with a 409 conflict if X-Inertia-Version values
        // don't match for GET requests. See more at:
        // https://inertiajs.com/the-protocol#asset-versioning
        if parts.method == "GET"
            && request.is_xhr
            && inertia.version.is_some()
            && request.version != inertia.version
        {
            let mut headers = HeaderMap::new();
            headers.insert("X-Inertia-Location", parts.uri.path().parse().unwrap());
            return Err((StatusCode::CONFLICT, headers));
        }

        inertia.request = Some(request);
        Ok(inertia)
    }
}

impl Inertia {
    /// Constructs a new Inertia object.
    ///
    /// `layout` provides information about how to render the initial
    /// page load. See the [crate::vite] module for an implementation
    /// of this for vite.
    pub fn new(
        version: Option<String>,
        layout: Box<dyn Fn(String) -> String + Send + Sync>,
    ) -> Inertia {
        Inertia {
            request: None,
            version,
            layout: Arc::new(layout),
        }
    }

    /// Renders an Inertia response.
    pub fn render<S: Serialize>(self, component: &'static str, props: S) -> Response {
        let request = self.request.expect("no request set");
        let url = request.url.clone();
        let page = Page {
            component,
            props: serde_json::to_value(props).expect("serialize"),
            url,
            version: self.version.clone(),
        };
        Response {
            page,
            request,
            layout: self.layout,
            version: self.version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{self, response::IntoResponse, routing::get, Router, Server};
    use reqwest::StatusCode;
    use serde_json::json;
    use std::net::TcpListener;

    #[tokio::test]
    async fn it_works() {
        async fn handler(i: Inertia) -> impl IntoResponse {
            i.render("foo!", json!({"bar": "baz"}))
        }

        let layout =
            Box::new(|props| format!(r#"<html><body><div id="app" data-page='{}'></div>"#, props));

        let inertia = Inertia::new(Some("123".to_string()), layout);

        let app = Router::new()
            .route("/test", get(handler))
            .with_state(inertia);

        let listener = TcpListener::bind("127.0.0.1:0").expect("Could not bind ephemeral socket");
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let server = Server::from_tcp(listener)
                .unwrap()
                .serve(app.into_make_service());
            server.await.expect("server error");
        });

        let res = reqwest::get(format!("http://{}/test", &addr))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers()
                .get("X-Inertia-Version")
                .map(|h| h.to_str().unwrap()),
            Some("123")
        );
    }

    #[tokio::test]
    async fn it_responds_with_conflict_on_version_mismatch() {
        async fn handler(i: Inertia) -> impl IntoResponse {
            i.render("foo!", json!({"bar": "baz"}))
        }

        let layout =
            Box::new(|props| format!(r#"<html><body><div id="app" data-page='{}'></div>"#, props));

        let inertia = Inertia::new(Some("123".to_string()), layout);

        let app = Router::new()
            .route("/test", get(handler))
            .with_state(inertia);

        let listener = TcpListener::bind("127.0.0.1:0").expect("Could not bind ephemeral socket");
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let server = Server::from_tcp(listener)
                .unwrap()
                .serve(app.into_make_service());
            server.await.expect("server error");
        });

        let client = reqwest::Client::new();

        let res = client
            .get(format!("http://{}/test", &addr))
            .header("X-Inertia", "true")
            .header("X-Inertia-Version", "456")
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::CONFLICT);
        assert_eq!(
            res.headers()
                .get("X-Inertia-Location")
                .map(|h| h.to_str().unwrap()),
            Some("/test")
        );
    }
}

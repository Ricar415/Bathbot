use std::time::Instant;

use bytes::Bytes;
use eyre::{Result, WrapErr};
use http::{header::CONTENT_LENGTH, Response};
use hyper::{
    client::{connect::dns::GaiResolver, Client as HyperClient, HttpConnector},
    header::{CONTENT_TYPE, USER_AGENT},
    Body, Error as HyperError, Method, Request,
};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use leaky_bucket_lite::LeakyBucket;
use prometheus::Registry;
use tokio::time::Duration;

use crate::{metrics::ClientMetrics, multipart::Multipart, ClientError, Site, MY_USER_AGENT};

const INTERNAL_ERROR: &str = "500";

pub(crate) type InnerClient = HyperClient<HttpsConnector<HttpConnector<GaiResolver>>, Body>;

pub struct Client {
    pub(crate) client: InnerClient,
    #[cfg(feature = "twitch")]
    twitch: bathbot_model::TwitchData,
    ratelimiters: [LeakyBucket; 13],
    metrics: ClientMetrics,
}

impl Client {
    /// `twitch_login` consists of `(twitch client id, twitch token)`
    pub async fn new(
        #[cfg(feature = "twitch")] (twitch_client_id, twitch_token): (&str, &str),
        metrics: &Registry,
    ) -> Result<Self> {
        let metrics = ClientMetrics::new(metrics).wrap_err("failed to create client metrics")?;

        let connector = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .build();

        let client = HyperClient::builder().build(connector);

        #[cfg(feature = "twitch")]
        let twitch = Self::get_twitch_token(&client, twitch_client_id, twitch_token)
            .await
            .wrap_err("failed to get twitch token")?;

        let ratelimiter = |per_second| {
            LeakyBucket::builder()
                .max(per_second)
                .tokens(per_second)
                .refill_interval(Duration::from_millis(1000 / per_second as u64))
                .refill_amount(1)
                .build()
        };

        let ratelimiters = [
            ratelimiter(2),  // DiscordAttachment
            ratelimiter(2),  // Huismetbenen
            ratelimiter(5),  // MissAnalyzer
            ratelimiter(2),  // Osekai
            ratelimiter(10), // OsuAvatar
            ratelimiter(10), // OsuBadge
            ratelimiter(2),  // OsuMapFile
            ratelimiter(10), // OsuMapsetCover
            LeakyBucket::builder() // OsuReplay, allows 6 per minute
                .max(10)
                .tokens(10)
                .refill_interval(Duration::from_secs(7))
                .refill_amount(1)
                .build(),
            ratelimiter(2), // OsuStats
            ratelimiter(2), // OsuTracker
            ratelimiter(1), // Respektive
            ratelimiter(5), // Twitch
        ];

        Ok(Self {
            client,
            ratelimiters,
            #[cfg(feature = "twitch")]
            twitch,
            metrics,
        })
    }

    pub(crate) async fn ratelimit(&self, site: Site) {
        self.ratelimiters[site as usize].acquire_one().await
    }

    pub(crate) async fn make_get_request(
        &self,
        url: impl AsRef<str>,
        site: Site,
    ) -> Result<Bytes, ClientError> {
        let url = url.as_ref();
        trace!("GET request to url {url}");

        let req = Request::builder()
            .uri(url)
            .method(Method::GET)
            .header(USER_AGENT, MY_USER_AGENT);

        let req = match site {
            #[cfg(not(feature = "twitch"))]
            Site::Twitch => {
                return Err(ClientError::Report(eyre::Report::msg(
                    "twitch request without twitch feature",
                )))
            }
            #[cfg(feature = "twitch")]
            Site::Twitch => req
                .header("Client-ID", self.twitch.client_id.clone())
                .header(
                    http::header::AUTHORIZATION,
                    format!("Bearer {}", self.twitch.oauth_token),
                ),
            _ => req,
        };

        let req = req
            .body(Body::empty())
            .wrap_err("failed to build GET request")?;

        let (response, start) = self
            .send_request(req, site)
            .await
            .wrap_err("failed to receive GET response")?;

        let bytes_res = Self::error_for_status(response, url).await;
        let latency = start.elapsed().as_secs_f64();

        self.metrics
            .response_time
            .with_label_values(&[site.as_str()])
            .observe(latency);

        bytes_res
    }

    pub(crate) async fn make_multipart_post_request(
        &self,
        url: impl AsRef<str>,
        site: Site,
        form: Multipart,
    ) -> Result<Bytes, ClientError> {
        let url = url.as_ref();
        trace!("POST multipart request to url {url}");

        let content_type = format!("multipart/form-data; boundary={}", form.boundary());
        let form = form.finish();

        let req = Request::builder()
            .method(Method::POST)
            .uri(url)
            .header(USER_AGENT, MY_USER_AGENT)
            .header(CONTENT_TYPE, content_type)
            .header(CONTENT_LENGTH, form.len())
            .body(Body::from(form))
            .wrap_err("Failed to build POST request")?;

        self.ratelimit(site).await;

        let (response, start) = self
            .send_request(req, site)
            .await
            .wrap_err("Failed to receive POST multipart response")?;

        let bytes_res = Self::error_for_status(response, url).await;
        let latency = start.elapsed().as_secs_f64();

        self.metrics
            .response_time
            .with_label_values(&[site.as_str()])
            .observe(latency);

        bytes_res
    }

    pub(crate) async fn make_json_post_request(
        &self,
        url: impl AsRef<str>,
        site: Site,
        json: Vec<u8>,
    ) -> Result<Bytes, ClientError> {
        let url = url.as_ref();
        trace!("POST json request to url {url}");

        let req = Request::builder()
            .method(Method::POST)
            .uri(url)
            .header(USER_AGENT, MY_USER_AGENT)
            .header(CONTENT_TYPE, "application/json")
            .header(CONTENT_LENGTH, json.len())
            .body(Body::from(json))
            .wrap_err("Failed to build POST json request")?;

        self.ratelimit(site).await;

        let (response, start) = self
            .send_request(req, site)
            .await
            .wrap_err("Failed to receive POST response")?;

        let bytes_res = Self::error_for_status(response, url).await;
        let latency = start.elapsed().as_secs_f64();

        self.metrics
            .response_time
            .with_label_values(&[site.as_str()])
            .observe(latency);

        bytes_res
    }

    pub(crate) async fn error_for_status(
        response: Response<Body>,
        url: &str,
    ) -> Result<Bytes, ClientError> {
        let status = response.status();

        match status.as_u16() {
            200..=299 => hyper::body::to_bytes(response.into_body())
                .await
                .wrap_err("Failed to extract response bytes")
                .map_err(ClientError::Report),
            400 => Err(ClientError::BadRequest),
            404 => Err(ClientError::NotFound),
            429 => Err(ClientError::Ratelimited),
            _ => Err(eyre!("Failed with status code {status} when requesting url {url}").into()),
        }
    }

    async fn send_request(
        &self,
        req: Request<Body>,
        site: Site,
    ) -> Result<(Response<Body>, Instant), HyperError> {
        self.ratelimit(site).await;

        let start = Instant::now();
        let response_fut = self.client.request(req);

        match response_fut.await {
            Ok(res) => {
                let status = res.status().as_u16().to_string();
                let labels = [site.as_str(), status.as_str()];
                self.metrics.request_count.with_label_values(&labels).inc();

                Ok((res, start))
            }
            Err(err) => {
                let labels = [site.as_str(), INTERNAL_ERROR];
                self.metrics.request_count.with_label_values(&labels).inc();

                Err(err)
            }
        }
    }
}

use std::sync::Arc;
use std::time::Duration;

use futures::Future;
use tokio::timer::Timeout;

use telegram_bot_raw::{HttpRequest, Request, ResponseType};

use crate::connector::{default_connector, Connector};
use crate::errors::Error;
use crate::stream::UpdatesStream;

/// Main type for sending requests to the Telegram bot API.
#[derive(Clone)]
pub struct Api(Arc<ApiInner>);

struct ApiInner {
    token: String,
    connector: Box<dyn Connector>,
}

impl Api {
    ///  create a new `Api` instance.
    ///
    /// # Example
    ///
    /// Using default connector.
    ///
    /// ```rust
    /// use telegram_bot::Api;
    ///
    /// # fn main() {
    /// # let telegram_token = "token";
    /// let api = Api::new(telegram_token);
    /// # }
    /// ```
    pub fn new<T: AsRef<str>>(token: T) -> Api {
        Api(Arc::new(ApiInner {
            token: token.as_ref().to_string(),
            connector: default_connector(),
        }))
    }

    /// Create a stream which produces updates from the Telegram server.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use telegram_bot::Api;
    /// use futures::StreamExt;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let api: Api = Api::new("token");
    ///
    /// let mut stream = api.stream();
    /// let update = stream.next().await;
    ///     println!("{:?}", update);
    /// # }
    /// ```
    pub fn stream(&self) -> UpdatesStream {
        UpdatesStream::new(&self)
    }

    /// Send a request to the Telegram server and wait for a response, timing out after `duration`.
    /// Future will resolve to `None` if timeout fired.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use telegram_bot::{Api, GetMe};
    /// # use std::time::Duration;
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let telegram_token = "token";
    /// # let api = Api::new(telegram_token);
    /// # if false {
    /// let result = api.send_timeout(GetMe, Duration::from_secs(2)).await;
    /// println!("{:?}", result);
    /// # }
    /// # }
    /// ```
    pub fn send_timeout<Req: Request>(
        &self,
        request: Req,
        duration: Duration,
    ) -> impl Future<Output = Result<Option<<Req::Response as ResponseType>::Type>, Error>> + Send
    {
        let api = self.clone();
        let request = request.serialize();
        async move {
            match Timeout::new(api.send_http_request::<Req::Response>(request?), duration).await {
                Err(_) => Ok(None),
                Ok(Ok(result)) => Ok(Some(result)),
                Ok(Err(error)) => Err(error),
            }
        }
    }

    /// Send a request to the Telegram server and wait for a response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use telegram_bot::{Api, GetMe};
    /// #
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let telegram_token = "token";
    /// # let api = Api::new(telegram_token);
    /// # if false {
    /// let result = api.send(GetMe).await;
    /// println!("{:?}", result);
    /// # }
    /// # }
    /// ```
    pub fn send<Req: Request>(
        &self,
        request: Req,
    ) -> impl Future<Output = Result<<Req::Response as ResponseType>::Type, Error>> + Send {
        let api = self.clone();
        let request = request.serialize();
        async move { api.send_http_request::<Req::Response>(request?).await }
    }

    async fn send_http_request<Resp: ResponseType>(
        &self,
        request: HttpRequest,
    ) -> Result<Resp::Type, Error> {
        let http_response = self.0.connector.request(&self.0.token, request).await?;
        let response = Resp::deserialize(http_response)?;
        Ok(response)
    }
}

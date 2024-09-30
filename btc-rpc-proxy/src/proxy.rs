use bytes::Bytes;
use http_body_util::BodyExt;
use http_body_util::Full;

use hyper::body::Incoming;
use hyper::header::{HeaderMap, HeaderValue};
use hyper::http::uri::InvalidUri;
use hyper::{Request, Response, Uri};
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use lazy_static::lazy_static;

#[derive(Debug)]
pub enum ProxyError {
  InvalidUri(InvalidUri),
  HyperError(hyper::Error),
  Forwarding(hyper_util::client::legacy::Error),
}

impl From<hyper_util::client::legacy::Error> for ProxyError {
  fn from(err: hyper_util::client::legacy::Error) -> ProxyError {
    ProxyError::Forwarding(err)
  }
}

impl From<hyper::Error> for ProxyError {
  fn from(err: hyper::Error) -> ProxyError {
    ProxyError::HyperError(err)
  }
}

impl From<InvalidUri> for ProxyError {
  fn from(err: InvalidUri) -> ProxyError {
    ProxyError::InvalidUri(err)
  }
}

fn is_hop_header(name: &str) -> bool {
  use unicase::Ascii;

  // A list of the headers, using `unicase` to help us compare without
  // worrying about the case, and `lazy_static!` to prevent reallocation
  // of the vector.
  lazy_static! {
    static ref HOP_HEADERS: Vec<Ascii<&'static str>> = vec![
      Ascii::new("Connection"),
      Ascii::new("Keep-Alive"),
      Ascii::new("Proxy-Authenticate"),
      Ascii::new("Proxy-Authorization"),
      Ascii::new("Te"),
      Ascii::new("Trailers"),
      Ascii::new("Transfer-Encoding"),
      Ascii::new("Upgrade"),
      Ascii::new("Host"),
    ];
  }

  HOP_HEADERS.iter().any(|h| h == &name)
}

/// Returns a clone of the headers without the [hop-by-hop headers].
///
/// [hop-by-hop headers]: http://www.w3.org/Protocols/rfc2616/rfc2616-sec13.html
fn remove_hop_headers(headers: &HeaderMap<HeaderValue>) -> HeaderMap<HeaderValue> {
  let mut result = HeaderMap::new();
  headers
    .iter()
    .filter(|(k, _)| !is_hop_header(k.as_str()))
    .for_each(|(k, v)| {
      result.insert(k.clone(), v.clone());
    });
  result
}

async fn transform_response(
  mut response: Response<Incoming>,
) -> Result<Response<Full<Bytes>>, ProxyError> {
  *response.headers_mut() = remove_hop_headers(response.headers());
  let (p, b) = response.into_parts();
  let bytes = b.collect().await?.to_bytes();
  let response = Response::from_parts(p, Full::from(bytes));
  Ok(response)
}

fn forward_uri(
  forward_url: &str,
  req: &Request<impl hyper::body::Body>,
) -> Result<Uri, InvalidUri> {
  println!("forward_uri: {} ", forward_url);
  println!("req.uri(): {} ", req.uri());
  println!("req.uri().path(): {} ", req.uri().path());
  println!("req.uri().query() : {:?} ", req.uri().query());
  let forward_uri = match req.uri().query() {
    Some(query) => {
      println!("query: {:?} ", query);
      format!("{}{}?{}", forward_url, req.uri().path(), query)
    }
    // None => format!("{}{}", forward_url, req.uri().path()),\
    None => format!("{}", forward_url),
  };
  let uri = forward_uri.parse::<Uri>();
  println!("forward_uri.parse::<Uri>: {:?} ", uri);
  uri
}

async fn transform_request(
  host: &str,
  mut request: Request<Incoming>,
) -> Result<Request<Full<Bytes>>, ProxyError> {
  *request.headers_mut() = remove_hop_headers(request.headers());
  *request.uri_mut() = forward_uri(host, &request)?;
  let (p, b) = request.into_parts();
  let bytes = b.collect().await?.to_bytes();
  let body = String::from_utf8_lossy(&bytes);
  println!("request.body: {} ", body);

  let request = Request::from_parts(p, Full::from(bytes));
  Ok(request)
}

pub async fn call(
  forward_uri: &str,
  request: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, ProxyError> {
  let proxied_request = transform_request(&forward_uri, request).await?;
  let https = hyper_tls::HttpsConnector::new();
  let client = Client::builder(TokioExecutor::new()).build::<_, Full<Bytes>>(https);
  let response = client.request(proxied_request).await?;
  let proxied_response = transform_response(response).await?;
  Ok(proxied_response)
}

mod proxy;

use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header::RANGE, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::net::TcpListener;

fn debug_request(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
  let body_str = format!("{:?}", req);
  println!("{}", body_str);
  Ok(Response::new(Full::from("No stream found")))
}

fn try_match_range_header(req: &Request<Incoming>) -> Option<(usize, usize)> {
  if let Ok(range_control) = req.headers()[RANGE].to_str() {
    let range = range_control
      .trim_start_matches("bytes=")
      .split('-')
      .collect::<Vec<&str>>();
    let start = range[0].parse::<usize>().ok()?;
    let end = range[1].parse::<usize>().ok()?;
    (end > start).then(|| (start, end))
  } else {
    None
  }
}

async fn forward(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
  println!("{:?}", req);
  if req.uri().path().starts_with("/") {
    let if_range = try_match_range_header(&req);
    match proxy::call("https://go.getblock.io", req).await {
      Ok(response) => match if_range {
        Some((start, end)) => {
          let body = response
            .collect()
            .await?
            .to_bytes()
            .iter()
            .copied()
            .collect::<Vec<u8>>();
          if body.len() < end - start + 1 || body.len() < end {
            Ok(
              Response::builder()
                .status(StatusCode::OK)
                .body(Full::from(body))
                .unwrap(),
            )
          } else {
            let partial = body[start..end].to_vec();
            Ok(
              Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(
                  "Content-Range",
                  format!("bytes {}-{}/{}", start, end, body.len()),
                )
                .body(Full::from(Bytes::from(partial)))
                .unwrap(),
            )
          }
        }
        None => Ok(response),
      },
      Err(error) => {
        println!("{:?}", error);
        Ok(
          Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::from(Bytes::from("Internal Server Error")))
            .unwrap(),
        )
      }
    }
  } else {
    debug_request(req)
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  env_logger::init();
  let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
  let listener = TcpListener::bind(addr).await?;
  loop {
    let (stream, _) = listener.accept().await?;
    let io = TokioIo::new(stream);
    tokio::task::spawn(async move {
      if let Err(err) = http1::Builder::new()
        .serve_connection(io, service_fn(forward))
        .await
      {
        eprintln!("Error serving connection: {:?}", err);
      }
    });
  }
}

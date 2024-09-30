mod cache;
mod cli;
mod proxy;

use cache::LruCache;
use clap::Parser;

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{header::RANGE, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, Notify};

pub const IDEMPOTENCY_KEY: &str = "X-Idempotency";
pub const FORWARD_SOLANA_RPC: &str = "X-Forward-Solana";

fn try_match_range_header(req: &Request<Incoming>) -> Option<(usize, usize)> {
  if let Some(range_control) = req.headers().get(RANGE).map(|v| v.to_str().ok()).flatten() {
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

fn try_match_cache_header(req: &Request<Incoming>, h_key: &str) -> Option<String> {
  req
    .headers()
    .get(h_key)
    .map(|v| v.to_str().ok())
    .flatten()
    .map(|key| key.to_string())
}

async fn forward(
  target: impl AsRef<str>,
  req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
  let if_range = try_match_range_header(&req);
  match proxy::call(target.as_ref(), req).await {
    Ok(response) => match if_range {
      Some((start, end)) => {
        let body = response
          .collect()
          .await?
          .to_bytes()
          .iter()
          .copied()
          .collect::<Vec<u8>>();
        if body.len() <= end - start {
          Ok(
            Response::builder()
              .status(StatusCode::OK)
              .body(Full::from(body))
              .unwrap(),
          )
        } else {
          let partial = if end >= body.len() {
            body[start..].to_vec()
          } else {
            body[start..=end].to_vec()
          };
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let args = cli::Cli::parse();
  let addr = SocketAddr::from((args.run.addr, args.run.port));
  let target = args.run.forward;
  let listener = TcpListener::bind(addr).await?;
  let req_cache = Arc::new(cache::MemoryCache::<String>::new(1000));
  let resp_cache = Arc::new(cache::MemoryCache::<Response<Full<Bytes>>>::new(1000));
  let notify_map = Arc::new(Mutex::new(HashMap::<String, Arc<Notify>>::new()));

  loop {
    let req_cache = req_cache.clone();
    let resp_cache = resp_cache.clone();
    let notify_map = notify_map.clone();
    let default_target = target.clone();
    let (stream, _) = listener.accept().await?;
    let io = TokioIo::new(stream);
    tokio::task::spawn(async move {
      let f = |req| async {
        println!("Received request: {:#?}", req);
        let key = try_match_cache_header(&req, IDEMPOTENCY_KEY);
        if let Some(key) = key {
          // first check resp cache,if find existed resp ,return it
          if let Some(response) = resp_cache.get(&key).await {
            println!("find response from cache: {} -> {:#?}", key, response);
            return Ok(response);
          } else {
            // if not existed resp ,check the req forwarded?
            if req_cache.get(&key).await.is_some() {
              // already exist req,just waiting
              let notify = {
                let mut notify_map = notify_map.lock().await;
                notify_map
                  .entry(key.clone())
                  .or_insert_with(|| Arc::new(Notify::new()))
                  .clone()
              };
              notify.notified().await;
              // weak up and get resp
              if let Some(response) = resp_cache.get(&key).await {
                println!("waited response: {} -> {:#?}", key, response);
                return Ok(response);
              } else {
                eprintln!("Cache inconsistency for key {}", key);
                return Err("Cache inconsistency".into());
              }
            } else {
              // new req, need to forward it
              let req_content = format!("{:?}", req);
              // update req cache for the key
              req_cache.put(key.clone(), req_content.clone()).await;
              println!(
                "forward new request for X-Idempotency: {} -> {:#?}",
                key, req_content
              );

              let forward_rpc = try_match_cache_header(&req, FORWARD_SOLANA_RPC)
                .unwrap_or(default_target.to_string());
              println!("forward url: {}", forward_rpc);
              let rsp = forward(&forward_rpc, req).await;
              match rsp {
                Ok(response) => {
                  println!(
                    "Received response for X-Idempotency: {} -> {:#?}",
                    key, response
                  );
                  resp_cache.put(key.clone(), response.clone()).await;

                  // notify all the waiters
                  if let Some(notify) = notify_map.lock().await.remove(&key) {
                    println!("Notify all the waiters for X-Idempotency request: {} ", key);
                    notify.notify_waiters();
                  }

                  Ok(response)
                }
                Err(err) => {
                  let e = format!("{}", err);
                  eprintln!("Error forwarding request: {}", e);
                  Err(e)
                }
              }
            }
          }
        } else {
          // directly forward
          println!("without X-Idempotency just forword req ...");
          forward(&default_target, req)
            .await
            .map_err(|err| format!("{}", err))
        }
      };

      if let Err(err) = http1::Builder::new()
        .serve_connection(io, service_fn(f))
        .await
      {
        eprintln!("Error serving connection: {:?}", err);
      }
    });
  }
}

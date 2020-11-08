#![deny(warnings)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use std::sync;
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::sync::atomic::AtomicUsize;

use hyper::{Body, http, Method, Request, Response, Server, StatusCode};
use hyper::body::Bytes;
use hyper::http::header;
use hyper::service::{make_service_fn, service_fn};
use serde::{Deserialize, Serialize};
use tokio::future;
use url::Url;
use std::borrow::BorrowMut;

#[derive(Serialize, Deserialize, Debug)]
enum StateName {
    SelectCardsState,
}

#[derive(Serialize, Deserialize, Debug)]
struct GameTable {
    enemy_row_2: [i64; 5],
    enemy_row_1: [i64; 5],
    territory_row: [i64; 5],
    player_row_1: [i64; 5],
    player_row_2: [i64; 5],
}

#[derive(Serialize, Deserialize, Debug)]
struct GameState {
    table: GameTable,
    hand: Vec<i64>,
    state: StateName,
    discarded_cards: Vec<i64>,
}

type SyncState = Arc<RwLock<GameState>>;

fn make_state() -> GameState {
    GameState {
        table: GameTable {
            enemy_row_2: [1, 0, 0, 1, 0],
            enemy_row_1: [1, 0, 0, 1, 0],
            territory_row: [0, 0, 0, 0, 0],
            player_row_1: [3, 1, 2, 1, 2],
            player_row_2: [1, -1, 3, 1, 3],
        },
        hand: vec![3, 2, 2],
        state: StateName::SelectCardsState,
        discarded_cards: vec![],
    }
}


/// This is our service handler. It receives a Request, routes on its
/// path, and returns a Future of a Response.
async fn echo(req: Request<Body>, db: SyncState) -> Result<Response<Body>, hyper::Error> {
    let slice = "Try POSTing data to /echo such as: `curl localhost:3000/echo -XPOST -d 'hello world'`";
    match (req.method(), req.uri().path()) {
        // Serve some instructions at /
        (&Method::GET, "/") => Ok(Response::new(Body::from(
            slice,
        ))),

        (&Method::GET, "/ololo") => Ok(process_ololo(req, db).unwrap()),

        // Simply echo the body back to the client.
        (&Method::POST, "/echo") => {
            upd_state(db);
            Ok(Response::new(req.into_body()))
        }

        // Reverse the entire body before sending back to the client.
        //
        // Since we don't know the end yet, we can't simply stream
        // the chunks as they arrive as we did with the above uppercase endpoint.
        // So here we do `.await` on the future, waiting on concatenating the full body,
        // then afterwards the content can be reversed. Only then can we return a `Response`.
        (&Method::POST, "/echo/reversed") => {
            let whole_body = hyper::body::to_bytes(req.into_body()).await?;

            let reversed_body = process_bytes(whole_body);

            Ok(Response::new(Body::from(reversed_body)))
        }

        // Return the 404 Not Found for other routes.
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}


fn upd_state(state: SyncState) {
    let mut x = state.write().unwrap();
    x.hand = vec![1, 2, 3, 4, 5];
}

fn process_ololo(request: Request<Body>, db: SyncState) -> http::Result<Response<Body>> {
    // let dbx = *db.lock().unwrap();
    let jj = serde_json::to_string(&db).unwrap();
    Response::builder()
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(Body::from(jj))
}

// fn process_ololo() -> Result<Response<Body>, hyper::Error> {
// let uri_string = req.uri().query().unwrap();
// let request_url = Url::parse(&format!("http://a.ru?{}", uri_string)).unwrap();
// let params = request_url
//     .query_pairs()
//     .collect::<Vec<_>>();


// Ok(Response::new(Body::from(
//     // format!("{:?}", jj),
//     jj
// )))
//     let jj: String = serde_json::to_string(&make_state()).unwrap();
//     Response::builder()
//         .status(StatusCode::NOT_FOUND)
//         .header("Access-Control-Allow-Origin", "*")
//         .header("Content-Type", "application/json; charset=utf-8")
//         .body(Body::from(jj))
// }

fn process_bytes(chunk: Bytes) -> Vec<u8> {
    chunk
        .iter()
        .map(|byte| byte.to_ascii_uppercase())
        .collect::<Vec<u8>>()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // let db = Box::leak(Box::new(make_state())) as &'static GameState;

    let closure = Arc::new(RwLock::new(make_state()));

    // let tmp = make_state();
    // let xyz = upd_state(tmp);
    // println!("----");
    // println!("{:?}", xyz);
    // println!("----");

    let addr = ([127, 0, 0, 1], 3000).into();

    let service = make_service_fn(move |_| {
        let closure = closure.clone();
        async {
            Ok::<_, hyper::Error>(service_fn(move |request| {
                // Clone again to ensure that client outlives this closure.
                echo(request, closure.to_owned())
            }))
        }
    });

    let server = Server::bind(&addr).serve(service);

    println!("Listening on http://{}", addr);

    server.await?;

    Ok(())
}
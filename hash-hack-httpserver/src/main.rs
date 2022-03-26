use std::net::SocketAddr;

use warp::{ Filter, hyper::StatusCode, };
use serde_derive::Serialize;
use tracing::{self, Level};
use tracing_subscriber::{
    FmtSubscriber,
    fmt::format::FmtSpan
};

use hash_hack_dbms::query::query_bili2;

use clap:: Parser;


#[derive(Parser)]
#[clap(author, about)]
struct Cli {
    #[clap(short='p', default_value_t = 6067)]
    port: u16
}

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}



#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let port = cli.port;

    /* Set Logger */
    let subscriber = FmtSubscriber::builder()
    .with_max_level(Level::TRACE)
    .with_span_events(FmtSpan::CLOSE)
    .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    /* Set Routes */
    /* Bili2 */
    let bili2
    = warp::path("bili2")
        .and(
            warp::path!("hashuid" / String)
            .and(warp::get())
            .map(|raw: String| {
                let id = match u32::from_str_radix(&raw, 16) {
                    Ok(id) => id,
                    Err(_) => {
                        return warp::reply::json(&ErrorMessage {
                            code: StatusCode::BAD_REQUEST.as_u16(),
                            message: "hex u32 required!".to_string()
                        })
                    },
                };

                match query_bili2(id) {
                    Ok(res) => {
                        warp::reply::json(&res)
                    },
                    Err(_) => {
                        tracing::error!("[HashUID] Read DB Failed");

                        warp::reply::json(&ErrorMessage {
                            code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                            message: "read db failed".to_string()
                        })
                    }
                }
            })
    );

    let routers
    = bili2;

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    warp::serve(routers).run(addr).await;

}

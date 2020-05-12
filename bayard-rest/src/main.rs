#[macro_use]
extern crate clap;

use clap::{App, AppSettings, Arg};
use crossbeam_channel::select;
use log::*;

use bayard_common::log::set_logger;
use bayard_common::signal::sigterm_channel;
use bayard_rest::rest::server::RestServer;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    set_logger();

    let threads = format!("{}", num_cpus::get().to_owned());

    let app = App::new(crate_name!())
        .setting(AppSettings::DeriveDisplayOrder)
        .version(crate_version!())
        .author(crate_authors!())
        .about("Bayard REST server")
        .help_message("Prints help information.")
        .version_message("Prints version information.")
        .version_short("v")
        .arg(
            Arg::with_name("HOST")
                .help("Node address.")
                .short("H")
                .long("host")
                .value_name("HOST")
                .default_value("0.0.0.0")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("PORT")
                .help("HTTP service port number.")
                .short("p")
                .long("port")
                .value_name("PORT")
                .default_value("8000")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("SERVER")
                .help("Index service address.")
                .short("s")
                .long("server")
                .value_name("IP:PORT")
                .default_value("0.0.0.0:5000")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("HTTP_WORKER_THREADS")
                .help("Number of HTTP worker threads. By default http server uses number of available logical cpu as threads count.")
                .short("w")
                .long("http-worker-threads")
                .value_name("HTTP_WORKER_THREADS")
                .default_value(&threads)
                .takes_value(true),
        );

    let matches = app.get_matches();

    let host = matches.value_of("HOST").unwrap();
    let port = matches.value_of("PORT").unwrap().parse::<u16>().unwrap();
    let server = matches.value_of("SERVER").unwrap();
    let http_worker_threads = matches
        .value_of("HTTP_WORKER_THREADS")
        .unwrap()
        .parse::<usize>()
        .unwrap();

    let rest_address = format!("{}:{}", host, port);

    let mut rest_server = RestServer::new(rest_address.as_str(), server, http_worker_threads);
    info!("start rest service on {}", rest_address.as_str());

    // Wait for signals for termination (SIGINT, SIGTERM).
    let sigterm_receiver = sigterm_channel().unwrap();
    loop {
        select! {
            recv(sigterm_receiver) -> _ => {
                info!("receive signal");
                break;
            }
        }
    }

    match rest_server.shutdown().await {
        Ok(_) => {
            info!("stop rest service on {}:{}", host, port);
            Ok(())
        }
        Err(e) => {
            error!(
                "failed to stop rest service on {}:{}: error={}",
                host, port, e
            );
            Err(e)
        }
    }
}

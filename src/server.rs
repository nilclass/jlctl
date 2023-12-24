use crate::{device::Device, netlist::NodeFile};
use actix_web::{
    delete, get, middleware::Logger, post, put, web, App, HttpServer, Responder, ResponseError,
    Result,
};
use env_logger::Env;
use log::info;
use std::sync::{Arc, Mutex};

struct Shared {
    device: Arc<Mutex<Device>>,
}

#[derive(Debug)]
struct Error(anyhow::Error);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResponseError for Error {}

#[get("/netlist")]
async fn netlist(shared: web::Data<Shared>) -> Result<impl Responder> {
    let netlist = shared.device.lock().unwrap().netlist().map_err(Error)?;
    Ok(web::Json(netlist))
}

#[get("/bridges")]
async fn bridges(shared: web::Data<Shared>) -> Result<impl Responder> {
    let nodefile: NodeFile = shared
        .device
        .lock()
        .unwrap()
        .netlist()
        .map_err(Error)?
        .into();

    Ok(web::Json(nodefile))
}

#[put("/bridges")]
async fn add_bridges(
    shared: web::Data<Shared>,
    json: web::Json<NodeFile>,
) -> Result<impl Responder> {
    let mut device = shared.device.lock().unwrap();
    let mut nodefile: NodeFile = device.netlist().map_err(Error)?.into();

    nodefile.add_from(json.0);
    device.send_nodefile(&nodefile).map_err(Error)?;

    Ok(web::Json(nodefile))
}

#[delete("/bridges")]
async fn remove_bridges(
    shared: web::Data<Shared>,
    json: web::Json<NodeFile>,
) -> Result<impl Responder> {
    let mut device = shared.device.lock().unwrap();
    let mut nodefile: NodeFile = device.netlist().map_err(Error)?.into();

    nodefile.remove_from(json.0);
    device.send_nodefile(&nodefile).map_err(Error)?;

    Ok(web::Json(nodefile))
}

#[post("/bridges/clear")]
async fn clear_bridges(shared: web::Data<Shared>) -> Result<impl Responder> {
    shared
        .device
        .lock()
        .unwrap()
        .clear_nodefile()
        .map_err(Error)?;
    Ok(web::Json(true))
}

#[actix_web::main]
pub async fn start(device: Device, listen_address: &str) -> std::io::Result<()> {
    let device = Arc::new(Mutex::new(device));

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    info!("Starting HTTP server, listening on {:?}", listen_address);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(Shared {
                device: Arc::clone(&device),
            }))
            .service(netlist)
            .service(bridges)
            .service(add_bridges)
            .service(remove_bridges)
            .service(clear_bridges)
    })
    .workers(2)
    .bind(listen_address)?
    .run()
    .await
}

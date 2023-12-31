use crate::{device_manager::DeviceManager, netlist::NodeFile};
use actix_cors::Cors;
use actix_web::{
    delete, get, http, middleware::Logger, post, put, web, App, HttpResponse, HttpServer,
    Responder, ResponseError, Result,
};
use log::info;
use serde_json::json;
use std::sync::{Arc, Mutex};

struct Shared {
    device_manager: Arc<Mutex<DeviceManager>>,
}

#[derive(Debug)]
struct Error(anyhow::Error);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::BadGateway().json(json!({ "error": self.0.to_string() }))
    }
}

#[get("/netlist")]
async fn netlist(shared: web::Data<Shared>) -> Result<impl Responder> {
    let netlist = shared
        .device_manager
        .lock()
        .unwrap()
        .with_device(|device| device.netlist())
        .map_err(Error)?;

    Ok(web::Json(netlist))
}

#[get("/bridges")]
async fn bridges(shared: web::Data<Shared>) -> Result<impl Responder> {
    let nodefile: NodeFile = shared
        .device_manager
        .lock()
        .unwrap()
        .with_device(|device| device.netlist())
        .map_err(Error)?
        .into();

    Ok(web::Json(nodefile))
}

#[put("/bridges")]
async fn add_bridges(
    shared: web::Data<Shared>,
    json: web::Json<NodeFile>,
) -> Result<impl Responder> {
    let nodefile = shared
        .device_manager
        .lock()
        .unwrap()
        .with_device(|device| {
            let mut nodefile: NodeFile = device.netlist()?.into();
            nodefile.add_from(json.0);
            device.send_nodefile(&nodefile)?;
            Ok(nodefile)
        })
        .map_err(Error)?;

    Ok(web::Json(nodefile))
}

#[delete("/bridges")]
async fn remove_bridges(
    shared: web::Data<Shared>,
    json: web::Json<NodeFile>,
) -> Result<impl Responder> {
    let nodefile = shared
        .device_manager
        .lock()
        .unwrap()
        .with_device(|device| {
            let mut nodefile: NodeFile = device.netlist()?.into();
            nodefile.remove_from(json.0);
            device.send_nodefile(&nodefile)?;
            Ok(nodefile)
        })
        .map_err(Error)?;

    Ok(web::Json(nodefile))
}

#[post("/bridges/clear")]
async fn clear_bridges(shared: web::Data<Shared>) -> Result<impl Responder> {
    shared
        .device_manager
        .lock()
        .unwrap()
        .with_device(|device| device.clear_nodefile())
        .map_err(Error)?;

    Ok(web::Json(true))
}

#[actix_web::main]
pub async fn start(device_manager: DeviceManager, listen_address: &str) -> std::io::Result<()> {
    let device_manager = Arc::new(Mutex::new(device_manager));

    info!("Starting HTTP server, listening on {:?}", listen_address);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin_fn(|origin, _req_head| {
                origin.as_bytes().starts_with(b"http://localhost:")
            })
            .allowed_methods(vec!["GET", "PUT", "POST", "DELETE"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600);

        App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .app_data(web::Data::new(Shared {
                device_manager: Arc::clone(&device_manager),
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

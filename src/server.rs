use crate::{
    device_manager::DeviceManager,
    types::{Net, SupplySwitchPos},
    validate,
};
use actix_cors::Cors;
use actix_web::{
    get, http, middleware::{Logger, NormalizePath}, post, put, web, App, HttpResponse, HttpServer, Responder,
    ResponseError, Result,
};
use log::info;
use serde_json::json;
use std::{sync::{Arc, Mutex}};
use std::net::TcpListener;

#[cfg(feature = "jumperlab")]
mod jumperlab;

struct Shared {
    device_manager: Arc<Mutex<DeviceManager>>,
}

impl Shared {
    fn netlist(&self) -> Result<Vec<Net>> {
        Ok(self
            .device_manager
            .lock()
            .unwrap()
            .with_device(|device| device.netlist())
            .map_err(Error)?)
    }
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

#[get("/status")]
async fn get_status(shared: web::Data<Shared>) -> Result<impl Responder> {
    let status = shared
        .device_manager
        .lock()
        .unwrap()
        .status()
        .map_err(Error)?;
    Ok(web::Json(status))
}

#[get("/nets")]
async fn get_nets(shared: web::Data<Shared>) -> Result<impl Responder> {
    Ok(web::Json(shared.netlist()?))
}

#[put("/nets")]
async fn put_nets(shared: web::Data<Shared>, json: web::Json<Vec<Net>>) -> Result<impl Responder> {
    let netlist = shared
        .device_manager
        .lock()
        .unwrap()
        .with_device(|device| {
            device.set_netlist(validate::netlist(json.into_inner())?)?;
            device.netlist()
        })
        .map_err(Error)?;

    Ok(web::Json(netlist))
}

#[get("/nets/{index}")]
async fn get_net(path: web::Path<u8>, shared: web::Data<Shared>) -> Result<impl Responder> {
    let index = path.into_inner();
    Ok(web::Json(
        shared.netlist()?.into_iter().find(|net| net.index == index),
    ))
}

#[get("/supply_switch_pos")]
async fn get_supply_switch_pos(shared: web::Data<Shared>) -> Result<impl Responder> {
    let pos = shared
        .device_manager
        .lock()
        .unwrap()
        .with_device(|device| device.supply_switch())
        .map_err(Error)?;
    Ok(web::Json(pos.to_string()))
}

#[put("/supply_switch_pos/{pos}")]
async fn set_supply_switch_pos(
    path: web::Path<String>,
    shared: web::Data<Shared>,
) -> Result<impl Responder> {
    let pos: SupplySwitchPos = path.into_inner().parse().map_err(Error)?;
    shared
        .device_manager
        .lock()
        .unwrap()
        .with_device(|device| device.set_supply_switch(pos))
        .map_err(Error)?;
    Ok(web::Json(pos.to_string()))
}

// #[get("/bridges")]
// async fn bridges(shared: web::Data<Shared>) -> Result<impl Responder> {
//     let nodefile: NodeFile = shared
//         .device_manager
//         .lock()
//         .unwrap()
//         .with_device(|device| device.netlist())
//         .map_err(Error)?
//         .into();

//     Ok(web::Json(nodefile))
// }

// #[put("/bridges")]
// async fn add_bridges(
//     shared: web::Data<Shared>,
//     json: web::Json<NodeFile>,
// ) -> Result<impl Responder> {
//     let nodefile = shared
//         .device_manager
//         .lock()
//         .unwrap()
//         .with_device(|device| {
//             let mut nodefile: NodeFile = device.netlist()?.into();
//             nodefile.add_from(json.0);
//             device.send_nodefile(&nodefile)?;
//             Ok(nodefile)
//         })
//         .map_err(Error)?;

//     Ok(web::Json(nodefile))
// }

// #[delete("/bridges")]
// async fn remove_bridges(
//     shared: web::Data<Shared>,
//     json: web::Json<NodeFile>,
// ) -> Result<impl Responder> {
//     let nodefile = shared
//         .device_manager
//         .lock()
//         .unwrap()
//         .with_device(|device| {
//             let mut nodefile: NodeFile = device.netlist()?.into();
//             nodefile.remove_from(json.0);
//             device.send_nodefile(&nodefile)?;
//             Ok(nodefile)
//         })
//         .map_err(Error)?;

//     Ok(web::Json(nodefile))
// }

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

pub fn start(device_manager: DeviceManager, listen_address: Option<&str>) -> std::io::Result<String> {
    let listener = TcpListener::bind(listen_address.unwrap_or("localhost:0"))?;
    let address = listener.local_addr()?.to_string();
    start_with_listener(device_manager, listener)?;
    Ok(address)
}

#[actix_web::main]
async fn start_with_listener(device_manager: DeviceManager, listener: TcpListener) -> std::io::Result<()> {
    let device_manager = Arc::new(Mutex::new(device_manager));

    let address = listener.local_addr()?;
    let ip = address.ip();
    let listen_address = format!("{}:{}", if ip.is_loopback() { "localhost".to_string() } else { ip.to_string() }, address.port());
    info!("Starting HTTP server, listening on {}", listen_address);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin_fn(|origin, _req_head| {
                origin.as_bytes().starts_with(b"http://localhost:")
            })
            .allowed_methods(vec!["GET", "PUT", "POST", "DELETE"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600);

        let app = App::new()
            .wrap(Logger::default())
            .wrap(cors)
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(Shared {
                device_manager: Arc::clone(&device_manager),
            }))
            .service(get_status)
            .service(get_net)
            .service(get_nets)
            .service(put_nets)
            .service(set_supply_switch_pos)
            .service(get_supply_switch_pos)
            .service(clear_bridges);

        #[cfg(feature = "jumperlab")]
        {
            println!("\n    To open Jumperlab, visit: http://{}/jumperlab\n", listen_address);
            return jumperlab::add_to_app(app);
        }

        #[allow(unreachable_code)]
        app
    })
    .workers(1)
    .listen(listener)?
    .run()
    .await
}

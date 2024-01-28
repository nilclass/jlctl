use actix_web::{
    dev::{ServiceFactory, ServiceRequest},
    get,
    web::{self, Data},
    App, HttpResponse, Responder, Result,
};
use std::io::{Cursor, Read};
use std::sync::{Arc, Mutex};
use zip::read::ZipArchive;

// Embed ZIP archive containing Jumperlab's assets
const ZIP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/jumperlab.zip"));

// Install `/jumperlab/*` route
pub fn add_to_app<T>(app: App<T>) -> App<T>
where
    T: ServiceFactory<ServiceRequest, Config = (), Error = actix_web::Error, InitError = ()>,
{
    let archive = Arc::new(Mutex::new(ZipArchive::new(Cursor::new(ZIP)).unwrap()));
    app.app_data(Data::new(archive))
        .service(handle_index)
        .service(handle_file)
}

#[get("/jumperlab")]
async fn handle_index(
    data: web::Data<Arc<Mutex<ZipArchive<Cursor<&[u8]>>>>>,
) -> Result<impl Responder> {
    serve_path("index.html", data)
}

#[get("/jumperlab/{path}*")]
async fn handle_file(
    path: web::Path<String>,
    data: web::Data<Arc<Mutex<ZipArchive<Cursor<&[u8]>>>>>,
) -> Result<impl Responder> {
    serve_path(path.as_str(), data)
}

fn serve_path(
    path: &str,
    data: web::Data<Arc<Mutex<ZipArchive<Cursor<&[u8]>>>>>,
) -> Result<impl Responder> {
    let archive = data.into_inner();
    let archive = &mut archive.lock().unwrap();
    let response = if let Ok(entry) = archive.by_name(path).as_mut() {
        let content_type = mime_guess::from_path(path)
            .first()
            .unwrap_or(mime_guess::mime::APPLICATION_OCTET_STREAM)
            .to_string();
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        HttpResponse::Ok().content_type(content_type).body(buf)
    } else {
        HttpResponse::NotFound().body("Not found")
    };
    Ok(response)
}

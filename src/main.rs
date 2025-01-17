mod config;
mod database;
mod error;
mod middleware;
mod models;
mod parser;
mod routes;
mod static_files;

#[cfg(all(target_env = "musl", target_pointer_width = "64"))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{
    cookie::SameSite,
    middleware::{Compress, Condition},
    web, App, HttpServer, ResponseError,
};
use database::Database;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let data = Database::load_or_create().expect("Failed to load");

    let config = config::config();
    let (cookie_key, secure_cookies, disable_registration, units) = (
        config.get_cookie_key(),
        config.secure_cookies,
        config.disable_registration,
        config.get_units(),
    );

    println!("Running at {}:{}", config.address, config.port);

    HttpServer::new(move || {
        App::new()
            .data(data.clone())
            .data(units.clone())
            .wrap(Compress::default())
            .wrap(Condition::new(
                disable_registration,
                middleware::DisableRegistration::default(),
            ))
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&cookie_key)
                    .name("tf-viewer")
                    .http_only(true)
                    .secure(secure_cookies)
                    .same_site(SameSite::Strict),
            ))
            .default_service(web::route().to(|| {
                error::Error::BadRequest(error::ErrorKind::NotFound, "Page not found")
                    .error_response()
            }))
            .configure(error::config)
            .configure(static_files::config)
            .configure(routes::authentication::config)
            .service(
                web::scope("")
                    .wrap(middleware::CheckLogin::new(
                        middleware::AuthType::Restricted,
                    ))
                    .configure(routes::index::config)
                    .configure(routes::upload::config)
                    .service(
                        web::scope("user")
                            .configure(routes::activity::config)
                            .configure(routes::user::config)
                            .configure(routes::gear::config),
                    ),
            )
    })
    .bind((config.address, config.port))?
    .run()
    .await
}

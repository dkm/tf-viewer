use askama_actix::{Template, TemplateIntoResponse};
use actix_web::{Responder, HttpRequest, get, HttpResponse, web};
use actix_identity::Identity;
use actix_multipart::{Multipart, Field};
use futures::{StreamExt, TryStreamExt};

use super::UrlFor;

#[derive(Template)]
#[template(path = "upload.html")]
struct UploadTemplate<'a> {
    url: UrlFor,
    id: Identity,
    title: &'a str,
}

pub async fn upload(id: Identity, req: HttpRequest) -> impl Responder {
    UploadTemplate {
        url: UrlFor::new(&id, req),
        id: id,
        title: "Upload",
    }.into_response()
}

pub async fn upload_post(
    req: HttpRequest,
    data: web::Data<crate::Database>,
    id: Identity,
    mut payload: Multipart
    ) -> impl Responder {

    let mut f: Vec<u8> = Vec::new();

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field
            .content_disposition()
            .ok_or_else(|| actix_web::error::ParseError::Incomplete).expect("Fail");

            
        while let Some(chunk) = field.next().await {
            let chunk = chunk.unwrap();
            f.extend_from_slice(chunk.as_ref());
        }
    }

    let gear = data.as_ref().users.get(id.identity().unwrap().as_str()).unwrap().standard_gear;
    let parsed = web::block(move || crate::parser::parse(&f, &gear)).await;

    if let Ok(x) = parsed {
        {
            let record = x.record.clone();
            let id = x.id.clone();
            std::thread::spawn(move || super::utils::generate_thumb(record, &id));
        }

        data.as_ref().activities.insert(x, id.identity().unwrap().as_str());
        HttpResponse::Ok().finish().into_body()
    }
    else {
        HttpResponse::BadRequest().finish().into_body()
    }
}

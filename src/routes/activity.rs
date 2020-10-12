use actix_web::{get, Responder, web, HttpRequest};
use actix_identity::Identity;
use askama_actix::{Template, TemplateIntoResponse};
use serde::{Serialize, Deserialize};
use super::{ErrorTemplate, UrlFor, date_format};

#[derive(Template)]
#[template(path = "activity/activity.html")]
struct ActivityTemplate<'a> {
    url: UrlFor,
    id: Identity,
    user: &'a str,
    activity: crate::Activity,
    title: &'a str,
}

pub async fn activity(
    req: HttpRequest,
    data: web::Data<crate::Database>,
    id: Identity,
    web::Path((user, activity_id)): web::Path<(String, String)>
    ) -> impl Responder {


    if !data.as_ref().activities.exists(&user, &activity_id).unwrap() {
        return ErrorTemplate {
            url: UrlFor::new(&id, req),
            id: id,
            title: "Error",
        }.into_response()
    }

    let activity = data.as_ref().activities.get_activity(&user, &activity_id).unwrap();

    ActivityTemplate {
        url: UrlFor::new(&id, req),
        id: id,
        user: &user,
        activity: activity,
        title: "Activity",
    }.into_response()
}


#[derive(Template)]
#[template(path = "activity/activityindex.html")]
struct ActivityIndexTemplate<'a> {
    url: UrlFor,
    id: Identity,
    user: &'a str,
    title: &'a str,
}

pub async fn activityindex(
    req: HttpRequest,
    data: web::Data<crate::Database>,
    id: Identity,
    user: web::Path<String>
    ) -> impl Responder {

    ActivityIndexTemplate {
        url: UrlFor::new(&id, req),
        id: id,
        user: &user,
        title: "Activities",
    }.into_response()
}

#[derive(Deserialize)]
pub struct DataRequest {
    pub draw: usize,
    pub start: usize,
    pub length: usize,
    pub column: usize,
    pub dir: String,
}

#[derive(Serialize, Debug)]
struct DataResponse {
    draw: usize,
    recordsTotal: usize,
    recordsFiltered: usize,
    data: Vec<Data>,
}

#[derive(Serialize, Debug)]
struct Data {
    #[serde(with = "date_format")]
    date: chrono::DateTime<chrono::Local>,
    activity_type: String,
    duration: Option<f64>,
    distance: Option<f64>,
    calories: u16,
    cadence_avg: Option<u8>,
    heartrate_avg: Option<u8>,
    heartrate_max: Option<u8>,
    speed_avg: Option<f64>,
    speed_max: Option<f64>,
    ascent: Option<u16>,
    descent: Option<u16>,
    id: String,
}

pub async fn activityindex_post(
    request: web::Json<DataRequest>,
    data: web::Data<crate::Database>,
    user: web::Path<String>
    ) -> impl Responder {

    let user1 = user.to_owned();
    let data1 = data.to_owned();

    let iter = web::block(move || data.as_ref().activities.iter(&user.to_owned()))
        .await
        .unwrap();

    let mut id = web::block(move || data1.as_ref().activities.iter_id(&user1)).await.unwrap();
   
    let mut sessions: Vec<Data> = iter
        .zip(id)
        .map(|(x,y)| -> Data {  Data {
            date: x.start_time.0,
            activity_type: x.activity_type,
            duration: x.duration_active,
            distance: x.distance,
            calories: x.calories,
            cadence_avg: x.cadence_avg,
            heartrate_avg: x.heartrate_avg,
            heartrate_max: x.heartrate_max,
            speed_avg: x.speed_avg,
            speed_max: x.speed_max,
            ascent: x.ascent,
            descent: x.descent,
            id: y,
        }})
        .collect::<Vec<Data>>();

    let amount = sessions.len();

    match request.column {
        0 => sessions.sort_by_key(|k| k.date),
        2 => sessions.sort_by(|a, b| a.duration.partial_cmp(&b.duration).unwrap()),
        3 => sessions.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap()),
        4 => sessions.sort_by_key(|k| k.calories),
        5 => sessions.sort_by_key(|k| k.cadence_avg),
        6 => sessions.sort_by_key(|k| k.heartrate_avg),
        7 => sessions.sort_by_key(|k| k.heartrate_max),
        8 => sessions.sort_by(|a, b| a.speed_avg.partial_cmp(&b.speed_avg).unwrap()),
        9 => sessions.sort_by(|a, b| a.speed_max.partial_cmp(&b.speed_max).unwrap()),
        10 => sessions.sort_by_key(|k| k.ascent),
        11 => sessions.sort_by_key(|k| k.descent),
        _ => (),
    };

    if request.dir.as_str() == "asc" {
        sessions.reverse();
    }

    let results: Vec<Data> = sessions
        .into_iter()
        .skip(request.start)
        .take(request.length)
        .collect();


    web::Json(
        DataResponse {
            draw: request.draw,
            recordsTotal: amount,
            recordsFiltered: amount,
            data: results,
        }
    )
}
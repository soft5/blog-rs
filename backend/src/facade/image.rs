use core::{result::Result};

use bytes::Buf;
use hyper::header::{self, HeaderMap, HeaderValue};
use warp::{
    filters::multipart::FormData,
    filters::path::Tail,
    http::{response::Response},
    reply::{Response as WarpResponse},
    Rejection, Reply,
};

use blog_common::{
    dto::{user::UserInfo},
    result::{Error},
};

use crate::{
    db::{post, user},
    facade::{session_id_cookie, wrap_json_data, wrap_json_err},
    service::{self, status},
    util::{
        common,
    },
};

pub async fn verify_image(token: Option<String>) -> Result<WarpResponse, Rejection> {
    let token = token.unwrap_or(common::simple_uuid());
    dbg!(&token);
    match status::get_verify_code(&token) {
        Ok(n) => {
            let b = crate::image::image::gen_verify_image(n.as_slice());
            let mut r = Response::new(b.into());
            let mut header = HeaderMap::with_capacity(2);
            header.insert(header::CONTENT_TYPE, HeaderValue::from_str("image/png").unwrap());
            header.insert(
                header::SET_COOKIE,
                HeaderValue::from_str(&session_id_cookie(&token)).unwrap(),
            );
            // header.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_str("*").unwrap());
            // header.insert(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, HeaderValue::from_str("true").unwrap());
            let headers = r.headers_mut();
            headers.extend(header);
            Ok(r)
        },
        Err(e) => return Ok(Response::new("Wrong request token".into())),
    }
}

pub async fn get_upload_image(tail: Tail) -> Result<impl Reply, Rejection> {
    let tail_str = tail.as_str();
    service::image::get_upload_image(tail_str)
        .await
        .map(|d| {
            let content_length = d.len();
            // 这里指定返回值，否则Rustc推到不出来类型
            let mut r: Response<hyper::Body> = Response::new(d.into());
            let mut header = HeaderMap::with_capacity(2);
            let image_mime = if tail_str.ends_with("png") {
                "image/png"
            } else {
                "image/jpg"
            };
            header.insert(header::CONTENT_TYPE, HeaderValue::from_str(image_mime).unwrap());
            header.insert(header::CONTENT_LENGTH, HeaderValue::from(content_length));
            let headers = r.headers_mut();
            headers.extend(header);
            r
        })
        .or_else(|e| {
            let message = format!("{}", e.0);
            Ok(Response::new(message.into()))
        })
}

pub async fn upload(post_id: u64, user: Option<UserInfo>, data: FormData) -> Result<impl Reply, Rejection> {
    if user.is_none() {
        return Ok(wrap_json_err(500, Error::NotAuthed));
    }
    let upload_image = service::image::upload(post_id, data).await;
    upload_image
        .map(|d| wrap_json_data(&d))
        .or_else(|e| Ok(wrap_json_err(500, e.0)))
}

pub async fn upload_title_image(post_id: u64, user: Option<UserInfo>, data: FormData) -> Result<impl Reply, Rejection> {
    if user.is_none() {
        return Ok(wrap_json_err(500, Error::NotAuthed));
    }
    let result = service::image::upload(post_id, data).await;
    if let Err(e) = result {
        return Ok(wrap_json_err(500, e.0));
    }
    let images = result.unwrap();
    let image = &images[0];
    post::update_title_image(post_id as i64, &image.relative_path)
        .await
        .map(|d| wrap_json_data(image))
        .or_else(|e| Ok(wrap_json_err(500, e.0)))
}

pub async fn save(
    post_id: u64,
    filename: String,
    user: Option<UserInfo>,
    body: impl Buf,
) -> Result<impl Reply, Rejection> {
    if user.is_none() {
        return Ok(wrap_json_err(500, Error::NotAuthed));
    }
    let upload_image = service::image::save(post_id, filename, body).await;
    upload_image
        .map(|d| wrap_json_data(&d))
        .or_else(|e| Ok(wrap_json_err(500, e.0)))
}

// pub async fn resize_blog_image<B: AsRef<&[u8]>, T: AsRef<&str>>(b: B, type: T) {}

pub async fn random_title_image(post_id: u64) -> Result<impl Reply, Rejection> {
    crate::service::image::random_title_image(post_id)
        .await
        .map(|f| wrap_json_data(&f))
        .or_else(|e| Ok(wrap_json_err(500, e.0)))
}

use std::path::PathBuf;

use blog_common::dto::FormDataItem;
use blog_common::{dto::post::UploadImage, result::Error};
use bytes::Buf;
use rand::Rng;
use warp::filters::multipart::{FormData};

use crate::{
    image::image,
    util::{
        io::{self, SupportFileType},
        result::Result,
    },
};

pub async fn get_upload_image(path: &str) -> Result<Vec<u8>> {
    let mut path_buf = PathBuf::with_capacity(32);
    path_buf.push("upload");
    let v: Vec<&str> = path.split_terminator('/').collect();
    for n in v {
        path_buf.push(n);
    }
    match tokio::fs::read(path_buf.as_path()).await {
        Ok(d) => Ok(d),
        Err(e) => {
            eprintln!("{} {:?}", path, e);
            Err(Error::FileNotFound.into())
        },
    }
}

pub async fn upload(post_id: u64, data: FormData) -> Result<Vec<UploadImage>> {
    let items = io::save_upload_file(
        post_id,
        data,
        &[SupportFileType::Png, SupportFileType::Jpg, SupportFileType::Gif],
    )
    .await?;
    let mut images: Vec<UploadImage> = Vec::with_capacity(items.len());
    for i in items.iter() {
        match i {
            FormDataItem::FILE(f) => {
                image::resize_from_file(&f).await?;
                let relative_path = f.relative_path.to_string();
                let original_filename = f.original_filename.to_string();
                images.push(UploadImage::new(relative_path, original_filename));
            },
            _ => {},
        }
    }
    Ok(images)
}

pub async fn save(post_id: u64, filename: String, body: impl Buf) -> Result<UploadImage> {
    let filename = urlencoding::decode(&filename)?;
    let filename = filename.into_owned();

    let file_info = io::save_upload_stream(
        post_id,
        filename,
        body,
        &[SupportFileType::Png, SupportFileType::Jpg, SupportFileType::Gif],
    )
    .await?;
    image::resize_from_file(&file_info).await?;
    let d = UploadImage::new(file_info.relative_path, file_info.original_filename);
    Ok(d)
}

// pub async fn resize_blog_image<B: AsRef<&[u8]>, T: AsRef<&str>>(b: B, type: T) {}

// https://rust-lang-nursery.github.io/rust-cookbook/web/clients/download.html
pub async fn random_title_image(id: u64) -> Result<String> {
    let url = {
        let mut rng = rand::thread_rng();
        if rng.gen_range(1..=100) > 75 {
            // https://source.unsplash.com/random/1000x500?keywords.join(",")&sig=cache_buster
            "https://source.unsplash.com/random/1000x500"
        } else {
            "https://picsum.photos/1000/500"
        }
    };
    let response = reqwest::get(url).await?;
    let file_ext = response
        .url()
        .path_segments()
        .and_then(|segments| segments.last())
        .and_then(|name| {
            if name.is_empty() {
                None
            } else {
                name.find(".").map(|pos| &name[pos + 1..])
            }
        })
        .unwrap_or(match response.headers().get("Content-Type") {
            Some(h) => {
                let r = h.to_str();
                match r {
                    Ok(header) => match header {
                        "image/jpeg" => "jpg",
                        "image/png" => "png",
                        _ => "",
                    },
                    Err(e) => {
                        eprintln!("{:?}", e);
                        ""
                    },
                }
            },
            None => "",
        });
    if file_ext.is_empty() {
        return Err(Error::UploadFailed.into());
    }
    let filename = format!("{}.{}", id, file_ext);
    // let mut file = tokio::fs::File::create(Path::new(&filename)).await?;
    let (mut file, _path_buf, relative_path) = io::get_save_file(id, &filename, file_ext, false).await?;
    let b = response.bytes().await?;
    tokio::io::copy_buf(&mut &b[..], &mut file).await?;
    // file.shutdown()
    Ok(relative_path)
}

pub async fn delete_post_images(post_id: u64) -> Result<()> {
    let (path, _) = io::get_save_path(post_id, "", "", false).await?;
    // let dir = path.parent().unwrap();
    let dir = std::env::current_dir()?.join(path);
    println!("dir={:?}", dir);
    let mut files = tokio::fs::read_dir(dir).await?;
    let post_id = post_id.to_string();
    while let Some(entry) = files.next_entry().await? {
        if entry.file_name().into_string().unwrap().starts_with(&post_id) {
            println!("Deleting {:?}", entry.file_name());
            tokio::fs::remove_file(entry.path()).await?;
        }
    }
    Ok(())
}

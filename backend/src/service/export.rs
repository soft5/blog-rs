use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use lazy_static::lazy_static;
use tera::Tera;
use zip::write::FileOptions;

use crate::db::model::Post;
use crate::db::post;
use crate::util::{self, result::Result};

static HUGO_TEMPLATE: &'static str = include_str!("../resource/static-site/template/hugo.txt");

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = Tera::default();
        if let Err(e) = tera.add_raw_template("hugo.md", HUGO_TEMPLATE) {
            eprintln!("{:?}", e);
        }
        tera
    };
}

fn render(post: &Post, template: &str) -> String {
    let mut context = tera::Context::new();
    context.insert("title", &post.title);
    context.insert("content", &post.markdown_content);
    match TEMPLATES.render(template, &context) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{:?}", e);
            String::new()
        },
    }
}

// fn write_posts(posts: &Vec<Post>, mut writer: impl std::io::Write) -> Result<()> {
// fn write_posts(posts: &Vec<Post>, callback: impl Fn(dyn std::io::Write, &str, &String) -> Result<()>) -> Result<()> {
fn write_posts<C>(posts: &Vec<Post>, mut callback: C) -> Result<()>
where
    C: FnMut(&String, &Post) -> Result<()>,
{
    let mut file_name = String::with_capacity(32);
    for post in posts {
        file_name.push_str(post.id.to_string().as_str());
        file_name.push_str(".md");

        callback(&file_name, &post);

        file_name.clear();
    }
    Ok(())
}

// fn write_to_zip(mut writer: zip::ZipWriter<File>, filename: &str, content: &String) -> Result<()> {
//     writer.start_file(filename, FileOptions::default())?;
//     writer.write_all(content.as_bytes())?;
//     Ok(())
// }

pub async fn hugo() -> Result<String> {
    let posts = post::all().await?;

    let export_dir = std::env::current_dir()?.join("export");
    if !export_dir.exists() {
        tokio::fs::create_dir(export_dir.as_path()).await?;
    }
    let mut filename = util::common::simple_uuid();
    filename.push_str(".zip");
    let output_file = export_dir.join(filename.as_str());
    let file = std::fs::File::create(output_file)?;
    let mut zip = zip::ZipWriter::new(file);

    let mut zip_file = |file_name: &String, post: &Post| -> Result<()> {
        zip.start_file(file_name, FileOptions::default())?;
        let content = render(post, "hugo.md");
        zip.write_all(content.as_bytes())?;
        Ok(())
    };

    write_posts(&posts, zip_file);

    /*
    let mut file_name = String::with_capacity(32);
    for post in posts.iter() {
        file_name.push_str(post.id.to_string().as_str());
        file_name.push_str(".md");

        zip_file(&file_name, post)?;

        // zip.start_file(file_name.as_str(), FileOptions::default())?;

        // let content = render(post, "hugo.md");
        // zip.write_all(content.as_bytes())?;

        file_name.clear();
    }
    */
    zip.finish()?;

    Ok(filename)
}

pub async fn git(mut root_path: PathBuf, last_export_timestamp: i64) -> Result<()> {
    let posts = post::all_by_since(last_export_timestamp).await?;
    let mut write_file = |filename: &String, post: &Post| -> Result<()> {
        root_path.set_file_name(filename);
        let mut file = OpenOptions::new().write(true).truncate(true).open(root_path.as_path())?;
        let content = render(post, "hugo.md");
        file.write_all(content.as_bytes())?;
        Ok(())
    };
    write_posts(&posts, write_file);
    Ok(())
}
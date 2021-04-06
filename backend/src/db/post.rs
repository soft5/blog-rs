use core::time::Duration;
use std::time::SystemTime;

use blog_common::{
    dto::{
        post::{NewPost, PostDetail},
        PaginationData,
    },
    result::Error,
};
use chrono::Timelike;
use comrak::{markdown_to_html, ComrakOptions};
use sqlx::{Row, Sqlite};

use crate::{
    db::{
        self,
        model::{Post, Tag},
        tag,
        tag::get_names,
        SqlParam, DATA_SOURCE,
    },
    util::{common, result::Result, snowflake},
};

fn review_rendered_content(c: &str) -> String {
    let r = common::HTML_TAG_REGEX.replace(c, "");
    let r = r.replace(r"\n", "").to_string();
    if r.len() < 200 {
        return r;
    }
    return r[..200].to_string();
}

async fn to_detail_list(posts: Vec<Post>) -> Result<Vec<PostDetail>> {
    let post_ids: Vec<i64> = posts.iter().map(|p| p.id).collect();
    let tags_map = tag::get_tags_by_post_ids(post_ids).await?;
    let post_detail_list = posts
        .iter()
        .map(|i| {
            let mut detail: PostDetail = i.into();
            detail.content = review_rendered_content(&i.rendered_content);
            let tags = tags_map.get(&i.id);
            if tags.is_some() {
                detail.tags = Some(tags.unwrap().iter().map(|t| t.name.clone()).collect());
            }
            detail
        })
        .collect::<Vec<_>>();
    // for post in post_detail_list.iter_mut() {
    //     let tags = tags_map.get(&post.id);
    // if tags.is_none() {
    //     continue;
    // }
    //     *post.tags = Some(tags.unwrap().iter().map(|t| t.name.clone()).collect());
    // }
    Ok(post_detail_list)
}

pub async fn list(page_num: u8, page_size: u8) -> Result<PaginationData<Vec<PostDetail>>> {
    let row = sqlx::query("SELECT COUNT(id) FROM post")
        .fetch_one(&DATA_SOURCE.get().unwrap().sqlite)
        .await?;
    let total: i64 = row.get(0);
    // println!("total={}", total);
    if total < 0 {
        return Ok(PaginationData { total: 0, data: vec![] });
    }

    let mut offset: i64 = ((page_num - 1) * page_size) as i64;
    if offset > total {
        offset = total - page_size as i64;
    }
    let d = sqlx::query_as::<Sqlite, Post>(
        "SELECT id,title,'' AS markdown_content,rendered_content,created_at,updated_at FROM post ORDER BY id DESC LIMIT ?, ?",
    )
        .bind(offset as i64)
        .bind(page_size)
        .fetch_all(&DATA_SOURCE.get().unwrap().sqlite)
        .await?;
    Ok(PaginationData {
        total: total as u64,
        data: to_detail_list(d).await?,
    })
    /*
    let mut p: Vec<crate::db::SqlParam> = Vec::new();
    p.push(SqlParam::I64(offset));
    p.push(SqlParam::I8(page_size as i8));
    let id_array =
        db::sqlite_get_list::<crate::db::Id>("SELECT id FROM post ORDER BY id DESC LIMIT ?,?", Some(p)).await?;
    let id_array: Vec<i64> = id_array.iter().map(|d| d.id).collect();
    let d = db::sled_get_list::<PostDetail>(&DATA_SOURCE.get().unwrap().setting, &id_array).await?;
    */
}

pub async fn list_by_tag(tag_name: String, page_num: u8, page_size: u8) -> Result<PaginationData<Vec<PostDetail>>> {
    let tag_name = urlencoding::decode(&tag_name)?;
    let tag = sqlx::query_as::<Sqlite, Tag>("SELECT id,name FROM tag WHERE name = ?")
        .bind(&tag_name)
        .fetch_optional(&DATA_SOURCE.get().unwrap().sqlite)
        .await?;
    if tag.is_none() {
        return Err(Error::TagNotFound.into());
    }
    let tag = tag.unwrap();

    let r = sqlx::query("SELECT COUNT(*) FROM tag_usage WHERE tag_id = ?")
        .bind(tag.id)
        .fetch_one(&DATA_SOURCE.get().unwrap().sqlite)
        .await?;
    let r = r.try_get::<i64, usize>(0);
    if let Err(e) = r {
        eprintln!("{:?}", e);
        return Err(Error::SqliteDbError.into());
    }

    let total = r.unwrap();
    if total < 1 {
        return Ok(PaginationData { total: 0, data: vec![] });
    }

    let mut offset: i64 = ((page_num - 1) * page_size) as i64;
    if offset > total {
        offset = total - page_size as i64;
    }
    let d = sqlx::query_as::<Sqlite, Post>(
        "SELECT id,title,'' AS markdown_content,rendered_content,created_at,updated_at FROM post WHERE id IN (SELECT post_id FROM tag_usage WHERE tag_id = ? ORDER BY id DESC LIMIT ?, ?)",
    )
    .bind(tag.id)
    .bind(offset as i64)
    .bind(page_size)
    .fetch_all(&DATA_SOURCE.get().unwrap().sqlite)
    .await?;
    Ok(PaginationData {
        total: total as u64,
        data: to_detail_list(d).await?,
    })
}

pub async fn save(new_post: NewPost) -> Result<PostDetail> {
    // needs to be in a transaction
    let transaction = DATA_SOURCE.get().unwrap().sqlite.begin().await?;

    let id = snowflake::gen_id();
    // println!("id {}", id);
    if new_post.tags.is_some() {
        super::tag::record_usage(id, new_post.tags.as_ref().unwrap()).await?;
    }

    // let parser = pulldown_cmark::Parser::new(body);
    // let mut html_text = String::new();
    // pulldown_cmark::html::push_html(&mut html_text, parser);

    let post_detail = PostDetail {
        id: id as i64,
        title: new_post.title,
        content: markdown_to_html(&new_post.content, &ComrakOptions::default()),
        tags: new_post.tags,
        created_at: chrono::offset::Utc::now(),
        updated_at: None,
    };

    // save to sqlite
    let last_insert_rowid =
        sqlx::query("INSERT INTO post(id, title, markdown_content, rendered_content, created_at)VALUES(?,?,?,?,?)")
            .bind(&post_detail.id)
            .bind(&post_detail.title)
            .bind(&new_post.content)
            .bind(&post_detail.content)
            .bind(post_detail.created_at.second() as i64)
            .execute(&DATA_SOURCE.get().unwrap().sqlite)
            .await?
            .last_insert_rowid();

    if last_insert_rowid < 1 {
        // println!("last_insert_rowid {}", last_insert_rowid);
        return Err(Error::SaveBlogFailed.into());
    }

    // 这里只关心 commit，因为 https://docs.rs/sqlx/0.5.1/sqlx/struct.Transaction.html 说到
    // If neither are called before the transaction goes out-of-scope, rollback is called. In other words, rollback is called on drop if the transaction is still in-progress.
    transaction.commit().await?;

    Ok(post_detail)
}

pub async fn show(id: u64) -> Result<PostDetail> {
    // let r: Option<PostDetail> = db::sled_get(&DATA_SOURCE.get().unwrap().blog, id.to_le_bytes()).await?;
    let id = id as i64;
    let r = sqlx::query_as::<Sqlite, Post>(
        "SELECT id,title,'' AS markdown_content,rendered_content,created_at,updated_at FROM post WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&DATA_SOURCE.get().unwrap().sqlite)
    .await?;
    if r.is_none() {
        Err(Error::CannotFoundBlog.into())
    } else {
        let tags = sqlx::query_as::<Sqlite, Tag>("SELECT t.id AS id, t.name AS name FROM tag t INNER JOIN tag_usage u ON t.id = u.tag_id WHERE u.post_id = ? ORDER BY t.created_at DESC")
            .bind(id)
            .fetch_all(&DATA_SOURCE.get().unwrap().sqlite)
            .await?.iter().map(|t| t.name.clone()).collect();
        let mut post_detail: PostDetail = (&r.unwrap()).into();
        post_detail.tags = Some(tags);
        Ok(post_detail)
    }
}

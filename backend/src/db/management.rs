use blog_common::util::time;
use blog_common::dto::user::UserInfo;
use sqlx::{Row, Sqlite};

use crate::{
    db::{
        self,
        model::{Setting},
    },
    service::status,
    util::{crypt, result::Result},
};

pub async fn has_admin_password() -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(id) FROM settings WHERE item='admin_password'")
        .fetch_one(db::get_sqlite())
        .await?;
    let total: i64 = row.get(0);
    dbg!(total);
    return Ok(total > 0);
}

pub async fn admin_login(token: &str, password: &str) -> Result<bool> {
    let d = get_setting("admin_password").await?;

    if let Some(settings) = d {
        if crypt::verify_password(password, &settings.content)? {
            status::user_online(token, UserInfo { id: 1 });
            return Ok(true);
        }
    }
    return Ok(false);
}

pub async fn update_setting(setting: Setting) -> Result<()> {
    // db::sled_save(&DATA_SOURCE.get().unwrap().management, "settings", &setting).await?;
    let content = if setting.item.eq("admin_password") {
        if setting.content.is_empty() {
            String::new()
        } else {
            crypt::encrypt_password(&setting.content)?
        }
    } else {
        setting.content
    };

    let now = time::unix_epoch_sec() as i64;

    let r = sqlx::query("UPDATE settings SET content=?,updated_at=? WHERE item=?")
        .bind(&content)
        .bind(now)
        .bind(&setting.item)
        .execute(db::get_sqlite())
        .await?;

    if r.rows_affected() < 1 {
        sqlx::query("INSERT INTO settings(item,content,created_at,updated_at)VALUES(?,?,?,?)")
            .bind(&setting.item)
            .bind(&content)
            .bind(now)
            .bind(now)
            .execute(db::get_sqlite())
            .await?;
    }
    Ok(())
}

pub async fn get_setting(item: &str) -> Result<Option<Setting>> {
    let r = sqlx::query_as::<Sqlite, Setting>("SELECT * FROM settings WHERE item=?")
        .bind(item)
        .fetch_optional(super::get_sqlite())
        .await?;
    Ok(r)
}

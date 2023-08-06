use diesel::prelude::*;
use diesel::update;
use crate::db::get_connection;
use crate::schema::config::dsl::config;
use crate::schema::config::now_playing_body_realized;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::config)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Config {
    pub current_song_position: i64,
    pub current_song_id: Option<i32>,
    pub window_width: i32,
    pub window_height: i32,
    pub maximized: i32,
    pub now_playing_body_realized: i32,
}

pub fn update_now_playing_body_realized(realized: bool) {
    update(config).set(now_playing_body_realized.eq(if realized { 1 } else { 0 })).execute(&mut get_connection())
        .unwrap();
}

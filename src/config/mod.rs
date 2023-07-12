use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::config)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Config {
    pub volume: f32,
    pub current_song_id: Option<i32>,
    pub window_width: i32,
    pub window_height: i32,
    pub maximized: i32,
}

use diesel::{Queryable, Selectable};

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::collections)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(in crate::collection) struct Collection {
    pub id: i32,
    pub path: String,
    pub row: i32,
    pub modified: Option<i64>,
}

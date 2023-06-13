use std::env::var;
use std::ops::Deref;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::SqliteConnection;
use once_cell::sync::Lazy;

const DATABASE_URL: &'static str = "DATABASE_URL";
static CONNECTION: Lazy<Pool<ConnectionManager<SqliteConnection>>> = Lazy::new(|| {
    let database_url = var(DATABASE_URL).expect(format!("{} must be set", DATABASE_URL).as_str());
    Pool::builder().test_on_check_out(true).build(ConnectionManager::<SqliteConnection>::new(database_url))
        .expect("Could not build connection pool")
});

pub fn get_connection() -> PooledConnection<ConnectionManager<SqliteConnection>> {
    CONNECTION.deref().get().expect("should be able to get connection from pool")
}

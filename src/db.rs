use std::ops::Deref;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::SqliteConnection;
use once_cell::sync::Lazy;

static CONNECTION: Lazy<Pool<ConnectionManager<SqliteConnection>>> = Lazy::new(|| {
    Pool::builder().test_on_check_out(true).build(ConnectionManager::<SqliteConnection>::new("music-player.sqlite"))
        .expect("Could not build connection pool")
});

pub fn get_connection() -> PooledConnection<ConnectionManager<SqliteConnection>> {
    CONNECTION.deref().get().expect("should be able to get connection from pool")
}

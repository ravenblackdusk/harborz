use std::ops::Deref;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::SqliteConnection;
use once_cell::sync::Lazy;
use diesel_migrations::{embed_migrations, EmbeddedMigrations};

static CONNECTION: Lazy<Pool<ConnectionManager<SqliteConnection>>> = Lazy::new(|| {
    Pool::builder().test_on_check_out(true).build(ConnectionManager::<SqliteConnection>::new("music-player.sqlite"))
        .expect("Could not build connection pool")
});

pub fn get_connection() -> PooledConnection<ConnectionManager<SqliteConnection>> {
    CONNECTION.deref().get().expect("should be able to get connection from pool")
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
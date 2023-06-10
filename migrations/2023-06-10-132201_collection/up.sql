create table collections
(
    id   integer not null
        constraint collections_pk
            primary key autoincrement,
    path TEXT    not null
);

create table collections
(
    id   integer not null
        constraint collections_pk
            primary key autoincrement,
    path TEXT    not null
);

create unique index collections_path_uindex
    on collections (path);

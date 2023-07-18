create table history_bodies
(
    id        integer not null
        constraint history_bodies_pk
            primary key autoincrement,
    query     TEXT,
    body_type TEXT    not null
);

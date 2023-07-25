create table bodies
(
    id                integer not null
        constraint bodies_pk
            primary key autoincrement,
    query             TEXT,
    body_type         TEXT    not null,
    scroll_adjustment REAL,
    navigation_type   TEXT    not null
);

drop table history_bodies;

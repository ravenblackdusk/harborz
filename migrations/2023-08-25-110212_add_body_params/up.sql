create table bodies_dg_tmp
(
    id                INTEGER not null
        constraint bodies_pk
            primary key autoincrement,
    body_type         TEXT    not null,
    scroll_adjustment REAL,
    navigation_type   TEXT    not null,
    params            TEXT    not null
);

drop table bodies;

alter table bodies_dg_tmp
    rename to bodies;

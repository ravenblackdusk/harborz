create table bodies_dg_tmp
(
    id                integer not null
        constraint bodies_pk
            primary key autoincrement,
    query1            TEXT,
    body_type         TEXT    not null,
    scroll_adjustment REAL,
    navigation_type   TEXT    not null,
    query2            TEXT
);

insert into bodies_dg_tmp(id, query1, body_type, scroll_adjustment, navigation_type)
select id, query, body_type, scroll_adjustment, navigation_type
from bodies;

drop table bodies;

alter table bodies_dg_tmp
    rename to bodies;

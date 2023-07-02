-- noinspection SqlWithoutWhere
delete
from songs;
-- noinspection SqlWithoutWhere
delete
from collections;

-- noinspection SqlAddNotNullColumn
alter table songs
    add duration sqlite_uint64 not null;

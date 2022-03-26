## Requiements
Nothing but Rust nightly toolchain

## Part-1 Local

`make hhdbms`

## Part-2 Http Server

`make hhserv`

## TODO

最关键的问题是不知道处理B站处理CRC32的Hash冲突解决的算法,

目前情况是计算得 1-12亿范围的uid的Hash碰撞有2亿条(不包括第一条).

尝试了简单再次CRC32`(CRC32(CRC32(x).to_string().encode()))`的冲突解决策略,没有成功

测试数据, 还原 `c4ff7ac1` 的uid

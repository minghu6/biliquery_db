# biliquery_db
Bilibili comment hash to user id rainbow table system


# Setup data
```
pip install -r requirements.txt
./helper init <data-id>
```

# Select database
```
./helper use <data-id>
```

# Requirements
libev

## Arch
`pacman -S libev`

## Ubuntu

`apt install libev-dev`

# Run server
```
make
./biliquery
```
程序本体大约只要 1MB 的内存，在 SSD 上每秒大概可以承受几十万请求，当然也可以改成纯内存的能上千万，不过没啥意义。

冲突表在内存里，但是由于 HASH 冲突非常多，冲突表甚至占了整整 4G 内存，待优化。

# API ( no sla, 300billon )

~~http://biliquery.typcn.com/api/user/hash/~~

http://localhost:6067/[用户Hash]

[用户Hash] 直接copy自弹幕的xml文件


# LBNL
把原来的实现重新润色了一下,并补充了一个helper.py的数据管理的辅助工具, 跑了几圈试试发现几个问题:
1. 在PHP里用echo来准备数据我不是很能认可(IO太慢了)
1. C++主程序第一次加载数据的时候有一个Free Invalid Pointer的未找到原因的问题,
但我不打算在上面花时间.

建议切换到rust-impl分支上,有一个rust的重新开始的实现, 伸缩性更好, 生成数据的效率更高
